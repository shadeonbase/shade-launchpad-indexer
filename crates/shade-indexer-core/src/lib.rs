//! Multi-launchpad event subscription and decoding.
//!
//! Public surface:
//! - [`types`] — normalized deploy schema.
//! - [`registry`] — factory metadata loader.
//! - [`decode`] — log → [`types::NormalizedDeploy`].
//! - [`subscriber`] — websocket subscription loop.

pub mod decode;
pub mod error;
pub mod registry;
pub mod subscriber;
pub mod types;

pub use error::IndexerError;
pub use registry::{FactoryRegistry, FactorySpec};
pub use subscriber::LaunchpadSubscriber;
pub use types::{Launchpad, NormalizedDeploy};
