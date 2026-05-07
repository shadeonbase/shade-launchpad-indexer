use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Launchpad {
    Clanker,
    Flaunch,
    Bankr,
    Zora,
}

impl Launchpad {
    pub fn as_str(&self) -> &'static str {
        match self {
            Launchpad::Clanker => "clanker",
            Launchpad::Flaunch => "flaunch",
            Launchpad::Bankr => "bankr",
            Launchpad::Zora => "zora",
        }
    }

    pub fn topic(&self) -> &'static str {
        match self {
            Launchpad::Clanker => "shade.launches.clanker",
            Launchpad::Flaunch => "shade.launches.flaunch",
            Launchpad::Bankr => "shade.launches.bankr",
            Launchpad::Zora => "shade.launches.zora",
        }
    }
}

impl fmt::Display for Launchpad {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedDeploy {
    pub launchpad: Launchpad,
    pub token: Address,
    pub deployer: Address,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_hash: B256,
    pub log_index: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_supply: Option<U256>,
    /// Launchpad-specific extras (cast hash, salt, royalty bps, etc.).
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub raw: serde_json::Value,
}

impl NormalizedDeploy {
    pub fn key(&self) -> String {
        format!("{:#x}", self.token)
    }
}
