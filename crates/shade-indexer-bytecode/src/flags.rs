//! Bitmask flags written into `deploy_enrichment.bytecode_flags`.
//!
//! Flag values are part of the public schema. **Do not renumber existing
//! values** — only append new ones. Persisted history depends on stability.

pub const BYTECODE_HONEYPOT: i32 = 1 << 0;
pub const BYTECODE_MINT_BACKDOOR: i32 = 1 << 1;
pub const BYTECODE_FEE_ON_TRANSFER: i32 = 1 << 2;
pub const BYTECODE_BLACKLIST: i32 = 1 << 3;
pub const BYTECODE_PAUSABLE: i32 = 1 << 4;
pub const BYTECODE_OWNERSHIP_TRAP: i32 = 1 << 5;

/// Decode a flag mask back into its named components for logging or display.
pub fn flag_names(mask: i32) -> Vec<&'static str> {
    let mut out = Vec::new();
    if mask & BYTECODE_HONEYPOT != 0 {
        out.push("honeypot");
    }
    if mask & BYTECODE_MINT_BACKDOOR != 0 {
        out.push("mint_backdoor");
    }
    if mask & BYTECODE_FEE_ON_TRANSFER != 0 {
        out.push("fee_on_transfer");
    }
    if mask & BYTECODE_BLACKLIST != 0 {
        out.push("blacklist");
    }
    if mask & BYTECODE_PAUSABLE != 0 {
        out.push("pausable");
    }
    if mask & BYTECODE_OWNERSHIP_TRAP != 0 {
        out.push("ownership_trap");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_names_combine() {
        let mask = BYTECODE_HONEYPOT | BYTECODE_BLACKLIST;
        assert_eq!(flag_names(mask), vec!["honeypot", "blacklist"]);
    }

    #[test]
    fn empty_mask() {
        assert_eq!(flag_names(0), Vec::<&str>::new());
    }
}
