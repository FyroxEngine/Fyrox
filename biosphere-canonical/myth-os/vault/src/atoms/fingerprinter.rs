// VAULT-ATOM-02: Cryptographic Fingerprinter
// Assigns BLAKE3 hash identities to all capsule payloads.

use mythos::identity::Blake3Hash;

pub struct Fingerprinter;

impl Fingerprinter {
    pub fn hash(data: &[u8]) -> Blake3Hash {
        let hash = blake3::hash(data);
        Blake3Hash(*hash.as_bytes())
    }

    /// Returns true if the data matches the expected fingerprint.
    pub fn verify(data: &[u8], expected: &Blake3Hash) -> bool {
        let actual = blake3::hash(data);
        actual.as_bytes() == &expected.0
    }

    /// Hash multiple slices without allocating a concatenated buffer (tree hashing).
    pub fn hash_many(parts: &[&[u8]]) -> Blake3Hash {
        let mut hasher = blake3::Hasher::new();
        for part in parts {
            hasher.update(part);
        }
        Blake3Hash(*hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_verify() {
        let data = b"quill actor payload v1";
        let fp = Fingerprinter::hash(data);
        assert!(Fingerprinter::verify(data, &fp));
        assert!(!Fingerprinter::verify(b"tampered", &fp));
    }
}
