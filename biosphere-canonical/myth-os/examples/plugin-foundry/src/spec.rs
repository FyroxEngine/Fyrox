/// The data model the Foundry UI populates before forging a plugin.
///
/// When the user hits "Forge", this spec is serialized as JSON and emitted
/// as a DAT WirePacket. A code-generation tool downstream can consume it
/// to scaffold a real Rust crate.
use serde::{Deserialize, Serialize};

/// One wire type entry — mirrors WireType but as plain string for portability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireEntry {
    /// e.g. "SPA", "ENR", "AUD", "DAT" — canonical WireType tag
    pub tag: String,
    /// Human note: what this signal means for this plugin
    pub note: String,
}

impl WireEntry {
    pub fn new(tag: impl Into<String>, note: impl Into<String>) -> Self {
        Self { tag: tag.into(), note: note.into() }
    }
}

/// An ATOM slot declaration for this plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomEntry {
    /// ATOM identifier, e.g. "FILTER", "TRANSFORM", "SAMPLE"
    pub atom_id: String,
    /// Human label shown in the node graph
    pub label: String,
    /// Brief description of what this ATOM does in this plugin's context
    pub description: String,
}

impl AtomEntry {
    pub fn new(
        atom_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            atom_id: atom_id.into(),
            label: label.into(),
            description: description.into(),
        }
    }
}

/// An asset to bundle with the generated plugin (icon, sample, shader, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub name: String,
    pub path: String,
    pub media_type: String,  // mirrors MediaType tag: "Image", "Audio", etc.
}

impl AssetEntry {
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self { name: name.into(), path: path.into(), media_type: media_type.into() }
    }
}

/// Plugin type — affects Heraldry rank and how the Foundry scaffolds the output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PluginKind {
    /// Extends a core instrument. Gets a Glyph inheriting a parent Crest.
    #[default]
    Plugin,
    /// Cross-instrument. Gets an independent Sigil.
    Addon,
}

/// The complete specification for a plugin to be forged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSpec {
    // ── IDENTITY panel ─────────────────────────────────────────────────────
    pub kind: PluginKind,
    /// Snake-case crate name, e.g. "myth-erosion", "user-tide-sim"
    pub crate_name: String,
    /// Human display name
    pub display_name: String,
    /// What this plugin does — shown in the UI and emitted into lib.rs doc comment
    pub description: String,
    /// Semantic version
    pub version: (u32, u32, u32),

    // ── HERALDRY panel ─────────────────────────────────────────────────────
    /// For Plugin: "Glyph:<Symbol>↑<ParentCrest>"
    /// For Addon:  "Sigil:<Symbol>"
    pub heraldry_symbol: String,
    /// For Plugin kind: which core instrument Crest this inherits from
    pub parent_crest: String,
    /// The Glyph/Sigil symbol name itself, e.g. "Erosion", "TideWatch"
    pub symbol_name: String,

    // ── WIRE CONFIG panel ──────────────────────────────────────────────────
    pub wire_in: Vec<WireEntry>,
    pub wire_out: Vec<WireEntry>,

    // ── ATOMS panel ────────────────────────────────────────────────────────
    pub atoms: Vec<AtomEntry>,

    // ── ASSETS panel ───────────────────────────────────────────────────────
    pub assets: Vec<AssetEntry>,

    // ── OUTPUT panel ───────────────────────────────────────────────────────
    /// Where to scaffold the generated crate (relative to workspace root)
    pub output_path: String,

    // ── Metadata ───────────────────────────────────────────────────────────
    pub forged_at: i64,
    pub spec_version: String,
}

impl PluginSpec {
    pub fn new_plugin(crate_name: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            kind:           PluginKind::Plugin,
            crate_name:     crate_name.into(),
            display_name:   display_name.into(),
            description:    String::new(),
            version:        (0, 1, 0),
            heraldry_symbol: String::new(),
            parent_crest:   String::new(),
            symbol_name:    String::new(),
            wire_in:        Vec::new(),
            wire_out:       Vec::new(),
            atoms:          Vec::new(),
            assets:         Vec::new(),
            output_path:    String::from("crates/"),
            forged_at:      chrono::Utc::now().timestamp(),
            spec_version:   "foundry-v1.0".into(),
        }
    }

    pub fn new_addon(crate_name: impl Into<String>, display_name: impl Into<String>) -> Self {
        let mut s = Self::new_plugin(crate_name, display_name);
        s.kind = PluginKind::Addon;
        s.output_path = "crates/".into();
        s
    }

    /// Assemble the heraldry_symbol string from its components.
    /// Call this before forging so the emitted spec is consistent.
    pub fn build_heraldry(&mut self) {
        self.heraldry_symbol = match self.kind {
            PluginKind::Plugin => {
                if self.parent_crest.is_empty() || self.symbol_name.is_empty() {
                    String::new()
                } else {
                    format!("Glyph:{}↑{}", self.symbol_name, self.parent_crest)
                }
            }
            PluginKind::Addon => {
                if self.symbol_name.is_empty() {
                    String::new()
                } else {
                    format!("Sigil:{}", self.symbol_name)
                }
            }
        };
    }

    /// Quick validation — returns a list of error strings if anything is missing.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.crate_name.is_empty()   { errors.push("Crate name is required".into()); }
        if self.display_name.is_empty() { errors.push("Display name is required".into()); }
        if self.heraldry_symbol.is_empty() { errors.push("Heraldry symbol is required — fill HERALDRY panel and build".into()); }
        if self.wire_in.is_empty() && self.wire_out.is_empty() {
            errors.push("At least one wire_in or wire_out type is required".into());
        }
        errors
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_spec_builds_heraldry() {
        let mut spec = PluginSpec::new_plugin("myth-erosion", "Erosion");
        spec.symbol_name = "Erosion".into();
        spec.parent_crest = "Atlas".into();
        spec.build_heraldry();
        assert_eq!(spec.heraldry_symbol, "Glyph:Erosion↑Atlas");
    }

    #[test]
    fn addon_spec_builds_sigil() {
        let mut spec = PluginSpec::new_addon("myth-tide-watch", "Tide Watch");
        spec.symbol_name = "TideWatch".into();
        spec.build_heraldry();
        assert_eq!(spec.heraldry_symbol, "Sigil:TideWatch");
    }

    #[test]
    fn spec_validates_required_fields() {
        let spec = PluginSpec::new_plugin("", "");
        let errs = spec.validate();
        assert!(!errs.is_empty());
    }

    #[test]
    fn spec_serializes_to_json() {
        let mut spec = PluginSpec::new_plugin("test-plugin", "Test Plugin");
        spec.wire_in.push(WireEntry::new("DAT", "input data"));
        spec.wire_out.push(WireEntry::new("DAT", "output data"));
        spec.symbol_name = "Test".into();
        spec.parent_crest = "Core".into();
        spec.build_heraldry();
        let json = spec.to_json().unwrap();
        assert!(json.contains("test-plugin"));
        assert!(json.contains("Glyph:Test↑Core"));
    }
}
