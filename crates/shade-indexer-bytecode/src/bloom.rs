//! Bloom filter over arbitrary byte strings (used for 4-byte selectors).
//!
//! Sizing follows the standard formulas:
//!
//! - For `n` expected items and target false-positive rate `p`,
//!   `m = -n·ln p / (ln 2)²` bits and `k = (m/n)·ln 2` hashes.
//!
//! [`Bloom::with_capacity`] picks `(m, k)` automatically given `(n, p)`.

use blake2::digest::consts::U16;
use blake2::{Blake2b, Digest};

/// Standard Bloom filter with double-hashing trick:
/// `h_i(x) = h_a(x) + i·h_b(x) mod m`.
#[derive(Debug, Clone)]
pub struct Bloom {
    bits: Vec<u64>,
    m_bits: usize,
    k: u32,
    n_inserted: u64,
}

impl Bloom {
    pub fn new(m_bits: usize, k: u32) -> Self {
        assert!(m_bits >= 64, "bloom must have at least 64 bits");
        assert!(k >= 1, "bloom must have at least 1 hash");
        let words = m_bits.div_ceil(64);
        Self {
            bits: vec![0; words],
            m_bits,
            k,
            n_inserted: 0,
        }
    }

    /// Pick `(m, k)` automatically for `n` items at false-positive rate `p`.
    pub fn with_capacity(n: usize, p: f64) -> Self {
        assert!(n > 0, "n must be > 0");
        assert!(p > 0.0 && p < 1.0, "p must be in (0, 1)");
        let ln2 = std::f64::consts::LN_2;
        let m = (-(n as f64) * p.ln() / (ln2 * ln2)).ceil() as usize;
        let k = ((m as f64 / n as f64) * ln2).round().max(1.0) as u32;
        let m = m.next_power_of_two().max(64);
        Self::new(m, k)
    }

    pub fn m_bits(&self) -> usize {
        self.m_bits
    }
    pub fn k(&self) -> u32 {
        self.k
    }
    pub fn len(&self) -> u64 {
        self.n_inserted
    }
    pub fn is_empty(&self) -> bool {
        self.n_inserted == 0
    }

    /// Estimated false-positive rate for the current load factor.
    pub fn estimated_fpr(&self) -> f64 {
        let load = self.n_inserted as f64 / self.m_bits as f64;
        (1.0 - (-(self.k as f64) * load).exp()).powi(self.k as i32)
    }

    pub fn insert(&mut self, item: &[u8]) {
        let (a, b) = double_hash(item);
        for i in 0..self.k {
            let bit = combine(a, b, i, self.m_bits);
            self.set_bit(bit);
        }
        self.n_inserted += 1;
    }

    pub fn contains(&self, item: &[u8]) -> bool {
        let (a, b) = double_hash(item);
        for i in 0..self.k {
            let bit = combine(a, b, i, self.m_bits);
            if !self.get_bit(bit) {
                return false;
            }
        }
        true
    }

    fn set_bit(&mut self, bit: usize) {
        let (w, b) = (bit / 64, bit % 64);
        self.bits[w] |= 1u64 << b;
    }

    fn get_bit(&self, bit: usize) -> bool {
        let (w, b) = (bit / 64, bit % 64);
        (self.bits[w] >> b) & 1 == 1
    }
}

fn double_hash(item: &[u8]) -> (u64, u64) {
    let mut hasher = Blake2b::<U16>::new();
    hasher.update(item);
    let out = hasher.finalize();
    let a = u64::from_le_bytes(out[0..8].try_into().unwrap());
    let b = u64::from_le_bytes(out[8..16].try_into().unwrap());
    // Ensure b is odd so it's coprime with any power-of-two m_bits.
    (a, b | 1)
}

fn combine(a: u64, b: u64, i: u32, m_bits: usize) -> usize {
    let h = a.wrapping_add((i as u64).wrapping_mul(b));
    (h as usize) % m_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_and_finds() {
        let mut b = Bloom::with_capacity(100, 0.01);
        b.insert(b"\x70\xa0\x82\x31"); // balanceOf(address)
        b.insert(b"\xa9\x05\x9c\xbb"); // transfer
        assert!(b.contains(b"\x70\xa0\x82\x31"));
        assert!(b.contains(b"\xa9\x05\x9c\xbb"));
        assert!(!b.contains(b"\x00\x00\x00\x00"));
    }

    #[test]
    fn fpr_under_target() {
        let mut b = Bloom::with_capacity(1_000, 0.01);
        for i in 0u32..1_000 {
            b.insert(&i.to_le_bytes());
        }
        let mut hits = 0u32;
        for i in 1_000u32..11_000 {
            if b.contains(&i.to_le_bytes()) {
                hits += 1;
            }
        }
        // Allow 3x slack — 0.01 target, observed should be < ~0.04 in practice.
        assert!(hits < 400, "fpr too high: {hits}/10000");
    }

    #[test]
    fn auto_sized_pow2_m() {
        let b = Bloom::with_capacity(100, 0.01);
        assert!(b.m_bits().is_power_of_two());
        assert!(b.k() >= 1);
    }
}
