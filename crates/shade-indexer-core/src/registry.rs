use crate::error::IndexerError;
use crate::types::Launchpad;
use alloy_primitives::{Address, B256};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
struct RawFactories {
    #[serde(default)]
    clanker: Option<RawFactory>,
    #[serde(default)]
    flaunch: Option<RawFactory>,
    #[serde(default)]
    bankr: Option<RawFactory>,
    #[serde(default)]
    zora: Option<RawFactory>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawFactory {
    address: String,
    event_topic: String,
    #[serde(default)]
    abi_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FactorySpec {
    pub launchpad: Launchpad,
    pub address: Address,
    pub event_topic: B256,
    pub abi_path: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FactoryRegistry {
    by_launchpad: HashMap<Launchpad, FactorySpec>,
    /// Lookup table: (address, topic0) → launchpad.
    routing: HashMap<(Address, B256), Launchpad>,
}

impl FactoryRegistry {
    pub fn from_toml_path(path: impl AsRef<Path>) -> Result<Self, IndexerError> {
        let body = fs::read_to_string(path).map_err(IndexerError::Io)?;
        Self::from_toml_str(&body)
    }

    pub fn from_toml_str(body: &str) -> Result<Self, IndexerError> {
        let parsed: RawFactories = toml::from_str(body)
            .map_err(|e| IndexerError::Config(format!("factories.toml: {e}")))?;

        let mut reg = Self::default();
        for (lp, raw) in [
            (Launchpad::Clanker, parsed.clanker),
            (Launchpad::Flaunch, parsed.flaunch),
            (Launchpad::Bankr, parsed.bankr),
            (Launchpad::Zora, parsed.zora),
        ] {
            if let Some(raw) = raw {
                reg.insert(lp, &raw)?;
            }
        }
        if reg.by_launchpad.is_empty() {
            return Err(IndexerError::Config(
                "factories.toml has no launchpad entries".into(),
            ));
        }
        Ok(reg)
    }

    fn insert(&mut self, lp: Launchpad, raw: &RawFactory) -> Result<(), IndexerError> {
        let address = Address::from_str(raw.address.trim())
            .map_err(|e| IndexerError::Config(format!("{lp} address: {e}")))?;
        let event_topic = B256::from_str(raw.event_topic.trim())
            .map_err(|e| IndexerError::Config(format!("{lp} event_topic: {e}")))?;
        let spec = FactorySpec {
            launchpad: lp,
            address,
            event_topic,
            abi_path: raw.abi_path.clone(),
        };
        self.routing.insert((address, event_topic), lp);
        self.by_launchpad.insert(lp, spec);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.by_launchpad.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_launchpad.is_empty()
    }

    pub fn all_addresses(&self) -> Vec<Address> {
        self.by_launchpad.values().map(|s| s.address).collect()
    }

    pub fn all_topics(&self) -> Vec<B256> {
        self.by_launchpad.values().map(|s| s.event_topic).collect()
    }

    pub fn specs(&self) -> impl Iterator<Item = &FactorySpec> {
        self.by_launchpad.values()
    }

    pub fn match_log(&self, address: Address, topic0: B256) -> Option<Launchpad> {
        self.routing.get(&(address, topic0)).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[clanker]
address = "0x9b84fcE5Dcd9a38d2D01d5D72373F6b6b067c3e1"
event_topic = "0x1111111111111111111111111111111111111111111111111111111111111111"

[zora]
address = "0x777777C338d93e2C7adf08D102d45CA7CC4Ed021"
event_topic = "0x2222222222222222222222222222222222222222222222222222222222222222"
"#;

    #[test]
    fn parses_two_launchpads() {
        let reg = FactoryRegistry::from_toml_str(SAMPLE).unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.all_addresses().len(), 2);
        assert_eq!(reg.all_topics().len(), 2);
    }

    #[test]
    fn routing_resolves_launchpad() {
        let reg = FactoryRegistry::from_toml_str(SAMPLE).unwrap();
        let addr = Address::from_str("0x9b84fcE5Dcd9a38d2D01d5D72373F6b6b067c3e1").unwrap();
        let topic =
            B256::from_str("0x1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();
        assert_eq!(reg.match_log(addr, topic), Some(Launchpad::Clanker));
    }

    #[test]
    fn empty_registry_errors() {
        let err = FactoryRegistry::from_toml_str("").unwrap_err();
        assert!(matches!(err, IndexerError::Config(_)));
    }
}
