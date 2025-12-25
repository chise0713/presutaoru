//! # presutaoru (ぷれすたおる)
//!
//! A linux Pressure Stall Information (PSI) file descriptor wrapper / monitor library for Rust.
//!
//! ```no_run
//! use presutaoru::{PsiFdBuilder, PsiEntry, StallType, PsiMonitor};
//! use std::time::Duration;
//!
//! let psi_fd = PsiFdBuilder::default()
//!     .entry(presutaoru::PsiEntry::Cpu)
//!     .stall_type(presutaoru::StallType::Some)
//!     .stall_amount(Duration::from_micros(500))
//!     .time_window(Duration::from_secs(1))
//!     .build()
//!     .unwrap();
//!
//! let mut monitor = PsiMonitor::default();
//! monitor.add_fd("psi_fd", psi_fd);
//! let mut thread = monitor.into_thread().unwrap();
//!
//! while let Ok(event) = thread.recv() {
//!    println!("PSI Event: {:?}", event);
//! }
//! ```
#[cfg(not(any(target_os = "linux", target_os = "android")))]
compile_error!("presutaoru only supports Linux and Android platforms.");

mod entry;
mod fd;
#[cfg(feature = "monitor")]
mod monitor;
#[cfg(feature = "thread")]
mod thread;
#[cfg(feature = "tokio")]
mod tokio;

#[cfg(feature = "monitor")]
pub use crate::monitor::{Event, PsiMonitor};
#[cfg(feature = "thread")]
pub use crate::thread::PsiThread;
#[cfg(feature = "tokio")]
pub use crate::tokio::PsiTokioReactor;
pub use crate::{
    entry::PsiEntry,
    fd::{PsiFd, PsiFdBuilder, PsiFdBuilderError, StallType},
};
