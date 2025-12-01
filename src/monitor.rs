use std::{
    hash::Hash,
    io::{self, Error, Result},
};

use rustc_hash::FxHashMap;

use crate::PsiFd;
#[cfg(feature = "thread")]
use crate::thread::PsiThread;
#[cfg(feature = "tokio")]
use crate::tokio::PsiTokioReactor;

/// A monitor for multiple PSI file descriptors.
/// Which does not implement any polling mechanism itself, but allows
/// managing multiple PSI FDs conveniently.
pub struct PsiMonitor<T: Hash + Eq> {
    map: FxHashMap<T, PsiFd>,
}

impl<T> PsiMonitor<T>
where
    T: Hash + Eq,
{
    const PANIC_MSG_INVALID_FD: &str =
        r#"PsiFd added to PsiMonitor must be created using PsiFdBuilder"#;

    /// Add a `PsiFd` to the monitor
    pub fn add_fd(&mut self, id: T, fd: PsiFd) {
        if !fd.from_builder {
            panic!("{}", Self::PANIC_MSG_INVALID_FD);
        }
        _ = self.map.insert(id, fd);
    }

    /// Remove a `PsiFd` from the monitor
    pub fn remove_fd(&mut self, id: &T) -> Result<()> {
        self.map.remove(id).ok_or(io::Error::new(
            io::ErrorKind::NotFound,
            "PsiFd is not found",
        ))?;
        Ok(())
    }

    /// Clears the inner map, removing all `PsiFd`s.
    /// Keeps the allocated memory for reuse
    pub fn clear(&mut self) {
        self.map.clear()
    }

    /// Return the number of `PsiFd`s in the monitor
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Create a new empty monitor with the specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    /// Returns `true` if the monitor contains no `PsiFd`s.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Create an epoll-based monitoring thread
    #[cfg(feature = "thread")]
    pub fn into_thread(self) -> Result<PsiThread<T>>
    where
        T: Clone + Send + Sync,
    {
        PsiThread::new(self.map)
    }

    /// Embedding the monitor into tokio's reactor
    #[cfg(feature = "tokio")]
    pub fn into_tokio_reactor(self) -> Result<PsiTokioReactor<T>>
    where
        T: Clone + Send + Sync,
    {
        crate::tokio::PsiTokioReactor::new(self.map)
    }

    pub fn into_inner(self) -> FxHashMap<T, PsiFd> {
        self.map
    }
}

impl<T> Default for PsiMonitor<T>
where
    T: Hash + Eq,
{
    fn default() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }
}

#[derive(Debug)]
pub enum Event<T>
where
    T: Send,
{
    Ready(T),
    Failure(Error),
}
