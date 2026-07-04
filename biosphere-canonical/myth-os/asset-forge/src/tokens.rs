// Token vocabulary — valid values for each config field.
// Used for validation warnings (not hard errors — unknown tokens are allowed
// but flagged so typos surface early).

pub const VALID_ZONES: &[&str] = &[
    "CAVE", "UNDERGROUND", "DEEP", "VOID", "FUNGAL",
    "SKY", "FLOATING", "LUMINARITE", "OCEAN", "VOLCANIC",
    "AIRSHIP", "MECHANICAL", "INDUSTRIAL",
    "ALIEN", "ORGANIC", "JUNGLE", "XYRONA",
    "DESERT", "ARCTIC", "RUINS", "NEXUS",
];

pub const VALID_FUNCTIONS: &[&str] = &[
    "ENTRANCE", "EXIT", "CORRIDOR", "PASSAGE", "TUNNEL",
    "CHAMBER", "HALL", "ROOM", "ANTECHAMBER",
    "HULL", "DECK", "ENGINE", "BRIDGE", "CABIN",
    "SPIRE", "TOWER", "ARCH", "BUTTRESS", "COLUMN",
    "PLATFORM", "BRIDGE_SPAN", "RAMP", "STAIR",
    "PORTAL", "GATE", "DOORWAY", "WINDOW",
    "DETAIL", "PROP", "ACCENT", "DEBRIS", "GROWTH",
    "ROOT", "CANOPY", "TRUNK", "BRANCH",
];

pub const VALID_VARIANTS: &[&str] = &[
    "ORGANIC", "ARMORED", "RUINED", "PRISTINE", "ANCIENT",
    "OVERGROWN", "CRYSTALLINE", "BURNT", "FLOODED", "FROZEN",
    "LUMINOUS", "CORRUPTED", "RESTORED", "WEATHERED",
    "MECHANICAL", "ARCANE", "NATURAL", "ALIEN",
];

pub const VALID_SCALES: &[&str] = &[
    "1X1", "2X1", "1X2", "2X2", "3X1", "1X3", "3X3",
    "4X4", "1X1X2", "2X2X2", "2X1X2",
];

pub const VALID_SOCKET_TYPES: &[&str] = &[
    "OPEN", "CLOSED",
    "PASSAGE_SM", "PASSAGE_MD", "PASSAGE_LG", "PASSAGE_XL",
    "ARCH_SM", "ARCH_MD", "ARCH_LG",
    "EXTERIOR_OPEN", "EXTERIOR_CLIFF", "EXTERIOR_WATER",
    "CAVE_CEILING", "CAVE_FLOOR", "CAVE_WALL",
    "PLATFORM_TOP", "PLATFORM_EDGE",
    "GROUND_NATURAL", "GROUND_STONE", "GROUND_METAL",
    "HULL_PANEL", "DECK_SURFACE",
    "WATER_SURFACE", "VOID_EDGE",
];

pub const VALID_DIRECTIONS: &[&str] = &[
    "ISW", "INE", "INW", "ISE",
    "FRONT", "BACK", "SIDE_L", "SIDE_R", "TOP", "BOTTOM",
];

pub const VALID_ANGLES: &[&str] = &[
    "ISOMETRIC", "PERSPECTIVE", "ORTHOGRAPHIC",
];

pub const VALID_SHADERS: &[&str] = &[
    "PBR", "UNLIT", "MATCAP", "TOON", "WIREFRAME",
];

pub const VALID_QUANTUM_MODULES: &[&str] = &[
    "Genesis", "Quill", "Core", "Order", "Vault", "Loom",
    "Forge", "Mythos", "Codex", "Atlas", "Composer", "Prism",
    "Architect", "Chronicle", "Animus", "Nexus", "Cipher", "Agora",
];

pub const VALID_POSES: &[&str] = &[
    "T_POSE", "A_POSE", "IDLE", "ACTION", "COMBAT", "SITTING",
    "CROUCHING", "FLYING", "SWIMMING",
];

pub const VALID_RIGS: &[&str] = &[
    "HUMANOID", "QUADRUPED", "SERPENTINE", "WINGED",
    "TENTACLED", "MECHANICAL", "INSECTOID", "AMORPHOUS",
];

pub const VALID_LODS: &[&str] = &["HIGH", "MID", "LOW"];

// ── Validation ────────────────────────────────────────────────────────────────

pub struct ValidationReport {
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_clean(&self) -> bool { self.warnings.is_empty() }
}

pub fn validate(cfg: &crate::config::AssetConfig) -> ValidationReport {
    let mut w: Vec<String> = Vec::new();

    check_token(&mut w, "zone",    cfg.asset.zone.as_deref(),     VALID_ZONES);
    check_token(&mut w, "function",cfg.asset.function.as_deref(), VALID_FUNCTIONS);
    check_token(&mut w, "variant", cfg.asset.variant.as_deref(),  VALID_VARIANTS);
    check_token(&mut w, "scale",   cfg.asset.scale.as_deref(),    VALID_SCALES);
    check_token(&mut w, "render.direction", cfg.render.direction.as_deref(), VALID_DIRECTIONS);
    check_token(&mut w, "render.angle",     cfg.render.angle.as_deref(),     VALID_ANGLES);
    check_token(&mut w, "render.shader",    cfg.render.shader.as_deref(),    VALID_SHADERS);
    check_token(&mut w, "meta.quantum_module", cfg.meta.quantum_module.as_deref(), VALID_QUANTUM_MODULES);

    if let Some(ref s) = cfg.sockets {
        check_socket(&mut w, "north", s.north.as_deref());
        check_socket(&mut w, "south", s.south.as_deref());
        check_socket(&mut w, "east",  s.east.as_deref());
        check_socket(&mut w, "west",  s.west.as_deref());
        check_socket(&mut w, "up",    s.up.as_deref());
        check_socket(&mut w, "down",  s.down.as_deref());
    }

    if let Some(ref ch) = cfg.character {
        check_token(&mut w, "character.pose", ch.pose.as_deref(), VALID_POSES);
        check_token(&mut w, "character.rig",  ch.rig.as_deref(),  VALID_RIGS);
        check_token(&mut w, "character.lod",  ch.lod.as_deref(),  VALID_LODS);
    }

    ValidationReport { warnings: w }
}

fn check_token(warnings: &mut Vec<String>, field: &str, value: Option<&str>, valid: &[&str]) {
    if let Some(v) = value {
        let upper = v.to_uppercase();
        if !valid.iter().any(|t| t.to_uppercase() == upper) {
            warnings.push(format!(
                "  ⚠  [{field}] unknown token '{v}' \
                 — not in vocabulary (allowed but may be a typo)"
            ));
        }
    }
}

fn check_socket(warnings: &mut Vec<String>, face: &str, value: Option<&str>) {
    check_token(warnings, &format!("socket.{face}"), value, VALID_SOCKET_TYPES);
}
