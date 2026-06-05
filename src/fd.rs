use std::{
    fmt::Display,
    fs::OpenOptions,
    io::{self, Cursor, Write as _},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
        unix::fs::OpenOptionsExt as _,
    },
    path::PathBuf,
    time::Duration,
};

use crate::PsiEntry;

// Linux UAPI: include/uapi/asm-generic/fcntl.h
const O_NONBLOCK: i32 = 0o4000;

#[derive(Debug, Clone, Copy)]
pub enum StallType {
    Some,
    Full,
}

impl Display for StallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Some => "some",
            Self::Full => "full",
        })
    }
}

/// ```console
/// <some|full> <stall amount in us> <time window in us>
/// ```
///
/// <https://docs.kernel.org/accounting/psi.html>
#[derive(Debug)]
#[repr(transparent)]
pub struct PsiFd {
    fd: OwnedFd,
}

impl PsiFd {
    /// Returns a builder for constructing a [`PsiFd`].
    pub fn builder() -> PsiFdBuilder<'static> {
        PsiFdBuilder::new()
    }

    /// # Safety
    /// The provided file descriptor must refer to a PSI
    /// file with a successfully registered trigger.
    pub unsafe fn new_unchecked(fd: OwnedFd) -> Self {
        Self { fd }
    }
}

impl AsRawFd for PsiFd {
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

impl AsFd for PsiFd {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl From<PsiFd> for OwnedFd {
    fn from(value: PsiFd) -> Self {
        value.fd
    }
}

/// Builder for [`PsiFd`]
#[derive(Debug, Default, Clone, Copy)]
pub struct PsiFdBuilder<'a> {
    entry: Option<PsiEntry<'a>>,
    stall_type: Option<StallType>,
    stall_amount: Option<Duration>,
    time_window: Option<Duration>,
}

/// Errors that can occur when building a [`PsiFd`]
#[derive(thiserror::Error, Debug)]
pub enum PsiFdBuilderError {
    #[error("no entry specified")]
    NoEntry,
    #[error("no stall type specified")]
    NoStallType,
    #[error("no stall amount specified")]
    NoStallAmount,
    #[error("no time window specified")]
    NoTimeWindow,
    #[error("time window must be greater than or equal to 500 milliseconds")]
    TimeWindowTooSmall,
    #[error("time window must be less than or equal to 10 seconds")]
    TimeWindowTooLarge,
    #[error("stall amount must be greater than or equal to 1 microsecond")]
    StallAmountTooSmall,
    #[error("stall amount must not exceed the time window")]
    StallAmountExceedsTimeWindow,
    #[error("no psi entry found {0}")]
    NoPsiEntry(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl<'a> PsiFdBuilder<'a> {
    /// Creates a new [`PsiFdBuilder`].
    pub fn new() -> PsiFdBuilder<'static> {
        PsiFdBuilder::default()
    }

    /// Sets the [`PsiEntry`] to monitor.
    pub fn entry(mut self, entry: PsiEntry<'a>) -> Self {
        self.entry = Some(entry);
        self
    }

    /// Sets the [`StallType`].
    pub fn stall_type(mut self, stall_type: StallType) -> Self {
        self.stall_type = Some(stall_type);
        self
    }

    /// Sets the accumulated stall duration threshold.
    ///
    /// The value must not exceed the configured
    /// time window.
    pub fn stall_amount(mut self, amount: Duration) -> Self {
        self.stall_amount = Some(amount);
        self
    }

    /// Sets the PSI observation window.
    ///
    /// The kernel requires the window to be in the range:
    /// 500 milliseconds to 10 seconds (inclusive).
    pub fn time_window(mut self, window: Duration) -> Self {
        self.time_window = Some(window);
        self
    }

    /// Build the [`PsiFd`].
    ///
    /// This opens the underlying [`PsiEntry`] and registers
    /// the configured trigger with the kernel.
    pub fn build(self) -> Result<PsiFd, PsiFdBuilderError> {
        let entry = self.entry.ok_or(PsiFdBuilderError::NoEntry)?;
        let stall_type = self.stall_type.ok_or(PsiFdBuilderError::NoStallType)?;
        let stall_amount = self.stall_amount.ok_or(PsiFdBuilderError::NoStallAmount)?;
        let time_window = self.time_window.ok_or(PsiFdBuilderError::NoTimeWindow)?;
        if time_window < Duration::from_millis(500) {
            return Err(PsiFdBuilderError::TimeWindowTooSmall);
        }
        if time_window > Duration::from_secs(10) {
            return Err(PsiFdBuilderError::TimeWindowTooLarge);
        }
        if stall_amount < Duration::from_micros(1) {
            return Err(PsiFdBuilderError::StallAmountTooSmall);
        }
        if stall_amount > time_window {
            return Err(PsiFdBuilderError::StallAmountExceedsTimeWindow);
        }

        let path = entry.path();

        let mut file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(PsiFdBuilderError::NoPsiEntry(path.into_owned()));
            }
            Err(e) => {
                return Err(e)?;
            }
        };

        // "<full|some> 10000000 10000000\n" = 23 bytes
        const PSI_TRIGGER_BUF_SIZE: usize = 24;

        let mut buf = [0u8; PSI_TRIGGER_BUF_SIZE];

        let len = {
            let mut cursor = Cursor::new(&mut buf[..]);

            writeln!(
                cursor,
                "{} {} {}",
                stall_type,
                stall_amount.as_micros(),
                time_window.as_micros(),
            )?;

            cursor.position() as usize
        };

        file.write_all(&buf[..len])?;

        let fd = OwnedFd::from(file);

        // SAFETY:
        // The trigger has been validated and registered
        Ok(unsafe { PsiFd::new_unchecked(fd) })
    }
}
