//! Postgres-backed enrichment worker.
//!
//! Each [`shade_indexer_core::NormalizedDeploy`] is upserted into `deploys`
//! and an enrichment row is computed and stored in `deploy_enrichment`:
//!
//! - top-10 holder share
//! - Gini coefficient of holder distribution
//! - Herfindahl-Hirschman index (HHI)
//! - liquidity-to-FDV ratio (`ρ`)
//! - liquidity lock status
//! - bytecode flags (bitmask)

pub mod metrics;
pub mod rpc_source;
pub mod store;
pub mod worker;

pub use metrics::{
    Enrichment, HolderSnapshot, BYTECODE_FEE_ON_TRANSFER, BYTECODE_HONEYPOT, BYTECODE_MINT_BACKDOOR,
};
pub use rpc_source::{RpcEnrichmentSource, RpcSourceConfig, UNI_V3_FACTORY_BASE, WETH_BASE};
pub use store::EnrichStore;
pub use worker::{EnrichWorker, EnrichmentSource};
