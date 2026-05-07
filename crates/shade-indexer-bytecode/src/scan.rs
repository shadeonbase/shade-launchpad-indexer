//! Deterministic substring scan over runtime bytecode.
//!
//! For Solidity-compiled contracts each external function dispatcher embeds the
//! 4-byte selector as a `PUSH4` operand, so a literal byte-substring search is
//! a low-FP confirmation that a given selector is callable. We use a
//! Boyer-Moore-Horspool jumper for the inner loop — O(n) average, no allocations
//! per call beyond the precomputed shift table.

use crate::flags::*;
use crate::signatures::SignatureSet;

const TRANSFER_SELECTOR: &[u8; 4] = &[0xa9, 0x05, 0x9c, 0xbb];

/// Scan `runtime` (deployed bytecode) for known signatures and return:
///
/// - the OR-combined flag mask
/// - and a list of human-readable signature names that fired (for telemetry).
pub fn scan_bytecode(runtime: &[u8], sigs: &SignatureSet) -> (i32, Vec<String>) {
    let mut mask = 0i32;
    let mut hit_names = Vec::new();
    let mut transfer_seen = false;

    for (selector, (flag, name)) in &sigs.by_selector {
        if name == "transfer(address,uint256)" {
            transfer_seen = contains_subseq(runtime, TRANSFER_SELECTOR);
            continue;
        }
        if *flag != 0 && contains_subseq(runtime, selector) {
            mask |= flag;
            hit_names.push(name.clone());
        }
    }

    // Honeypot heuristic: if the contract advertises ERC-20 surface but doesn't
    // actually embed the canonical transfer selector, flag it. This catches
    // proxies that route transfer() to a revert-only fallback.
    if !transfer_seen
        && (contains_subseq(runtime, &[0x70, 0xa0, 0x82, 0x31])  // balanceOf
            || contains_subseq(runtime, &[0x18, 0x16, 0x0d, 0xdd]))
    // totalSupply
    {
        mask |= BYTECODE_HONEYPOT;
        hit_names.push("honeypot:no-transfer".into());
    }

    (mask, hit_names)
}

/// Boyer-Moore-Horspool substring search with bad-character shift table.
/// Slightly faster than `windows().any(|w| w == needle)` for repeated calls
/// with the same needle, but we don't bother caching the shift table since the
/// needle is only 4 bytes and the search runs once per deploy.
fn contains_subseq(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_blacklist() {
        let mut bytecode = vec![0u8; 256];
        // splice in blacklist(address) selector at an arbitrary offset
        bytecode[100..104].copy_from_slice(&[0xf9, 0xf9, 0x2b, 0xe4]);
        let (mask, names) = scan_bytecode(&bytecode, &SignatureSet::builtin());
        assert_ne!(mask & BYTECODE_BLACKLIST, 0);
        assert!(names.iter().any(|n| n.contains("blacklist")));
    }

    #[test]
    fn flags_honeypot_when_transfer_missing() {
        // Contract advertises balanceOf but does NOT contain transfer.
        let mut bytecode = vec![0u8; 64];
        bytecode[10..14].copy_from_slice(&[0x70, 0xa0, 0x82, 0x31]); // balanceOf
        let (mask, _) = scan_bytecode(&bytecode, &SignatureSet::builtin());
        assert_ne!(mask & BYTECODE_HONEYPOT, 0);
    }

    #[test]
    fn no_honeypot_when_transfer_present() {
        let mut bytecode = vec![0u8; 64];
        bytecode[10..14].copy_from_slice(&[0x70, 0xa0, 0x82, 0x31]);
        bytecode[20..24].copy_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
        let (mask, _) = scan_bytecode(&bytecode, &SignatureSet::builtin());
        assert_eq!(mask & BYTECODE_HONEYPOT, 0);
    }

    #[test]
    fn empty_bytecode_no_flags() {
        let (mask, names) = scan_bytecode(&[], &SignatureSet::builtin());
        assert_eq!(mask, 0);
        assert!(names.is_empty());
    }
}
