//! # presutaoru (ぷれすたおる)
//!
//! A linux Pressure Stall Information (PSI) file descriptor wrapper / monitor library for Rust.
//!
//! ```no_run
//! # use std::{time::Duration, path::Path};
//! # use presutaoru::*;
//!
//! let psi_fd = PsiFdBuilder::default()
//!     .entry(PsiEntry::Global(GlobalEntryType::Cpu))
//!     .stall_type(StallType::Some)
//!     .stall_amount(Duration::from_micros(500))
//!     .time_window(Duration::from_secs(1))
//!     .build()
//!     .unwrap();
//!
//! // Example for cgroup-based PSI
//! let cgroup_psi_fd = PsiFdBuilder::default()
//!     .entry(PsiEntry::Cgroup(
//!         CgroupEntryType::Cpu,
//!         Path::new("/sys/fs/cgroup/system.slice"),
//!     ))
//!     .stall_type(StallType::Full)
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
    entry::{CgroupEntryType, GlobalEntryType, PsiEntry},
    fd::{PsiFd, PsiFdBuilder, PsiFdBuilderError, StallType},
};
