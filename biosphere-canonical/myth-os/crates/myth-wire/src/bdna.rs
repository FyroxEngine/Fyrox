use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── BDna ─────────────────────────────────────────────────────────────────────

/// B-DNA: the deterministic identity signature of a Quantum entity.
///
/// Exactly 64 boolean positions — this invariant is permanent and must never
/// be relaxed. Every capsule that enters a sealed Genesis container must have a
/// valid BDna. Nothing without provenance enters the archive.
///
/// The 64 positions encode traits across 7 trait tables. The specific meaning
/// of each position is defined in the `biospark-bdna` skill (not yet written —
/// for now treat positions as opaque bits with known length).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BDna(Vec<bool>);

/// The invariant length of a BDna sequence.
pub const BDNA_LENGTH: usize = 64;

#[derive(Debug, Error)]
pub enum BDnaError {
    #[error("BDna must be exactly {BDNA_LENGTH} bits, got {0}")]
    InvalidLength(usize),
}

impl BDna {
    /// Construct from exactly 64 bits. Returns an error if len ≠ 64.
    pub fn new(bits: Vec<bool>) -> Result<Self, BDnaError> {
        if bits.len() != BDNA_LENGTH {
            return Err(BDnaError::InvalidLength(bits.len()));
        }
        Ok(Self(bits))
    }

    /// Generate a zeroed BDna (all false). Used as a neutral seed.
    pub fn zero() -> Self {
        Self(vec![false; BDNA_LENGTH])
    }

    /// Generate a BDna from a BLAKE3 hash of the given seed bytes.
    /// Deterministic: same seed always produces the same BDna.
    pub fn from_seed(seed: &[u8]) -> Self {
        let hash = blake3::hash(seed);
        let bytes = hash.as_bytes();
        // Expand 32 bytes → 64 bits by taking one bit per byte (MSB)
        let bits: Vec<bool> = bytes.iter().flat_map(|b| {
            (0..8u8).rev().map(move |i| (b >> i) & 1 == 1)
        }).take(BDNA_LENGTH).collect();
        Self(bits)
    }

    /// The 64 boolean positions.
    pub fn bits(&self) -> &[bool] {
        &self.0
    }

    /// Returns the bit at position `i` (0-indexed). Panics if i ≥ 64.
    pub fn get(&self, i: usize) -> bool {
        self.0[i]
    }

    /// XOR this BDna with another, producing a child signature.
    /// Used for lineage derivation — child inherits from both parents.
    pub fn derive_child(&self, other: &BDna) -> BDna {
        let bits = self.0.iter().zip(other.0.iter()).map(|(a, b)| a ^ b).collect();
        BDna(bits)
    }

    /// Hamming distance between two BDna sequences (number of differing bits).
    pub fn distance(&self, other: &BDna) -> u32 {
        self.0.iter().zip(other.0.iter()).filter(|(a, b)| a != b).count() as u32
    }
}

impl std::fmt::Display for BDna {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bit in &self.0 {
            write!(f, "{}", if *bit { '1' } else { '0' })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length() {
        assert!(BDna::new(vec![false; 63]).is_err());
        assert!(BDna::new(vec![false; 65]).is_err());
        assert!(BDna::new(vec![false; 64]).is_ok());
    }

    #[test]
    fn zero_is_64_bits() {
        let z = BDna::zero();
        assert_eq!(z.bits().len(), BDNA_LENGTH);
        assert!(z.bits().iter().all(|&b| !b));
    }

    #[test]
    fn from_seed_is_deterministic() {
        let a = BDna::from_seed(b"myth-os-world-1");
        let b = BDna::from_seed(b"myth-os-world-1");
        assert_eq!(a, b);
        let c = BDna::from_seed(b"myth-os-world-2");
        assert_ne!(a, c);
    }

    #[test]
    fn from_seed_is_64_bits() {
        let d = BDna::from_seed(b"test");
        assert_eq!(d.bits().len(), BDNA_LENGTH);
    }

    #[test]
    fn distance_zero_to_self() {
        let a = BDna::from_seed(b"a");
        assert_eq!(a.distance(&a), 0);
    }

    #[test]
    fn round_trips_bincode() {
        let d = BDna::from_seed(b"bincode-test");
        let bytes = bincode::serialize(&d).unwrap();
        let back: BDna = bincode::deserialize(&bytes).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn display_is_64_chars() {
        let d = BDna::from_seed(b"display");
        let s = d.to_string();
        assert_eq!(s.len(), BDNA_LENGTH);
        assert!(s.chars().all(|c| c == '0' || c == '1'));
    }
}
