use arc_swap::ArcSwap;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

/// Heartbeats from the pipeline so /readyz can answer truthfully.
#[derive(Debug, Default)]
pub struct Liveness {
    pub last_event_unix: ArcSwap<u64>,
    pub kafka_connected: ArcSwap<bool>,
    pub postgres_connected: ArcSwap<bool>,
}

impl Liveness {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn mark_event(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        self.last_event_unix.store(Arc::new(now));
    }

    pub fn set_kafka(&self, ok: bool) {
        self.kafka_connected.store(Arc::new(ok));
    }
    pub fn set_postgres(&self, ok: bool) {
        self.postgres_connected.store(Arc::new(ok));
    }
}

#[derive(Serialize)]
struct ReadyResponse {
    ready: bool,
    last_event_unix: u64,
    seconds_since_event: u64,
    kafka_connected: bool,
    postgres_connected: bool,
}

/// Stale threshold for "no event seen recently". 5 minutes is generous; a
/// healthy ingest stream sees something every few seconds.
pub const STALE_THRESHOLD: Duration = Duration::from_secs(300);

pub async fn serve(addr: SocketAddr, liveness: Arc<Liveness>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .with_state(liveness);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "health server bound");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(l): State<Arc<Liveness>>) -> impl IntoResponse {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let last = **l.last_event_unix.load();
    let kafka = **l.kafka_connected.load();
    let pg = **l.postgres_connected.load();
    let stale = last == 0 || now.saturating_sub(last) > STALE_THRESHOLD.as_secs();
    let ready = kafka && pg && !stale;
    let body = ReadyResponse {
        ready,
        last_event_unix: last,
        seconds_since_event: now.saturating_sub(last),
        kafka_connected: kafka,
        postgres_connected: pg,
    };
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(body))
}
