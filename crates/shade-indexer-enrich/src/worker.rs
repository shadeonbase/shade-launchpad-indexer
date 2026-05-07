use async_trait::async_trait;
use shade_indexer_core::NormalizedDeploy;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{instrument, warn};

use crate::metrics::{Enrichment, HolderSnapshot};
use crate::store::EnrichStore;

/// Pluggable enrichment data fetcher. Implementations talk to RPC, Uniswap
/// subgraphs, lock-contract verifiers, etc. Stubbed in tests.
#[async_trait]
pub trait EnrichmentSource: Send + Sync + 'static {
    async fn fetch(&self, deploy: &NormalizedDeploy) -> anyhow::Result<HolderSnapshot>;
}

pub struct EnrichWorker {
    store: EnrichStore,
    source: Arc<dyn EnrichmentSource>,
}

impl EnrichWorker {
    pub fn new(store: EnrichStore, source: Arc<dyn EnrichmentSource>) -> Self {
        Self { store, source }
    }

    /// Drive the worker from a channel of deploys. Continues until the upstream
    /// channel closes.
    #[instrument(skip(self, rx))]
    pub async fn run(self, mut rx: mpsc::Receiver<NormalizedDeploy>) {
        while let Some(deploy) = rx.recv().await {
            if let Err(e) = self.process(&deploy).await {
                warn!(
                    error = ?e,
                    token = %format!("{:#x}", deploy.token),
                    "enrichment failed; continuing",
                );
                metrics::counter!("shade_indexer_enrich_failed").increment(1);
            }
        }
    }

    pub async fn process(&self, deploy: &NormalizedDeploy) -> anyhow::Result<()> {
        let id = self.store.upsert_deploy(deploy).await?;
        let snapshot = self.source.fetch(deploy).await?;
        let enrichment = Enrichment::from_snapshot(&snapshot);
        self.store.write_enrichment(id, &enrichment).await?;
        metrics::counter!(
            "shade_indexer_enriched",
            "launchpad" => deploy.launchpad.as_str(),
        )
        .increment(1);
        Ok(())
    }
}
