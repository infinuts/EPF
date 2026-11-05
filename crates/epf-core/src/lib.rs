//! Shared engine for the EPF desktop app and CLI.
//! Discovery and refresh are read-only. Router mutations require confirmation.
pub mod diagnostics;
pub mod engine;
pub mod model;
pub mod router;
pub mod service;
pub mod store;

pub use engine::Engine;
pub use model::*;
pub use router::UpnpRouter;
pub use store::Store;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Storage(String),
    #[error("Confirm this operation before changing router rules.")]
    ConfirmationRequired,
    #[error("Another EPF process is using the configuration. Try again in a moment.")]
    Busy,
    #[error("{0}")]
    Partial(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
