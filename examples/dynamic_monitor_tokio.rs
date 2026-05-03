use presutaoru::*;
use std::time::Duration;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum Id {
    Initial,
    Dynamic,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut monitor = PsiMonitor::default();
    monitor.add_fd(
        Id::Initial,
        PsiFdBuilder::default()
            .entry(PsiEntry::Cpu)
            .stall_amount(Duration::from_micros(1))
            .stall_type(StallType::Some)
            .time_window(Duration::from_secs(2))
            .build()
            .unwrap(),
    );

    let reactor = monitor.into_tokio_reactor().unwrap();
    let mut reactor = reactor.start().unwrap();
    println!("Reactor started with initial FD.");

    println!("Waiting for events (Ctrl+C to stop)...");

    // Phase 1: Process events for the first 1 second
    let _ = tokio::time::timeout(Duration::from_secs(1), process_events(&mut reactor)).await;

    // Phase 2: Add dynamic FD
    println!("Dynamically adding new FD...");
    let dynamic_fd = PsiFdBuilder::default()
        .entry(PsiEntry::Memory)
        .stall_amount(Duration::from_micros(500))
        .stall_type(StallType::Some)
        .time_window(Duration::from_secs(2))
        .build()
        .unwrap();
    reactor.add_fd(Id::Dynamic, dynamic_fd).unwrap();

    // Phase 3: Process events for the next 2 seconds (reaching the 3s total mark)
    let _ = tokio::time::timeout(Duration::from_secs(2), process_events(&mut reactor)).await;

    // Phase 4: Remove the initial FD
    println!("Dynamically removing initial FD...");
    reactor.remove_fd(&Id::Initial);

    // Phase 5: Process events indefinitely
    process_events(&mut reactor).await;
}

/// Helper function to drain the reactor cleanly
async fn process_events(reactor: &mut PsiTokioReactorActive<Id>) {
    while let Ok(r) = reactor.recv().await {
        match r {
            Event::Ready(id) => println!("PSI event triggered on: {:?}", id),
            Event::Failure(e) => eprintln!("Error: {}", e),
        }
    }
}
