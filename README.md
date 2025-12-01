# presutaoru (ぷれすたおる)

A linux Pressure Stall Information (PSI) file descriptor monitor library for Rust.

## Example

```rust
use presutaoru::{PsiFdBuilder, PsiEntry, StallType, PsiMonitor};
use std::time::Duration;

let psi_fd = PsiFdBuilder::default()
    .entry(presutaoru::PsiEntry::Cpu)
    .stall_type(presutaoru::StallType::Some)
    .stall_amount(Duration::from_micros(500))
    .time_window(Duration::from_secs(1))
    .build()
    .unwrap();
```

## Addionally, to monitor PSI events, enable the `monitor` feature.

```rust
//! [dependencies]
//! presutaoru = { version = "0.1", features = ["monitor"] }
let mut monitor = PsiMonitor::new();
monitor.add_fd(psi_fd);
```

### Running the monitor

When using std::thread:

```rust
// same as above
//! [dependencies]
//! presutaoru = { version = "0.1", features = ["monitor", "thread"] }
let mut thread = monitor.into_thread().unwrap();
thread.start().unwrap();

while let Ok(r) = thread.recv() {
    match r {
        Event::Ready(id) => println!("psi event triggerd on: {:?}", id),
        Event::Failure(e) => eprintln!("{}", e.to_string()),
    }
}
```

Or register the file desc to tokio's reactor:

```rust
// same as above
//! [dependencies]
//! presutaoru = { version = "0.1", features = ["monitor", "tokio"] }
use presutaoru::tokio::PsiTokioReactor;

let mut job = monitor.into_tokio_reactor().unwrap();
job.start().unwrap();

while let Ok(r) = job.recv().await {
    match r {
        Event::Ready(id) => println!("psi event triggerd on: {:?}", id),
        Event::Failure(e) => eprintln!("{}", e.to_string()),
    }
}
```

All file descriptors will be closed when the monitor or its thread / job are dropped.
```rust
// ...
drop(monitor); // all fds are closed here
// ...
// in the thread example
drop(thread); // all fds are closed here
// ...
// in the tokio example
drop(jobs);   // all fds are closed here
```

## Features
- `monitor`: Enable PSI event monitoring functionality.
- `thread`: Enable monitoring using std::thread.
- `tokio`: Enable monitoring using tokio's reactor.

All features are disabled by default. 

`thread` and `tokio` features require `monitor` feature to be enabled.

## License
This project is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.