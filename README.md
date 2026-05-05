# presutaoru (ぷれすたおる)

A linux Pressure Stall Information (PSI) file descriptor wrapper library for Rust.

This crate provides a thin wrapper around Linux PSI file descriptors.
It does not implement any monitoring abstraction — users are expected
to integrate with epoll or async runtimes manually.

## Integration

Typical approaches:

- Use `epoll` (via `libc` or crates like `nix`) and watch for `EPOLLPRI`
- Use async runtimes (e.g. `tokio::io::unix::AsyncFd`) with `Interest::PRIORITY`

A PSI file descriptor is a handle to a registered pressure trigger.

It does not carry data and is not readable in the conventional sense.
Instead, it becomes observable via `poll` / `epoll`, with `POLLPRI`
indicating that the PSI threshold has been exceeded.

## Example

Epoll: [examples/epoll.rs](./examples/epoll.rs)

Tokio: [examples/tokio.rs](./examples/tokio.rs)

## License
This project is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.