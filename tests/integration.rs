use std::{path::Path, time::Duration};

use presutaoru::*;
use tokio::{
    io::{Interest, unix::AsyncFd},
    time,
};

#[tokio::test]
async fn integration() {
    let builder = PsiFd::builder()
        .stall_amount(Duration::from_micros(1))
        .stall_type(StallType::Some)
        .time_window(Duration::from_secs(2));

    let psi_fd_global = builder
        .entry(PsiEntry::Global(GlobalEntryType::Cpu))
        .build()
        .unwrap();

    let async_fd_global = AsyncFd::with_interest(psi_fd_global, Interest::PRIORITY).unwrap();

    let psi_fd_cgroup = builder
        .entry(PsiEntry::Cgroup(
            CgroupEntryType::Cpu,
            Path::new("/sys/fs/cgroup"),
        ))
        .build()
        .unwrap();

    let async_fd_cgroup = AsyncFd::with_interest(psi_fd_cgroup, Interest::PRIORITY).unwrap();

    let joined = async { tokio::join!(async_fd_global.readable(), async_fd_cgroup.readable()) };

    match time::timeout(Duration::from_secs(2), joined).await {
        Ok((Ok(mut guard_1), Ok(mut guard_2))) => {
            guard_1.clear_ready();
            guard_2.clear_ready();
        }
        other => panic!("{other:?}"),
    }
}
