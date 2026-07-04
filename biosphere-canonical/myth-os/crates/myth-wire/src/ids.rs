use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── MythId ────────────────────────────────────────────────────────────────────

/// A cryptographically unique identity for every entity, capsule, module, or asset.
///
/// Wraps a UUID v4. Use `MythId::new()` to generate. Use `MythId::from_uuid()`
/// to reconstruct from a stored value. Always serialize as the UUID string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MythId(Uuid);

impl MythId {
    /// Generate a new random MythId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Reconstruct from an existing UUID (e.g. loaded from disk).
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse from a UUID string. Returns None if the string is not a valid UUID.
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Returns the UUID as a lowercase hyphenated string.
    /// Allocates — use `Display` (format!/to_string) when you don't need to store it.
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for MythId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MythId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── ChannelId ─────────────────────────────────────────────────────────────────

/// Identifies a Theater channel (1–16 canonical, user channels above 16).
///
/// The 16 canonical channels are defined in the biospark-theater skill.
/// Channels are u32 so user projects can extend beyond 16 without constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(pub u32);

impl ChannelId {
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ch{:02}", self.0)
    }
}

// ── Blake3Hash ────────────────────────────────────────────────────────────────

/// A BLAKE3 content fingerprint — 32 bytes.
///
/// Used for B-DNA lineage tracking, capsule provenance, and canon event stamping.
/// Every capsule that enters a sealed Genesis container must have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; 32]);

impl Blake3Hash {
    pub fn of(data: &[u8]) -> Self {
        Self(*blake3::hash(data).as_bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn myth_id_round_trips_bincode() {
        let id = MythId::new();
        let bytes = bincode::serialize(&id).unwrap();
        let back: MythId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn myth_id_parse_display_round_trip() {
        let id = MythId::new();
        let s = id.to_string();
        let back = MythId::parse(&s).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn channel_id_display() {
        assert_eq!(ChannelId::new(1).to_string(),  "Ch01");
        assert_eq!(ChannelId::new(12).to_string(), "Ch12");
    }

    #[test]
    fn blake3_hash_is_deterministic() {
        let h1 = Blake3Hash::of(b"myth-os");
        let h2 = Blake3Hash::of(b"myth-os");
        assert_eq!(h1, h2);
        let h3 = Blake3Hash::of(b"different");
        assert_ne!(h1, h3);
    }
}
