use serde::{Deserialize, Serialize};
use crate::{MythId, WireType};

// ── WirePacket ────────────────────────────────────────────────────────────────

/// The single legal message type between all modules and the Theater.
///
/// Every signal in the Quantum Ecosystem travels as a WirePacket. There are no
/// other message types crossing module boundaries. Modules publish packets;
/// the Theater routes them by `wire_type`; adapters subscribe to what they need.
///
/// # Payload encoding
///
/// `payload` is a bincode-encoded byte blob. The receiving module knows how to
/// decode it based on `wire_type`. The Theater never inspects payload bytes —
/// it routes on type alone. This is intentional: the router stays dumb so
/// modules stay decoupled.
///
/// # Tick
///
/// `tick` is the engine's simulation tick counter at the moment of emission.
/// Adapters use it for frame alignment and replay. It is NOT a wall-clock
/// timestamp — the engine controls the tick rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirePacket {
    /// The signal type — determines routing and payload schema.
    pub wire_type: WireType,

    /// The entity that produced this packet.
    pub source_id: MythId,

    /// Engine simulation tick at time of emission.
    pub tick: u64,

    /// Bincode-encoded payload. Schema is defined by `wire_type`.
    /// The Theater never reads this. Receiving adapters decode it.
    pub payload: Vec<u8>,
}

impl WirePacket {
    /// Construct a new packet. `payload` should be pre-encoded with bincode.
    pub fn new(wire_type: WireType, source_id: MythId, tick: u64, payload: Vec<u8>) -> Self {
        Self { wire_type, source_id, tick, payload }
    }

    /// Encode a typed value into a WirePacket payload using bincode.
    ///
    /// This is the canonical way to build a packet from a concrete payload type.
    /// Returns an error only if the value fails to serialize (extremely rare for
    /// well-formed types that implement Serialize).
    pub fn encode<T: Serialize>(
        wire_type: WireType,
        source_id: MythId,
        tick: u64,
        value: &T,
    ) -> Result<Self, bincode::Error> {
        let payload = bincode::serialize(value)?;
        Ok(Self::new(wire_type, source_id, tick, payload))
    }

    /// Decode the payload bytes into a concrete type using bincode.
    ///
    /// The caller is responsible for knowing the correct type for the wire type.
    /// Decoding the wrong type is safe (returns an error) but meaningless.
    pub fn decode<T: for<'de> Deserialize<'de>>(&self) -> Result<T, bincode::Error> {
        bincode::deserialize(&self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MythId;

    #[test]
    fn round_trips_bincode() {
        let src = MythId::new();
        let pkt = WirePacket::new(WireType::Data, src, 42, vec![1, 2, 3]);
        let bytes = bincode::serialize(&pkt).unwrap();
        let back: WirePacket = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.tick, 42);
        assert_eq!(back.payload, vec![1, 2, 3]);
        assert_eq!(back.wire_type, WireType::Data);
    }

    #[test]
    fn encode_decode_round_trip() {
        let src = MythId::new();
        let message = String::from("hello myth-os");
        let pkt = WirePacket::encode(WireType::Narrative, src, 1, &message).unwrap();
        let decoded: String = pkt.decode().unwrap();
        assert_eq!(decoded, message);
    }

    #[test]
    fn all_wire_types_can_be_carried() {
        let src = MythId::new();
        for wt in WireType::ALL {
            let pkt = WirePacket::new(wt, src.clone(), 0, vec![]);
            let bytes = bincode::serialize(&pkt).unwrap();
            let back: WirePacket = bincode::deserialize(&bytes).unwrap();
            assert_eq!(back.wire_type, wt);
        }
    }

    #[test]
    fn tick_zero_is_valid() {
        let src = MythId::new();
        let pkt = WirePacket::new(WireType::Control, src, 0, vec![]);
        assert_eq!(pkt.tick, 0);
    }

    #[test]
    fn large_payload_survives() {
        let src = MythId::new();
        let payload = vec![0xABu8; 65_536];
        let pkt = WirePacket::new(WireType::Audio, src, 99, payload.clone());
        let bytes = bincode::serialize(&pkt).unwrap();
        let back: WirePacket = bincode::deserialize(&bytes).unwrap();
        assert_eq!(back.payload, payload);
    }
}
