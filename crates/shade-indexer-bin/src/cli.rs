use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::commands;

#[derive(Debug, Parser)]
#[command(
    name = "shade-indexer",
    version,
    about = "Multi-launchpad event indexer for Base"
)]
pub struct Cli {
    /// Path to the indexer config (TOML).
    #[arg(
        short,
        long,
        env = "SHADE_CONFIG",
        default_value = "config/indexer.toml"
    )]
    pub config: PathBuf,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Run the live ingestion pipeline (subscriber → kafka + enrichment).
    Serve(commands::serve::Args),

    /// Apply migrations against the configured Postgres URL.
    Migrate(commands::migrate::Args),

    /// Backfill a contiguous block range without producing to Kafka.
    Backfill(commands::backfill::Args),

    /// Print the resolved config + factory registry summary and exit.
    InspectConfig,

    /// Decode a single tx hash and print the normalized deploy(s).
    DecodeTx(commands::decode_tx::Args),
}

impl Cli {
    pub async fn run(self) -> Result<()> {
        match self.cmd {
            Cmd::Serve(args) => commands::serve::run(self.config, args).await,
            Cmd::Migrate(args) => commands::migrate::run(self.config, args).await,
            Cmd::Backfill(args) => commands::backfill::run(self.config, args).await,
            Cmd::InspectConfig => commands::inspect_config::run(self.config).await,
            Cmd::DecodeTx(args) => commands::decode_tx::run(self.config, args).await,
        }
    }
}
