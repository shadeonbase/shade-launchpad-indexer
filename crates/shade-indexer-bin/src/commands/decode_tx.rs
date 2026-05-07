use alloy::providers::{Provider, ProviderBuilder};
use alloy_primitives::B256;
use anyhow::{anyhow, Context, Result};
use clap::Args as ClapArgs;
use shade_indexer_core::{decode::decode_deploy, FactoryRegistry};
use std::path::PathBuf;
use std::str::FromStr;
use tracing::warn;

use crate::config;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Tx hash (0x-prefixed) to fetch and decode.
    pub tx_hash: String,
}

pub async fn run(config_path: PathBuf, args: Args) -> Result<()> {
    let cfg = config::load(&config_path)?;
    let http_url =
        cfg.rpc.http_url.clone().ok_or_else(|| {
            anyhow!("decode-tx requires [rpc].http_url for getTransactionReceipt")
        })?;
    let provider = ProviderBuilder::new().on_http(http_url.parse().context("http_url")?);
    let registry = FactoryRegistry::from_toml_path(&cfg.factories)
        .with_context(|| format!("factories: {}", cfg.factories.display()))?;

    let hash = B256::from_str(&args.tx_hash).context("tx_hash")?;
    let receipt = provider
        .get_transaction_receipt(hash)
        .await
        .context("getTransactionReceipt")?
        .ok_or_else(|| anyhow!("tx not found: {}", args.tx_hash))?;

    let mut found = 0;
    for log in receipt.inner.logs() {
        let topic0 = match log.topic0() {
            Some(t) => *t,
            None => continue,
        };
        let Some(launchpad) = registry.match_log(log.address(), topic0) else {
            continue;
        };
        match decode_deploy(launchpad, log) {
            Ok(deploy) => {
                println!("{}", serde_json::to_string_pretty(&deploy)?);
                found += 1;
            }
            Err(e) => warn!(error = ?e, "decode failure"),
        }
    }
    if found == 0 {
        eprintln!("no factory events found in {}", args.tx_hash);
    }
    Ok(())
}
