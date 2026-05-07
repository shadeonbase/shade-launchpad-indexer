//! Curated signature set with bitmask flag attribution.
//!
//! Each signature is one of:
//!
//! - a 4-byte function selector (e.g. `keccak256("blacklist(address)")[0..4]`)
//! - an event topic (e.g. `keccak256("FeeChanged(uint256,uint256)")`)
//!
//! plus the flag bit it should raise when matched.

use crate::flags::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownSignature {
    /// Hex-encoded selector or topic (with or without `0x` prefix).
    pub selector: String,
    /// Bitmask flag this signature contributes to.
    pub flag: i32,
    /// Human-readable name (for telemetry only; not load-bearing).
    pub name: String,
}

#[derive(Debug, Default, Clone)]
pub struct SignatureSet {
    pub by_selector: HashMap<Vec<u8>, (i32, String)>,
}

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("hex: bad selector {0:?}")]
    Hex(String),
}

impl SignatureSet {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, SignatureError> {
        let body = std::fs::read_to_string(path)?;
        let raw: Vec<KnownSignature> = serde_json::from_str(&body)?;
        Self::try_from_iter(raw)
    }

    pub fn try_from_iter<I: IntoIterator<Item = KnownSignature>>(
        items: I,
    ) -> Result<Self, SignatureError> {
        let mut by_selector = HashMap::new();
        for s in items {
            let stripped = s.selector.strip_prefix("0x").unwrap_or(&s.selector);
            let bytes =
                hex::decode(stripped).map_err(|_| SignatureError::Hex(s.selector.clone()))?;
            by_selector.insert(bytes, (s.flag, s.name));
        }
        Ok(Self { by_selector })
    }

    pub fn len(&self) -> usize {
        self.by_selector.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_selector.is_empty()
    }

    /// Default in-tree set covering well-known scam patterns. Production
    /// deployments should load from JSON instead.
    pub fn builtin() -> Self {
        let items = vec![
            KnownSignature {
                selector: "0xf9f92be4".into(),
                flag: BYTECODE_BLACKLIST,
                name: "blacklist(address)".into(),
            },
            KnownSignature {
                selector: "0x537df3b6".into(),
                flag: BYTECODE_BLACKLIST,
                name: "addToBlacklist(address)".into(),
            },
            KnownSignature {
                selector: "0x40c10f19".into(),
                flag: BYTECODE_MINT_BACKDOOR,
                name: "mint(address,uint256)".into(),
            },
            KnownSignature {
                selector: "0xa9059cbb".into(),
                // transfer is not malicious by itself; intentionally NOT flagged.
                // Listed here only so a contract _missing_ it can be detected.
                flag: 0,
                name: "transfer(address,uint256)".into(),
            },
            KnownSignature {
                selector: "0x8456cb59".into(),
                flag: BYTECODE_PAUSABLE,
                name: "pause()".into(),
            },
            KnownSignature {
                selector: "0xf2fde38b".into(),
                flag: BYTECODE_OWNERSHIP_TRAP,
                name: "transferOwnership(address)".into(),
            },
            KnownSignature {
                selector: "0xe43252d7".into(),
                flag: BYTECODE_FEE_ON_TRANSFER,
                name: "setFee(uint256)".into(),
            },
        ];
        Self::try_from_iter(items).expect("builtin signatures parse")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_is_nonempty() {
        let s = SignatureSet::builtin();
        assert!(s.len() >= 5);
    }

    #[test]
    fn rejects_bad_hex() {
        let err = SignatureSet::try_from_iter(vec![KnownSignature {
            selector: "0xZZZZ".into(),
            flag: 1,
            name: "bad".into(),
        }])
        .unwrap_err();
        assert!(matches!(err, SignatureError::Hex(_)));
    }
}
