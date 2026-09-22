//! Platform activation and transport. Game data is only accessed by the caller's game thread.
use trainer_bridge::{Bridge, GamePort};

#[cfg(windows)]
pub struct Endpoint(trainer_transport::Server);

#[cfg(windows)]
pub fn enabled() -> bool {
    std::env::var("CAVESTORY_TRAINER").as_deref() == Ok("1")
}

#[cfg(windows)]
impl Endpoint {
    pub fn open() -> std::io::Result<Self> {
        trainer_transport::Server::new(&trainer_transport::name_for_pid(std::process::id())).map(Self)
    }
    pub fn poll<G: GamePort>(&mut self, bridge: &mut Bridge<G>) -> std::io::Result<usize> {
        self.0.poll(bridge)
    }
}

#[cfg(target_os = "android")]
pub use crate::trainer_android::{enabled, Endpoint};

// Unsupported platforms retain a compile-only shared-core entry.
#[cfg(not(any(windows, target_os = "android")))]
pub struct Endpoint;

#[cfg(not(any(windows, target_os = "android")))]
pub fn enabled() -> bool {
    false
}

#[cfg(not(any(windows, target_os = "android")))]
impl Endpoint {
    pub fn open() -> std::io::Result<Self> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
    pub fn poll<G: GamePort>(&mut self, _bridge: &mut Bridge<G>) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::Unsupported.into())
    }
}
