//! Kafka producer for normalized launchpad deploy events.
//!
//! Publishes one message per [`shade_indexer_core::NormalizedDeploy`] to a
//! per-launchpad topic (`shade.launches.{launchpad}`), keyed by token address
//! so consumers can partition by token.

pub mod producer;
pub use producer::{LaunchpadProducer, ProducerConfig, ProducerError};
