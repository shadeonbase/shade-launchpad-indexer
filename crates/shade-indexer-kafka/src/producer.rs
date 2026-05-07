use rdkafka::config::ClientConfig;
use rdkafka::error::KafkaError;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use rdkafka::util::Timeout;
use shade_indexer_core::NormalizedDeploy;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, instrument};

#[derive(Debug, Error)]
pub enum ProducerError {
    #[error("kafka: {0}")]
    Kafka(#[from] KafkaError),

    #[error("serialize: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("send timeout/cancelled")]
    SendCancelled,
}

#[derive(Debug, Clone)]
pub struct ProducerConfig {
    pub brokers: String,
    pub transactional_id: String,
    pub linger_ms: u32,
    pub message_timeout_ms: u32,
}

impl Default for ProducerConfig {
    fn default() -> Self {
        Self {
            brokers: "localhost:9092".to_string(),
            transactional_id: "shade-indexer".to_string(),
            linger_ms: 5,
            message_timeout_ms: 30_000,
        }
    }
}

pub struct LaunchpadProducer {
    inner: FutureProducer,
}

impl LaunchpadProducer {
    #[instrument(skip(cfg), fields(brokers = %cfg.brokers, txn_id = %cfg.transactional_id))]
    pub fn new(cfg: ProducerConfig) -> Result<Self, ProducerError> {
        let inner: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", &cfg.brokers)
            .set("enable.idempotence", "true")
            .set("transactional.id", &cfg.transactional_id)
            .set("acks", "all")
            .set("compression.type", "zstd")
            .set("max.in.flight.requests.per.connection", "5")
            .set("linger.ms", cfg.linger_ms.to_string())
            .set("message.timeout.ms", cfg.message_timeout_ms.to_string())
            .create()?;

        inner.init_transactions(Timeout::After(Duration::from_secs(10)))?;
        info!("kafka producer ready (transactional)");
        Ok(Self { inner })
    }

    /// Publish a single deploy as a one-message transaction. The transactional
    /// envelope guarantees that consumers configured with
    /// `isolation.level=read_committed` only see committed messages, even if
    /// the indexer crashes mid-batch.
    pub async fn publish(&self, deploy: &NormalizedDeploy) -> Result<(), ProducerError> {
        let topic = deploy.launchpad.topic();
        let key = deploy.key();
        let payload = serde_json::to_vec(deploy)?;

        self.inner.begin_transaction()?;

        let record: FutureRecord<'_, String, Vec<u8>> =
            FutureRecord::to(topic).key(&key).payload(&payload);
        let send_res = self
            .inner
            .send(record, Timeout::After(Duration::from_secs(5)))
            .await;

        match send_res {
            Ok((partition, offset)) => {
                debug!(topic, partition, offset, key = %key, "published");
            }
            Err((e, _)) => {
                let _ = self
                    .inner
                    .abort_transaction(Timeout::After(Duration::from_secs(5)));
                error!(error = ?e, topic, "send failed; transaction aborted");
                return Err(ProducerError::Kafka(e));
            }
        }

        self.inner
            .commit_transaction(Timeout::After(Duration::from_secs(10)))?;
        metrics::counter!("shade_indexer_kafka_published", "launchpad" => deploy.launchpad.as_str())
            .increment(1);
        Ok(())
    }

    /// Publish a batch under a single transaction. Either all messages are
    /// committed or none — important for ordered backfill replays.
    pub async fn publish_batch(&self, deploys: &[NormalizedDeploy]) -> Result<(), ProducerError> {
        if deploys.is_empty() {
            return Ok(());
        }

        self.inner.begin_transaction()?;

        for deploy in deploys {
            let topic = deploy.launchpad.topic();
            let key = deploy.key();
            let payload = serde_json::to_vec(deploy)?;
            let record: FutureRecord<'_, String, Vec<u8>> =
                FutureRecord::to(topic).key(&key).payload(&payload);
            if let Err((e, _)) = self
                .inner
                .send(record, Timeout::After(Duration::from_secs(5)))
                .await
            {
                let _ = self
                    .inner
                    .abort_transaction(Timeout::After(Duration::from_secs(5)));
                return Err(ProducerError::Kafka(e));
            }
        }

        self.inner
            .commit_transaction(Timeout::After(Duration::from_secs(10)))?;
        metrics::counter!("shade_indexer_kafka_batches").increment(1);
        metrics::counter!("shade_indexer_kafka_published_batch_size")
            .increment(deploys.len() as u64);
        Ok(())
    }
}
