use std::time::Duration;

use presutaoru::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum Id {
    Some1In2_000_000,
    Some2In2_000_000,
}

fn main() {
    let mut monitor = PsiMonitor::default();
    monitor.add_fd(
        Id::Some1In2_000_000,
        PsiFdBuilder::default()
            .entry(PsiEntry::Cpu)
            .stall_amount(Duration::from_micros(1))
            .stall_type(StallType::Some)
            .time_window(Duration::from_secs(2))
            .build()
            .unwrap(),
    );
    monitor.add_fd(
        Id::Some2In2_000_000,
        PsiFdBuilder::default()
            .entry(PsiEntry::Cpu)
            .stall_amount(Duration::from_micros(2))
            .stall_type(StallType::Some)
            .time_window(Duration::from_secs(2))
            .build()
            .unwrap(),
    );
    let mut thread = monitor.into_thread().unwrap();
    thread.start().unwrap();
    while let Ok(r) = thread.recv() {
        match r {
            Event::Ready(id) => println!("psi event triggerd on: {:?}", id),
            Event::Failure(e) => eprintln!("{}", e),
        }
    }
}
