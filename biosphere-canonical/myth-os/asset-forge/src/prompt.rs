// Prompt builder — assembles the [TOKEN][TOKEN]… prompt string from a config.
// Used by ComfyUI / Midjourney / Stable Diffusion workflows.

use crate::config::AssetConfig;

/// Build the structured token prompt string.
pub fn build(cfg: &AssetConfig) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Primary type token (always present)
    parts.push(format!("[{}]", cfg.asset.asset_type.to_uppercase()));

    // Zone / biome
    if let Some(ref z) = cfg.asset.zone {
        parts.push(format!("[{}]", z.to_uppercase()));
    }

    // Structural function
    if let Some(ref f) = cfg.asset.function {
        parts.push(format!("[{}]", f.to_uppercase()));
    }

    // Style variant
    if let Some(ref v) = cfg.asset.variant {
        parts.push(format!("[{}]", v.to_uppercase()));
    }

    // Grid scale
    if let Some(ref s) = cfg.asset.scale {
        parts.push(format!("[{}]", s.to_uppercase()));
    }

    // Render — background, direction, angle, shader
    if let Some(ref bg) = cfg.render.background {
        parts.push(format!("[{}]", bg.to_uppercase()));
    }
    if let Some(ref dir) = cfg.render.direction {
        parts.push(format!("[{}]", dir.to_uppercase()));
    }
    if let Some(ref ang) = cfg.render.angle {
        parts.push(format!("[{}]", ang.to_uppercase()));
    }
    if let Some(ref sh) = cfg.render.shader {
        parts.push(format!("[{}]", sh.to_uppercase()));
    }

    // Character-specific tokens
    if let Some(ref ch) = cfg.character {
        if let Some(ref p) = ch.pose {
            parts.push(format!("[{}]", p.to_uppercase()));
        }
        if let Some(ref r) = ch.rig {
            parts.push(format!("[{}]", r.to_uppercase()));
        }
        if let Some(ref fac) = ch.faction {
            parts.push(format!("[{}]", fac.to_uppercase()));
        }
        if let Some(ref role) = ch.role {
            parts.push(format!("[{}]", role.to_uppercase()));
        }
        if let Some(ref lod) = ch.lod {
            parts.push(format!("[LOD:{}]", lod.to_uppercase()));
        }
    }

    // Quantum module
    if let Some(ref qm) = cfg.meta.quantum_module {
        parts.push(format!("[QM:{}]", qm));
    }

    // Resonance
    if let Some(hz) = cfg.meta.resonance_hz {
        parts.push(format!("[RES:{hz}Hz]"));
    }

    // Tags appended verbatim
    if let Some(ref tags) = cfg.meta.tags {
        for tag in tags {
            parts.push(format!("[{tag}]"));
        }
    }

    parts.join("")
}

/// Build a shorter filename-safe token string (no brackets, underscores).
/// Used as the stem for output files: CAVE_ENTRANCE_ORGANIC_2X2_ISW_ISOMETRIC_PBR
pub fn build_stem(cfg: &AssetConfig, letter: Option<&str>) -> String {
    let mut parts: Vec<&str> = Vec::new();

    parts.push(cfg.asset.asset_type.as_str());

    if let Some(ref z) = cfg.asset.zone     { parts.push(z); }
    if let Some(ref f) = cfg.asset.function { parts.push(f); }
    if let Some(ref v) = cfg.asset.variant  { parts.push(v); }
    if let Some(ref s) = cfg.asset.scale    { parts.push(s); }

    if let Some(ref ch) = cfg.character {
        if let Some(ref p) = ch.pose { parts.push(p); }
        if let Some(ref r) = ch.rig  { parts.push(r); }
    }

    if let Some(ref dir) = cfg.render.direction { parts.push(dir); }
    if let Some(ref ang) = cfg.render.angle     { parts.push(ang); }
    if let Some(ref sh)  = cfg.render.shader    { parts.push(sh);  }

    let mut stem = parts.join("_").to_uppercase();

    if let Some(l) = letter {
        stem.push('_');
        stem.push_str(&l.to_uppercase());
    }

    stem
}

/// Derive the directory path relative to an output root:
///   <domain>/<zone>/<type>/<variant>/
pub fn build_dir(cfg: &AssetConfig) -> std::path::PathBuf {
    let domain = cfg.asset.resolved_domain();
    let zone   = cfg.asset.zone.as_deref().unwrap_or("MISC").to_uppercase();
    let atype  = cfg.asset.asset_type.to_uppercase();
    let var    = cfg.asset.variant.as_deref().unwrap_or("BASE").to_uppercase();

    std::path::PathBuf::from(domain).join(zone).join(atype).join(var)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AssetConfig, AssetInfo, RenderInfo, MetaInfo, Sockets};

    fn sample_cfg() -> AssetConfig {
        AssetConfig {
            asset: AssetInfo {
                asset_type: "CAVE_ENTRANCE".into(),
                domain:     None,
                zone:       Some("CAVE".into()),
                function:   Some("ENTRANCE".into()),
                variant:    Some("ORGANIC".into()),
                scale:      Some("2X2".into()),
                letter:     None,
                variants:   None,
            },
            sockets: Some(Sockets {
                north: Some("PASSAGE_MD".into()),
                south: Some("CLOSED".into()),
                east:  None, west: None, up: None, down: None,
            }),
            render: RenderInfo {
                background: Some("TRANSPARENT".into()),
                direction:  Some("ISW".into()),
                angle:      Some("ISOMETRIC".into()),
                shader:     Some("PBR".into()),
            },
            meta: MetaInfo {
                quantum_module: Some("Forge".into()),
                resonance_hz:   Some(174.6),
                tags:           Some(vec!["tileable".into(), "entrance".into()]),
            },
            character: None,
        }
    }

    #[test]
    fn prompt_contains_all_tokens() {
        let cfg = sample_cfg();
        let p = build(&cfg);
        assert!(p.contains("[CAVE_ENTRANCE]"));
        assert!(p.contains("[CAVE]"));
        assert!(p.contains("[ORGANIC]"));
        assert!(p.contains("[ISW]"));
        assert!(p.contains("[PBR]"));
        assert!(p.contains("[RES:174.6Hz]"));
    }

    #[test]
    fn stem_uppercase_underscored() {
        let cfg = sample_cfg();
        let s = build_stem(&cfg, Some("A"));
        assert!(s.starts_with("CAVE_ENTRANCE_CAVE_ENTRANCE_ORGANIC_2X2"));
        assert!(s.ends_with("_A"));
    }
}
