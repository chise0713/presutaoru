use std::{
    fmt::Display,
    fs::OpenOptions,
    io::{self, Write as _},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
        unix::fs::OpenOptionsExt as _,
    },
    path::PathBuf,
    time::Duration,
};

use libc::O_NONBLOCK;

use crate::PsiEntry;

#[derive(Clone, Copy)]
pub enum StallType {
    Some,
    Full,
}

impl Display for StallType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Some => write!(f, "some"),
            Self::Full => write!(f, "full"),
        }
    }
}

/// ```console
/// <some|full> <stall amount in us> <time window in us>
/// ```
///
/// <https://docs.kernel.org/accounting/psi.html>
#[derive(Debug)]
pub struct PsiFd {
    fd: OwnedFd,
    #[allow(dead_code)]
    pub(crate) from_builder: bool,
}

impl PsiFd {
    /// # Safety
    /// The provided `OwnedFd` must be a valid PSI file descriptor.
    pub unsafe fn new_unchecked(fd: OwnedFd) -> Self {
        Self {
            fd,
            from_builder: false,
        }
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

/// Builder for PsiFd
#[derive(Default, Clone)]
pub struct PsiFdBuilder {
    entry: Option<PsiEntry>,
    stall_type: Option<StallType>,
    stall_amount: Option<Duration>,
    time_window: Option<Duration>,
    cgroup: Option<PathBuf>,
}

/// Errors that can occur when building a PsiFd
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
    #[error("time window must be greater than 500 milliseconds")]
    TimeWindowTooSmall,
    #[error("stall amount must be less than time window")]
    StallAmountTooLarge,
    #[error("no psi entry found {0}")]
    NoPsiEntry(PsiEntry),
    #[error("cgroup psi entry not found at {0:?}")]
    NoCgroupPsiEntry(PathBuf),
    #[error("irq is not supported in cgroups")]
    IrqNotSupportedInCgroup,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl PsiFdBuilder {
    pub fn entry(mut self, entry: PsiEntry) -> Self {
        self.entry = Some(entry);
        self
    }

    pub fn stall_type(mut self, stall_type: StallType) -> Self {
        self.stall_type = Some(stall_type);
        self
    }

    pub fn stall_amount(mut self, amount: Duration) -> Self {
        self.stall_amount = Some(amount);
        self
    }

    pub fn time_window(mut self, window: Duration) -> Self {
        self.time_window = Some(window);
        self
    }

    /// Sets the cgroup path for the PSI file descriptor.
    ///
    /// If set, the builder will look for PSI files in `<path>/<type>.pressure`.
    /// Note that `Irq` is not supported in cgroups.
    pub fn cgroup(mut self, path: impl Into<PathBuf>) -> Self {
        self.cgroup = Some(path.into());
        self
    }

    /// Build the PsiFd, this will create and write the arguments to the underlying file descriptor
    pub fn build(self) -> Result<PsiFd, PsiFdBuilderError> {
        let entry = self.entry.ok_or(PsiFdBuilderError::NoEntry)?;
        let stall_type = self.stall_type.ok_or(PsiFdBuilderError::NoStallType)?;
        let stall_amount = self.stall_amount.ok_or(PsiFdBuilderError::NoStallAmount)?;
        let time_window = self.time_window.ok_or(PsiFdBuilderError::NoTimeWindow)?;
        if time_window < Duration::from_millis(500) {
            return Err(PsiFdBuilderError::TimeWindowTooSmall);
        }
        if stall_amount >= time_window {
            return Err(PsiFdBuilderError::StallAmountTooLarge);
        }

        let path = match self.cgroup {
            Some(ref cgroup_path) => {
                let file_name = entry
                    .cgroup_file_name()
                    .ok_or(PsiFdBuilderError::IrqNotSupportedInCgroup)?;
                cgroup_path.join(file_name)
            }
            None => entry.as_ref().to_path_buf(),
        };

        if !path.exists() {
            return Err(self
                .cgroup
                .as_ref()
                .map_or(PsiFdBuilderError::NoPsiEntry(entry), |_| {
                    PsiFdBuilderError::NoCgroupPsiEntry(path.clone())
                }));
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(path)?;
        file.write_all(
            format!(
                "{} {} {}\n",
                stall_type,
                stall_amount.as_micros(),
                time_window.as_micros()
            )
            .as_bytes(),
        )?;
        Ok(PsiFd {
            fd: file.into(),
            from_builder: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irq_not_supported_in_cgroup() {
        let builder = PsiFdBuilder::default()
            .entry(PsiEntry::Irq)
            .cgroup("/tmp")
            .stall_type(StallType::Some)
            .stall_amount(Duration::from_micros(1000))
            .time_window(Duration::from_millis(1000));

        match builder.build() {
            Err(PsiFdBuilderError::IrqNotSupportedInCgroup) => {}
            _ => panic!("Should have failed with IrqNotSupportedInCgroup"),
        }
    }
}
