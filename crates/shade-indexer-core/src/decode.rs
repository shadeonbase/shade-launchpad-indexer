use crate::error::IndexerError;
use crate::types::{Launchpad, NormalizedDeploy};
use alloy::rpc::types::Log;
use alloy_primitives::{Address, B256, U256};
use serde_json::json;

/// Decode a raw log to a [`NormalizedDeploy`] using launchpad-specific layout.
///
/// The decoders below are intentionally tolerant: when an event layout changes
/// upstream we want the indexer to keep flowing rather than panic. Fields that
/// cannot be located degrade to `None` / `Value::Null` and downstream
/// enrichment fills the gap.
pub fn decode_deploy(launchpad: Launchpad, log: &Log) -> Result<NormalizedDeploy, IndexerError> {
    let block_number = log
        .block_number
        .ok_or_else(|| IndexerError::Decode("log missing block_number".into()))?;
    let block_timestamp = log.block_timestamp.unwrap_or_default();
    let tx_hash = log
        .transaction_hash
        .ok_or_else(|| IndexerError::Decode("log missing tx_hash".into()))?;
    let log_index = log
        .log_index
        .ok_or_else(|| IndexerError::Decode("log missing log_index".into()))?;

    let topics: &[B256] = log.topics();
    let data: &[u8] = log.data().data.as_ref();

    let (token, deployer, initial_supply, raw) = match launchpad {
        Launchpad::Clanker => decode_clanker(topics, data)?,
        Launchpad::Flaunch => decode_flaunch(topics, data)?,
        Launchpad::Bankr => decode_bankr(topics, data)?,
        Launchpad::Zora => decode_zora(topics, data)?,
    };

    Ok(NormalizedDeploy {
        launchpad,
        token,
        deployer,
        block_number,
        block_timestamp,
        tx_hash,
        log_index,
        initial_supply,
        raw,
    })
}

type DecodeOut = (Address, Address, Option<U256>, serde_json::Value);

/// Clanker `TokenCreated(address indexed token, address indexed deployer, uint256 supply)`
fn decode_clanker(topics: &[B256], data: &[u8]) -> Result<DecodeOut, IndexerError> {
    let token = topic_to_address(topics, 1, "clanker.token")?;
    let deployer = topic_to_address(topics, 2, "clanker.deployer")?;
    let supply = data_to_u256(data, 0).ok();
    Ok((token, deployer, supply, json!({"layout": "TokenCreated"})))
}

/// Flaunch `Flaunched(address indexed token, address indexed deployer, bytes32 salt)`
fn decode_flaunch(topics: &[B256], data: &[u8]) -> Result<DecodeOut, IndexerError> {
    let token = topic_to_address(topics, 1, "flaunch.token")?;
    let deployer = topic_to_address(topics, 2, "flaunch.deployer")?;
    let salt = data
        .get(0..32)
        .map(|b| format!("0x{}", hex::encode(b)))
        .unwrap_or_default();
    Ok((
        token,
        deployer,
        None,
        json!({ "layout": "Flaunched", "salt": salt }),
    ))
}

/// Bankr `Spawn(address indexed token, address indexed deployer)`
fn decode_bankr(topics: &[B256], _data: &[u8]) -> Result<DecodeOut, IndexerError> {
    let token = topic_to_address(topics, 1, "bankr.token")?;
    let deployer = topic_to_address(topics, 2, "bankr.deployer")?;
    Ok((token, deployer, None, json!({"layout": "Spawn"})))
}

/// Zora `Created1155(address indexed token, address indexed deployer, ...)`
fn decode_zora(topics: &[B256], _data: &[u8]) -> Result<DecodeOut, IndexerError> {
    let token = topic_to_address(topics, 1, "zora.token")?;
    let deployer = topic_to_address(topics, 2, "zora.deployer")?;
    Ok((token, deployer, None, json!({"layout": "Created1155"})))
}

fn topic_to_address(topics: &[B256], idx: usize, what: &str) -> Result<Address, IndexerError> {
    let t = topics
        .get(idx)
        .ok_or_else(|| IndexerError::Decode(format!("{what}: topic[{idx}] missing")))?;
    Ok(Address::from_word(*t))
}

fn data_to_u256(data: &[u8], slot: usize) -> Result<U256, IndexerError> {
    let start = slot * 32;
    let end = start + 32;
    let chunk = data
        .get(start..end)
        .ok_or_else(|| IndexerError::Decode(format!("data slot[{slot}] out of range")))?;
    Ok(U256::from_be_slice(chunk))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::rpc::types::Log;
    use alloy_primitives::{LogData, U256};

    fn pad_address(a: Address) -> B256 {
        let mut bytes = [0u8; 32];
        bytes[12..].copy_from_slice(a.as_slice());
        B256::from(bytes)
    }

    #[test]
    fn decodes_clanker() {
        let token = Address::from([0x11u8; 20]);
        let deployer = Address::from([0x22u8; 20]);
        let supply = U256::from(1_000_000u64);
        let data: Vec<u8> = supply.to_be_bytes::<32>().to_vec();

        let inner = alloy::primitives::Log {
            address: Address::ZERO,
            data: LogData::new_unchecked(
                vec![
                    B256::from([0xAA; 32]),
                    pad_address(token),
                    pad_address(deployer),
                ],
                data.into(),
            ),
        };
        let log = Log {
            inner,
            block_number: Some(1),
            block_timestamp: Some(1_700_000_000),
            transaction_hash: Some(B256::from([0xBB; 32])),
            log_index: Some(0),
            ..Default::default()
        };

        let out = decode_deploy(Launchpad::Clanker, &log).unwrap();
        assert_eq!(out.token, token);
        assert_eq!(out.deployer, deployer);
        assert_eq!(out.initial_supply, Some(supply));
    }
}
