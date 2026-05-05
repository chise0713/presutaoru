use std::{os::fd::AsFd as _, time::Duration};

use nix::sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout};
use presutaoru::*;

fn main() {
    let psi_fd = PsiFdBuilder::default()
        .entry(PsiEntry::Global(GlobalEntryType::Cpu))
        .stall_amount(Duration::from_micros(1))
        .stall_type(StallType::Some)
        .time_window(Duration::from_secs(2))
        .build()
        .unwrap();

    let epfd = Epoll::new(EpollCreateFlags::empty()).unwrap();

    // NOTE: Borrow the fd with `as_fd()` instead of moving it.
    // Moving the OwnedFd may cause it to be dropped (closed) too early,
    // leaving epoll with an invalid fd and resulting in no events.
    epfd.add(psi_fd.as_fd(), EpollEvent::new(EpollFlags::EPOLLPRI, 1))
        .unwrap();

    let events = &mut [EpollEvent::empty(); 1];

    while let Ok(num) = epfd.wait(events, EpollTimeout::NONE) {
        for ev in &events[..num] {
            eprintln!("event occurred: {ev:?}")
        }
    }
}
