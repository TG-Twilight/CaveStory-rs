//! Local IPC boundary. Implementations must obey integrations/transport-contract.md.
use trainer_protocol::{Request, Response};

/// One authenticated connection, bound to the instance returned at open time.
/// Dropping it closes the OS connection; an interrupted write is not retry-safe.
pub trait ClientConnection {
    fn exchange(&mut self, request: &Request) -> std::io::Result<Response>;
}

#[cfg(windows)]
impl ClientConnection for windows::Connection {
    fn exchange(&mut self, request: &Request) -> std::io::Result<Response> {
        windows::Connection::exchange(self, request)
    }
}
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
