use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

use crate::config;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Directory of *.sql files to apply in lexical order.
    #[arg(long, default_value = "migrations")]
    pub dir: PathBuf,
}

pub async fn run(config_path: PathBuf, args: Args) -> Result<()> {
    let cfg = config::load(&config_path)?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&cfg.postgres.url)
        .await
        .context("postgres connect")?;

    let mut entries: Vec<_> = std::fs::read_dir(&args.dir)
        .with_context(|| format!("read_dir {}", args.dir.display()))?
        .filter_map(|r| r.ok())
        .filter(|e| e.path().extension().map(|x| x == "sql").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let sql =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        info!(path = %path.display(), "applying migration");
        sqlx::raw_sql(&sql)
            .execute(&pool)
            .await
            .with_context(|| format!("apply {}", path.display()))?;
    }
    info!("migrations applied");
    Ok(())
}
