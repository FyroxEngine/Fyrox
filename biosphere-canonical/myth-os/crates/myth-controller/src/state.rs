use egui::Color32;

// ─── Wire type palette (matches myth-wire canonical 17 types) ─────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wire {
    Dat, Ctl, Aud, Nar, Tmp, Agt, Vis, Spa, Bhv, Soc, Enr, Idn, Evt, Ast, Met, Lgc, Res,
}

impl Wire {
    pub fn color(self) -> Color32 {
        match self {
            Wire::Dat => Color32::from_rgb(57,  255, 20),   // bio-green
            Wire::Ctl => Color32::from_rgb(148, 163, 184),  // slate
            Wire::Aud => Color32::from_rgb(249, 115, 22),   // ember
            Wire::Nar => Color32::from_rgb(139, 92,  246),  // violet
            Wire::Tmp => Color32::from_rgb(255, 45,  181),  // hot-pink
            Wire::Agt => Color32::from_rgb(99,  102, 241),  // indigo
            Wire::Vis => Color32::from_rgb(93,  202, 165),  // aqua
            Wire::Spa => Color32::from_rgb(0,   191, 255),  // sky-blue
            Wire::Bhv => Color32::from_rgb(176, 107, 255),  // purple
            Wire::Soc => Color32::from_rgb(200, 168, 96),   // gold-warm
            Wire::Enr => Color32::from_rgb(251, 113, 133),  // rose
            Wire::Idn => Color32::from_rgb(251, 191, 36),   // gold
            Wire::Evt => Color32::from_rgb(239, 68,  68),   // red
            Wire::Ast => Color32::from_rgb(245, 158, 11),   // amber
            Wire::Met => Color32::from_rgb(168, 162, 158),  // stone
            Wire::Lgc => Color32::from_rgb(20,  184, 166),  // teal
            Wire::Res => Color32::from_rgb(192, 132, 252),  // amethyst (mythos)
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Wire::Dat => "DAT", Wire::Ctl => "CTL", Wire::Aud => "AUD",
            Wire::Nar => "NAR", Wire::Tmp => "TMP", Wire::Agt => "AGT",
            Wire::Vis => "VIS", Wire::Spa => "SPA", Wire::Bhv => "BHV",
            Wire::Soc => "SOC", Wire::Enr => "ENR", Wire::Idn => "IDN",
            Wire::Evt => "EVT", Wire::Ast => "AST", Wire::Met => "MET",
            Wire::Lgc => "LGC", Wire::Res => "RES",
        }
    }
}

// ─── Channel strip state ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ChannelState {
    pub index:      usize,
    pub name:       &'static str,
    pub tag:        &'static str,
    pub wire:       Wire,
    pub dot:        Color32,

    // Controls
    pub macro_val:  f32,
    pub sub:        [f32; 4],
    pub sub_labels: [&'static str; 4],

    // Pads
    pub mute:  bool,
    pub solo:  bool,
    pub armed: bool,

    // Post-fader level (0–1)
    pub fader: f32,
}

impl ChannelState {
    pub fn meter_level(&self) -> f32 {
        self.fader * 0.6 + (self.sub.iter().sum::<f32>() / 4.0) * 0.4
    }
}

// ─── 16 canonical modules ─────────────────────────────────────────────────────
//     Source of truth: quantum-modules-complete-registry.txt

pub fn default_channels() -> Vec<ChannelState> {
    #[allow(clippy::type_complexity)]
    let defs: &[(&str, &str, Wire, (u8,u8,u8), [&str; 4])] = &[
        ("TERRAIN",      "TRR", Wire::Spa, (30,  140, 255), ["HGT",  "ERO",  "FOLD", "SEED"]),
        ("ENVIRONMENT",  "ENV", Wire::Spa, (128, 80,  224), ["CLMT", "WTHR", "BOME", "TIDE"]),
        ("ARCHITECT",    "ARC", Wire::Spa, (100, 180, 255), ["GRID", "RULE", "MATL", "SCAL"]),
        ("LIGHTING",     "LGT", Wire::Vis, (224, 216, 255), ["SPEC", "SHAD", "VOL",  "LOD" ]),
        ("MODELING",     "MDL", Wire::Vis, (244, 192, 37),  ["SOUL", "FORM", "ANIM", "LORE"]),
        ("CHOREOGRAPHY", "CHO", Wire::Bhv, (220, 60,  120), ["NODE", "FLOW", "EXEC", "CORD"]),
        ("BEHAVIOR",     "BHV", Wire::Bhv, (144, 48,  208), ["PERC", "DCSN", "EXEC", "ADPT"]),
        ("SOCIETY",      "SOC", Wire::Soc, (200, 168, 96),  ["AUTH", "RANK", "LAW",  "TRBN"]),
        ("SEQUENCER",    "SEQ", Wire::Tmp, (176, 128, 48),  ["REC",  "IDX",  "NAV",  "ANLZ"]),
        ("STORY",        "STR", Wire::Nar, (140, 80,  255), ["OBSV", "GEN",  "PERF", "XPRT"]),
        ("MEMORY",       "MEM", Wire::Nar, (0,   192, 96),  ["INSC", "CLSF", "RETR", "PUB" ]),
        ("SOUND",        "SND", Wire::Aud, (220, 140, 30),  ["SIG",  "COMP", "PERF", "XPRT"]),
        ("LOGIC",        "LGC", Wire::Lgc, (32,  200, 208), ["PROP", "VALD", "ROUT", "AUDT"]),
        ("SIMULATION",   "SIM", Wire::Dat, (48,  224, 96),  ["INIT", "TICK", "HAZ",  "SNAP"]),
        ("FORGE",        "FRG", Wire::Ast, (255, 100, 0),   ["GEN",  "INTG", "TEST", "DLVR"]),
        ("NETWORK",      "NET", Wire::Evt, (220, 220, 220), ["PROT", "XLAT", "ROUT", "FED" ]),
    ];

    defs.iter().enumerate().map(|(i, (name, tag, wire, dot, sub_labels))| {
        let seed = i as f32;
        ChannelState {
            index: i,
            name,
            tag,
            wire: *wire,
            dot: Color32::from_rgb(dot.0, dot.1, dot.2),
            macro_val:  0.5 + (seed * 0.91).sin() * 0.3,
            sub: [
                (0.4 + (seed * 0.71).sin() * 0.35).clamp(0.0, 1.0),
                (0.5 + (seed * 1.13).cos() * 0.30).clamp(0.0, 1.0),
                (0.3 + (seed * 0.57).sin() * 0.40).clamp(0.0, 1.0),
                (0.6 + (seed * 0.83).cos() * 0.25).clamp(0.0, 1.0),
            ],
            sub_labels: *sub_labels,
            mute:  false,
            solo:  false,
            armed: i < 4,
            fader: 0.72 + (seed * 0.37).sin() * 0.15,
        }
    }).collect()
}

// ─── Master section ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MasterState {
    pub crossfader: f32,
    pub master_vol: f32,
    pub bpm:        f32,
    pub epoch:      u32,
    pub era:        &'static str,
    pub frequency:  f32,
    pub playing:    bool,
    pub looping:    bool,
}

impl Default for MasterState {
    fn default() -> Self {
        Self {
            crossfader: 0.5,
            master_vol: 0.80,
            bpm:        173.0,
            epoch:      300,
            era:        "OCEANIC",
            frequency:  339.6,
            playing:    false,
            looping:    false,
        }
    }
}

// ─── Top-level controller state ───────────────────────────────────────────────

pub struct ControllerState {
    pub channels:      Vec<ChannelState>,
    pub master:        MasterState,
    pub active_module: usize,
    /// Simulated time counter for any live animations.
    pub tick:          f64,
}

impl Default for ControllerState {
    fn default() -> Self {
        Self {
            channels:      default_channels(),
            master:        MasterState::default(),
            active_module: 0,
            tick:          0.0,
        }
    }
}
