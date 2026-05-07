use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use anyhow::{anyhow, Context, Result};
use clap::Args as ClapArgs;
use shade_indexer_core::{decode::decode_deploy, FactoryRegistry};
use std::path::PathBuf;
use tracing::{info, warn};

use crate::config;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Inclusive starting block.
    #[arg(long)]
    pub from_block: u64,

    /// Inclusive ending block. If omitted, uses provider's latest block.
    #[arg(long)]
    pub to_block: Option<u64>,

    /// Page size (blocks per `eth_getLogs` request). Base typically tolerates 5_000+.
    #[arg(long, default_value_t = 2_000)]
    pub page_size: u64,

    /// Print decoded deploys as JSON to stdout instead of doing anything else.
    #[arg(long, default_value_t = true)]
    pub stdout: bool,
}

pub async fn run(config_path: PathBuf, args: Args) -> Result<()> {
    let cfg = config::load(&config_path)?;
    let http_url = cfg.rpc.http_url.clone().ok_or_else(|| {
        anyhow!(
            "backfill requires [rpc].http_url; ws-only providers don't expose getLogs efficiently"
        )
    })?;
    let provider = ProviderBuilder::new().on_http(http_url.parse().context("http_url")?);
    let registry = FactoryRegistry::from_toml_path(&cfg.factories)
        .with_context(|| format!("factories: {}", cfg.factories.display()))?;

    let to = match args.to_block {
        Some(b) => b,
        None => provider.get_block_number().await.context("latest block")?,
    };
    let from = args.from_block;
    if from > to {
        return Err(anyhow!("from_block ({from}) > to_block ({to})"));
    }
    info!(from, to, page_size = args.page_size, "backfill starting");

    let mut total = 0u64;
    let mut start = from;
    while start <= to {
        let end = (start + args.page_size - 1).min(to);
        let filter = Filter::new()
            .address(registry.all_addresses())
            .event_signature(registry.all_topics())
            .from_block(start)
            .to_block(end);

        let logs = provider
            .get_logs(&filter)
            .await
            .with_context(|| format!("getLogs [{start}, {end}]"))?;
        for log in logs {
            let topic0 = match log.topic0() {
                Some(t) => *t,
                None => continue,
            };
            let address = log.address();
            let Some(launchpad) = registry.match_log(address, topic0) else {
                continue;
            };
            match decode_deploy(launchpad, &log) {
                Ok(deploy) => {
                    if args.stdout {
                        println!("{}", serde_json::to_string(&deploy).unwrap());
                    }
                    total += 1;
                }
                Err(e) => warn!(error = ?e, "decode failure"),
            }
        }
        start = end + 1;
    }

    info!(total, "backfill complete");
    Ok(())
}
