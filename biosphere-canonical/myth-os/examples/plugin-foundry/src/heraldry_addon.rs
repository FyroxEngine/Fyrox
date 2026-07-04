/// HeraldryAddon — wildcard addon that stamps heraldry metadata onto every
/// outgoing WirePacket payload.
///
/// This is the example MythAddon. It attaches to ALL plugins ("*" target)
/// and intercepts outgoing packets. If the packet payload is valid JSON,
/// the addon injects a `_heraldry` key with the source plugin's symbol.
/// Non-JSON payloads pass through unchanged.
///
/// # Heraldry
/// Addons carry Sigils — independent of any instrument Crest.
/// HeraldryAddon symbol: `Sigil:Scribe`
use myth_plugin::{MythAddon, PluginResult};
use myth_wire::WirePacket;

pub struct HeraldryAddon {
    /// The heraldry symbol of this addon itself.
    symbol: String,
}

impl HeraldryAddon {
    pub fn new() -> Self {
        Self { symbol: "Sigil:Scribe".into() }
    }
}

impl Default for HeraldryAddon {
    fn default() -> Self { Self::new() }
}

impl MythAddon for HeraldryAddon {
    fn id(&self) -> &str { "heraldry-scribe" }

    /// Wildcard: attaches to every registered plugin.
    fn target_plugin(&self) -> &str { "*" }

    fn heraldry_symbol(&self) -> &str { &self.symbol }

    fn on_output(
        &self,
        source_packet: &WirePacket,
        output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        // For each outgoing packet, try to stamp heraldry into JSON payloads.
        let stamped = output.into_iter().map(|mut pkt| {
            if let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(&pkt.payload) {
                if let Some(obj) = value.as_object_mut() {
                    // Inject heraldry trace — who produced this packet and when.
                    obj.insert(
                        "_heraldry".into(),
                        serde_json::json!({
                            "scribe": self.symbol,
                            "origin_wire": format!("{:?}", source_packet.wire_type),
                            "stamped_at": chrono::Utc::now().timestamp(),
                        }),
                    );
                    if let Ok(bytes) = serde_json::to_vec(&value) {
                        pkt.payload = bytes;
                    }
                }
            }
            // Non-JSON payloads pass through with no modification.
            pkt
        }).collect();

        Ok(stamped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myth_wire::{MythId, WirePacket, WireType};

    #[test]
    fn stamps_json_payload() {
        let addon = HeraldryAddon::new();
        let source = WirePacket::new(WireType::Data, MythId::new(), 0, vec![]);
        let payload = serde_json::to_vec(&serde_json::json!({ "hello": "world" })).unwrap();
        let output = vec![WirePacket::new(WireType::Data, MythId::new(), 0, payload)];

        let result = addon.on_output(&source, output).unwrap();
        assert_eq!(result.len(), 1);

        let value: serde_json::Value = serde_json::from_slice(&result[0].payload).unwrap();
        assert!(value.get("_heraldry").is_some());
        assert_eq!(value["hello"], "world");
    }

    #[test]
    fn non_json_passes_through_unchanged() {
        let addon = HeraldryAddon::new();
        let source = WirePacket::new(WireType::Data, MythId::new(), 0, vec![]);
        let raw_bytes = vec![0xFF, 0xFE, 0x00, 0x01]; // binary, not JSON
        let output = vec![WirePacket::new(WireType::Data, MythId::new(), 0, raw_bytes.clone())];

        let result = addon.on_output(&source, output).unwrap();
        assert_eq!(result[0].payload, raw_bytes);
    }

    #[test]
    fn heraldry_symbol_is_sigil() {
        let addon = HeraldryAddon::new();
        assert!(addon.heraldry_symbol().starts_with("Sigil:"));
    }

    #[test]
    fn targets_wildcard() {
        let addon = HeraldryAddon::new();
        assert_eq!(addon.target_plugin(), "*");
    }
}
