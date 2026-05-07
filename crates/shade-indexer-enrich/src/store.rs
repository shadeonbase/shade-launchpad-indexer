use chrono::{DateTime, TimeZone, Utc};
use shade_indexer_core::NormalizedDeploy;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, instrument};

use crate::metrics::Enrichment;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct EnrichStore {
    pool: PgPool,
}

impl EnrichStore {
    pub async fn connect(url: &str, max_conns: u32) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_conns)
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await?;
        Ok(Self { pool })
    }

    /// Insert (or no-op on conflict) a deploy row, returning its id.
    #[instrument(skip(self, deploy))]
    pub async fn upsert_deploy(&self, deploy: &NormalizedDeploy) -> Result<i64, StoreError> {
        let ts: DateTime<Utc> = Utc
            .timestamp_opt(deploy.block_timestamp as i64, 0)
            .single()
            .unwrap_or_else(Utc::now);

        let row = sqlx::query(
            r#"
            INSERT INTO deploys
                (launchpad, token, deployer, block_number, block_timestamp, tx_hash)
            VALUES
                ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (tx_hash, token) DO UPDATE
                SET block_timestamp = EXCLUDED.block_timestamp
            RETURNING id
            "#,
        )
        .bind(deploy.launchpad.as_str())
        .bind(deploy.token.as_slice())
        .bind(deploy.deployer.as_slice())
        .bind(deploy.block_number as i64)
        .bind(ts)
        .bind(deploy.tx_hash.as_slice())
        .fetch_one(&self.pool)
        .await?;

        let id: i64 = row.try_get("id")?;
        debug!(id, "deploy upserted");
        Ok(id)
    }

    pub async fn write_enrichment(&self, deploy_id: i64, e: &Enrichment) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO deploy_enrichment
                (deploy_id, top10_share, gini, hhi, liq_to_fdv_ratio, liq_locked, bytecode_flags)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (deploy_id) DO UPDATE SET
                top10_share      = EXCLUDED.top10_share,
                gini             = EXCLUDED.gini,
                hhi              = EXCLUDED.hhi,
                liq_to_fdv_ratio = EXCLUDED.liq_to_fdv_ratio,
                liq_locked       = EXCLUDED.liq_locked,
                bytecode_flags   = EXCLUDED.bytecode_flags,
                enriched_at      = now()
            "#,
        )
        .bind(deploy_id)
        .bind(clamp_decimal(e.top10_share, 6, 4))
        .bind(clamp_decimal(e.gini, 6, 4))
        .bind(clamp_decimal(e.hhi, 8, 4))
        .bind(clamp_decimal(e.liq_to_fdv_ratio, 10, 6))
        .bind(e.liq_locked)
        .bind(e.bytecode_flags)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// Bound a fractional value into the range Postgres can store for the given
/// `(precision, scale)` of `NUMERIC(p,s)`.
fn clamp_decimal(value: f64, precision: u32, scale: u32) -> f64 {
    let max = 10f64.powi((precision - scale) as i32) - 10f64.powi(-(scale as i32));
    let v = if value.is_finite() { value } else { 0.0 };
    v.clamp(-max, max)
}
