use crate::decode::decode_deploy;
use crate::error::IndexerError;
use crate::registry::FactoryRegistry;
use crate::types::NormalizedDeploy;

use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::{Filter, Log};
use futures_util::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, instrument, warn};

const CHANNEL_BUF: usize = 4_096;
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_secs(30);

pub struct LaunchpadSubscriber {
    ws_url: String,
    registry: Arc<FactoryRegistry>,
    out: mpsc::Sender<NormalizedDeploy>,
}

impl LaunchpadSubscriber {
    pub fn new(
        ws_url: impl Into<String>,
        registry: FactoryRegistry,
    ) -> (Self, mpsc::Receiver<NormalizedDeploy>) {
        let (tx, rx) = mpsc::channel(CHANNEL_BUF);
        (
            Self {
                ws_url: ws_url.into(),
                registry: Arc::new(registry),
                out: tx,
            },
            rx,
        )
    }

    /// Run the subscription loop forever, reconnecting with exponential backoff
    /// on any websocket disruption. Returns only when the downstream receiver
    /// is dropped.
    #[instrument(skip(self), fields(ws = %self.ws_url, factories = self.registry.len()))]
    pub async fn run(self) -> Result<(), IndexerError> {
        let mut backoff = Duration::from_millis(250);

        loop {
            match self.run_once().await {
                Ok(()) => {
                    info!("subscriber stream ended cleanly");
                    return Ok(());
                }
                Err(IndexerError::Subscription(msg)) => {
                    warn!(error = %msg, ?backoff, "subscription drop; reconnecting");
                    sleep(backoff).await;
                    backoff = (backoff * 2).min(RECONNECT_BACKOFF_MAX);
                }
                Err(e) => {
                    warn!(error = ?e, "subscriber fatal");
                    return Err(e);
                }
            }
        }
    }

    async fn run_once(&self) -> Result<(), IndexerError> {
        let provider = ProviderBuilder::new()
            .on_ws(WsConnect::new(self.ws_url.clone()))
            .await
            .map_err(|e| IndexerError::Subscription(format!("ws connect: {e}")))?;

        let filter = Filter::new()
            .address(self.registry.all_addresses())
            .event_signature(self.registry.all_topics());

        let sub = provider
            .subscribe_logs(&filter)
            .await
            .map_err(|e| IndexerError::Subscription(format!("subscribe_logs: {e}")))?;

        let mut stream = sub.into_stream();
        info!("subscription established");

        while let Some(log) = stream.next().await {
            if let Err(e) = self.handle_log(log).await {
                warn!(error = ?e, "log handling failure");
            }
            if self.out.is_closed() {
                debug!("downstream closed; stopping subscriber");
                return Ok(());
            }
        }
        Err(IndexerError::Subscription("stream ended".into()))
    }

    async fn handle_log(&self, log: Log) -> Result<(), IndexerError> {
        let topic0 = log
            .topic0()
            .copied()
            .ok_or_else(|| IndexerError::Decode("log without topic0".into()))?;
        let address = log.address();
        let Some(launchpad) = self.registry.match_log(address, topic0) else {
            return Ok(()); // not from a tracked factory
        };

        let deploy = decode_deploy(launchpad, &log)?;
        debug!(
            launchpad = %launchpad,
            token = %format!("{:#x}", deploy.token),
            "decoded deploy",
        );
        self.out
            .send(deploy)
            .await
            .map_err(|_| IndexerError::Subscription("downstream channel closed".into()))?;
        Ok(())
    }
}
