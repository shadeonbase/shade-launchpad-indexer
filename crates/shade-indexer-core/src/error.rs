use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("config: {0}")]
    Config(String),

    #[error("decode: {0}")]
    Decode(String),

    #[error("subscription: {0}")]
    Subscription(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
