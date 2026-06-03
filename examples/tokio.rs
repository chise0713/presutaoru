use std::time::Duration;

use presutaoru::*;
use tokio::io::{Interest, unix::AsyncFd};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let psi_fd = PsiFd::builder()
        .entry(PsiEntry::Global(GlobalEntryType::Cpu))
        .stall_amount(Duration::from_micros(1))
        .stall_type(StallType::Some)
        .time_window(Duration::from_secs(2))
        .build()
        .unwrap();

    let async_fd = AsyncFd::with_interest(psi_fd, Interest::PRIORITY).unwrap();

    while let Ok(mut guard) = async_fd.readable().await {
        eprintln!("event occurred: {guard:?}");
        guard.clear_ready();
    }
}
