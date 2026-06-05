//! # presutaoru (ぷれすたおる)
//!
//! A linux Pressure Stall Information (PSI) file descriptor wrapper library for Rust.
//!
//! ```no_run
//! # use std::{time::Duration, path::Path};
//! # use presutaoru::*;
//! let psi_fd = PsiFd::builder()
//!     .entry(PsiEntry::Global(GlobalEntryType::Cpu))
//!     .stall_type(StallType::Some)
//!     .stall_amount(Duration::from_micros(500))
//!     .time_window(Duration::from_secs(1))
//!     .build()
//!     .unwrap();
//!
//! // Example for cgroup-based PSI
//! let cgroup_psi_fd = PsiFd::builder()
//!     .entry(PsiEntry::Cgroup(
//!         CgroupEntryType::Cpu,
//!         Path::new("/sys/fs/cgroup/system.slice"),
//!     ))
//!     .stall_type(StallType::Full)
//!     .stall_amount(Duration::from_micros(500))
//!     .time_window(Duration::from_secs(1))
//!     .build()
//!     .unwrap();
//! ```
#[cfg(not(any(target_os = "linux", target_os = "android")))]
compile_error!("presutaoru only supports Linux and Android platforms.");

mod entry;
mod fd;

pub use crate::{
    entry::{CgroupEntryType, GlobalEntryType, PsiEntry},
    fd::{PsiFd, PsiFdBuilder, PsiFdBuilderError, StallType},
};
