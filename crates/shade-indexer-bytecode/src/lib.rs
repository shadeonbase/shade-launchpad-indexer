//! Bytecode pattern detector.
//!
//! Two complementary checks:
//!
//! 1. [`Bloom`] — probabilistic membership test for known-malicious 4-byte
//!    function selectors. False positives are possible (capped by parameters);
//!    false negatives are not. Suitable for a hot path because every check is
//!    `k` hashes plus `k` bit reads, with no per-element comparison.
//!
//! 2. [`scan_bytecode`] — deterministic substring scan that converts a hit-set
//!    of selectors into the bitmask flags emitted into Postgres
//!    (`deploy_enrichment.bytecode_flags`). Slower than the bloom but exact.
//!
//! The expected layered usage is:
//!
//! ```text
//!   Bloom::contains(selector)  // cheap pre-filter
//!     ↓ if true
//!   scan_bytecode(runtime, &known_signatures)  // exact confirmation
//! ```

pub mod bloom;
pub mod flags;
pub mod scan;
pub mod signatures;

pub use bloom::Bloom;
pub use flags::{BYTECODE_FEE_ON_TRANSFER, BYTECODE_HONEYPOT, BYTECODE_MINT_BACKDOOR};
pub use scan::scan_bytecode;
pub use signatures::{KnownSignature, SignatureSet};
