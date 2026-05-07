//! Pure-function metrics over a holder snapshot. Kept independent of any IO so
//! they're trivially unit-testable.

use serde::{Deserialize, Serialize};

// Re-exported from `shade-indexer-bytecode` to keep a single source of truth.
pub use shade_indexer_bytecode::{
    BYTECODE_FEE_ON_TRANSFER, BYTECODE_HONEYPOT, BYTECODE_MINT_BACKDOOR,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolderSnapshot {
    /// Holder balances in raw token units (decimals not normalized; ratios are decimals-invariant).
    pub balances: Vec<u128>,
    /// Pool liquidity in USD (or any common unit) at deploy + 1 block.
    pub liquidity_usd: f64,
    /// Fully-diluted valuation in the same unit as `liquidity_usd`.
    pub fdv_usd: f64,
    pub liquidity_locked: bool,
    pub bytecode_flags: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Enrichment {
    pub top10_share: f64,
    pub gini: f64,
    pub hhi: f64,
    pub liq_to_fdv_ratio: f64,
    pub liq_locked: bool,
    pub bytecode_flags: i32,
}

impl Enrichment {
    pub fn from_snapshot(s: &HolderSnapshot) -> Self {
        let total: u128 = s.balances.iter().sum();
        Self {
            top10_share: top_n_share(&s.balances, 10, total),
            gini: gini(&s.balances, total),
            hhi: hhi(&s.balances, total),
            liq_to_fdv_ratio: if s.fdv_usd > 0.0 {
                s.liquidity_usd / s.fdv_usd
            } else {
                0.0
            },
            liq_locked: s.liquidity_locked,
            bytecode_flags: s.bytecode_flags,
        }
    }
}

fn top_n_share(balances: &[u128], n: usize, total: u128) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let mut sorted: Vec<u128> = balances.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    let topn: u128 = sorted.iter().take(n).sum();
    (topn as f64) / (total as f64)
}

/// Gini coefficient via the sorted-cumulative formula:
/// `G = (2·Σ(i·x_i) − (n+1)·Σx_i) / (n·Σx_i)`
fn gini(balances: &[u128], total: u128) -> f64 {
    if balances.is_empty() || total == 0 {
        return 0.0;
    }
    let mut sorted: Vec<u128> = balances.to_vec();
    sorted.sort_unstable();
    let n = sorted.len() as f64;

    // Use f64 accumulators to avoid overflow on i*x_i for large balances.
    let mut weighted = 0.0;
    for (i, x) in sorted.iter().enumerate() {
        weighted += ((i + 1) as f64) * (*x as f64);
    }
    let sum = total as f64;
    let g = (2.0 * weighted - (n + 1.0) * sum) / (n * sum);
    g.clamp(0.0, 1.0)
}

/// Herfindahl-Hirschman index in [0, 1]: `Σ s_i²` where `s_i` is share.
fn hhi(balances: &[u128], total: u128) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let denom = total as f64;
    balances
        .iter()
        .map(|x| {
            let s = (*x as f64) / denom;
            s * s
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_distribution_zero_gini_low_hhi() {
        let s = HolderSnapshot {
            balances: vec![100; 10],
            liquidity_usd: 1.0,
            fdv_usd: 10.0,
            liquidity_locked: true,
            bytecode_flags: 0,
        };
        let e = Enrichment::from_snapshot(&s);
        assert!(e.gini.abs() < 1e-9, "gini was {}", e.gini);
        assert!((e.hhi - 0.1).abs() < 1e-9, "hhi was {}", e.hhi);
        assert!((e.liq_to_fdv_ratio - 0.1).abs() < 1e-9);
        assert!((e.top10_share - 1.0).abs() < 1e-9);
    }

    #[test]
    fn maximal_inequality_high_gini_high_hhi() {
        // One holder with 1_000_000, ninety-nine with 1.
        let mut balances = vec![1u128; 99];
        balances.push(1_000_000);
        let s = HolderSnapshot {
            balances,
            liquidity_usd: 0.0,
            fdv_usd: 100.0,
            liquidity_locked: false,
            bytecode_flags: 0,
        };
        let e = Enrichment::from_snapshot(&s);
        assert!(e.gini > 0.9, "gini was {}", e.gini);
        assert!(e.hhi > 0.95, "hhi was {}", e.hhi);
        assert!(!e.liq_locked);
    }

    #[test]
    fn empty_balances_safe() {
        let s = HolderSnapshot {
            balances: vec![],
            liquidity_usd: 0.0,
            fdv_usd: 0.0,
            liquidity_locked: false,
            bytecode_flags: 0,
        };
        let e = Enrichment::from_snapshot(&s);
        assert_eq!(e.top10_share, 0.0);
        assert_eq!(e.gini, 0.0);
        assert_eq!(e.hhi, 0.0);
        assert_eq!(e.liq_to_fdv_ratio, 0.0);
    }

    #[test]
    fn bytecode_flags_combine() {
        let flags = BYTECODE_HONEYPOT | BYTECODE_FEE_ON_TRANSFER;
        assert_eq!(flags & BYTECODE_HONEYPOT, BYTECODE_HONEYPOT);
        assert_eq!(flags & BYTECODE_MINT_BACKDOOR, 0);
    }
}
