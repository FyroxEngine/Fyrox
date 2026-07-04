use crate::SubmitMeta;
use uuid::Uuid;

/// Proof that a plugin passed Registry certification.
/// Stamped with a Heraldry Glyph and returned to Plugin Foundry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginCertificate {
    pub plugin_id:  String,
    /// Heraldry glyph: "Glyph:<Symbol>↑<ParentCrest>"
    pub heraldry:   String,
    pub hash:       String,
    pub issued_at:  String,
    pub author:     String,
    pub name:       String,
    pub version:    String,
}

/// Issue a certificate for a plugin that passed validation.
pub fn certify(meta: &SubmitMeta, hash: &str) -> PluginCertificate {
    let now = chrono::Utc::now().to_rfc3339();
    let heraldry = format!("Glyph:{}↑{}", meta.glyph_symbol, meta.parent_crest);

    PluginCertificate {
        plugin_id:  Uuid::new_v4().to_string(),
        heraldry,
        hash:       hash.to_string(),
        issued_at:  now,
        author:     meta.author.clone(),
        name:       meta.name.clone(),
        version:    meta.version.clone(),
    }
}
