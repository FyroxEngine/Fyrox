use mythos::quantum_module::QuantumModule;
use std::path::Path;
use tracing::{info, warn};

/// Scan a directory for *.json files and load each as a QuantumModule.
/// Files that fail validation are skipped with a warning — the engine keeps booting.
pub fn scan_modules(dir: impl AsRef<Path>) -> Vec<QuantumModule> {
    let dir = dir.as_ref();
    let mut modules = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            warn!(dir = %dir.display(), error = %err, "Could not read module directory");
            return modules;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        match QuantumModule::from_file(&path) {
            Ok(module) => {
                info!(
                    id   = %module.id,
                    name = %module.name,
                    dept = ?module.department,
                    "GENESIS: Module manifested"
                );
                modules.push(module);
            }
            Err(err) => {
                warn!(
                    path  = %path.display(),
                    error = %err,
                    "GENESIS WARNING: Failed to parse module — skipping"
                );
            }
        }
    }

    modules.sort_by(|a, b| a.id.cmp(&b.id));
    info!(count = modules.len(), "Module scan complete");
    modules
}

/// Bevy resource holding all loaded QuantumModules.
#[derive(bevy::prelude::Resource)]
pub struct ModuleRegistry(pub Vec<QuantumModule>);

impl ModuleRegistry {
    pub fn load_from(dir: impl AsRef<std::path::Path>) -> Self {
        Self(scan_modules(dir))
    }

    pub fn get(&self, id: &str) -> Option<&QuantumModule> {
        self.0.iter().find(|m| m.id == id)
    }

    pub fn by_department(
        &self,
        dept: mythos::quantum_module::Department,
    ) -> impl Iterator<Item = &QuantumModule> {
        self.0.iter().filter(move |m| m.department == dept)
    }
}

// ── Module manifest seeding ───────────────────────────────────────────────────
// Writes the 16 canonical QuantumModule JSON files to `assets/modules/` if they
// do not already exist.  Safe to call on every boot — existing files are never
// overwritten.

/// Seed data for one module.  All fields are `'static` so this can live in a
/// const array without heap allocation.
struct Seed {
    id:       &'static str,
    name:     &'static str,
    crest:    &'static str,
    color:    &'static str,  // hex e.g. "#1e8cff"
    dept:     &'static str,  // "Structure"|"Entities"|"Atmosphere"|"Dynamics"
    desc:     &'static str,
    status:   &'static str,  // "built"|"in-progress"|"planned"
    wire:     &'static str,  // primary_wire_out
    ch:       u8,            // Traktor channel 1–4
    /// (midi_cc, parameter_name, scale_min, scale_max)
    bindings: &'static [(u8, &'static str, f32, f32)],
}

/// The 16 canonical Quantum Genesis modules, ordered GEN-01 … GEN-16.
static MODULES: &[Seed] = &[
    // ── Dept I: WORLD CONSTRUCTION (Structure / Ch1) ─────────────────────────
    Seed {
        id: "GEN-01", name: "Terrain", crest: "◈", color: "#1e8cff",
        dept: "Structure",
        desc: "Procedural heightmap generation via seeded fractal noise. \
               Controls world elevation, biome boundaries, and surface topology.",
        status: "built", wire: "SPA", ch: 1,
        bindings: &[
            (7,  "height_scale",  2.0, 20.0),
            (16, "noise_seed",    0.0, 255.0),
            (17, "octaves",       1.0, 8.0),
        ],
    },
    Seed {
        id: "GEN-02", name: "Environment", crest: "◈", color: "#8050e0",
        dept: "Structure",
        desc: "Atmosphere, weather, sky colour and fog density. \
               Driven by time-of-day and biome state.",
        status: "built", wire: "SPA", ch: 1,
        bindings: &[
            (18, "fog_density",   0.0, 1.0),
            (19, "sky_luminance", 0.2, 2.0),
        ],
    },
    Seed {
        id: "GEN-03", name: "Architect", crest: "◈", color: "#64b4ff",
        dept: "Structure",
        desc: "Procedural structure placement — roads, ruins, settlements. \
               Uses container manifests to stamp geometry.",
        status: "built", wire: "SPA", ch: 1,
        bindings: &[
            (20, "density",  0.0, 1.0),
            (21, "variance", 0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-04", name: "Lighting", crest: "◈", color: "#e0d8ff",
        dept: "Structure",
        desc: "Dynamic directional light, ambient colour, and shadow parameters. \
               Responds to environment state for day/night cycles.",
        status: "built", wire: "VIS", ch: 1,
        bindings: &[
            (22, "illuminance", 1000.0, 100_000.0),
            (23, "sun_angle",   -1.57,  1.57),
        ],
    },

    // ── Dept II: ENTITY SYSTEMS (Entities / Ch2) ─────────────────────────────
    Seed {
        id: "GEN-05", name: "Modeling", crest: "◈", color: "#f4c025",
        dept: "Entities",
        desc: "Mesh generation and LOD management for Quill Actors. \
               Resolves B-DNA phenotype to visual form.",
        status: "built", wire: "VIS", ch: 2,
        bindings: &[
            (24, "lod_bias",    0.5, 2.0),
            (25, "poly_budget", 512.0, 8192.0),
        ],
    },
    Seed {
        id: "GEN-06", name: "Choreography", crest: "◈", color: "#dc3c78",
        dept: "Entities",
        desc: "Animation state machine and IK solver for entity movement. \
               Blend trees driven by Instinct outputs.",
        status: "built", wire: "BHV", ch: 2,
        bindings: &[
            (26, "blend_speed", 0.5, 4.0),
            (27, "ik_weight",   0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-07", name: "Behavior", crest: "◈", color: "#9030d0",
        dept: "Entities",
        desc: "High-level behaviour tree evaluator — goals, plans, reactive layers. \
               Reads from Instinct drives and Social bond graph.",
        status: "in-progress", wire: "BHV", ch: 2,
        bindings: &[
            (28, "aggression_bias", 0.0, 1.0),
            (29, "curiosity_bias",  0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-08", name: "Society", crest: "◈", color: "#c8a860",
        dept: "Entities",
        desc: "Reputation graph, faction alignment, and social contract enforcement. \
               Wire type SOC carries bond delta packets.",
        status: "built", wire: "SOC", ch: 2,
        bindings: &[
            (30, "cohesion",   0.0, 1.0),
            (31, "reputation", -1.0, 1.0),
        ],
    },

    // ── Dept III: NARRATIVE SYSTEMS (Atmosphere / Ch3) ───────────────────────
    Seed {
        id: "GEN-09", name: "Sequencer", crest: "◈", color: "#b08030",
        dept: "Atmosphere",
        desc: "Timeline sequencer for scripted narrative events and scene beats. \
               Emits TMP ticks to downstream modules.",
        status: "built", wire: "TMP", ch: 3,
        bindings: &[
            (46, "tempo",    60.0, 240.0),
            (47, "swing",     0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-10", name: "Story", crest: "◈", color: "#8c50ff",
        dept: "Atmosphere",
        desc: "Narrative arc manager — chapter state, branch selection, and \
               dramatic tension tracking across the Genesis container.",
        status: "built", wire: "NAR", ch: 3,
        bindings: &[
            (48, "tension",   0.0, 1.0),
            (49, "pacing",    0.2, 2.0),
        ],
    },
    Seed {
        id: "GEN-11", name: "Memory", crest: "◈", color: "#00c060",
        dept: "Atmosphere",
        desc: "Entity episodic memory store — experience capsules, recall decay, \
               and emotional tagging of past events.",
        status: "built", wire: "NAR", ch: 3,
        bindings: &[
            (50, "recall_decay",   0.0, 1.0),
            (51, "vividness_bias", 0.0, 2.0),
        ],
    },
    Seed {
        id: "GEN-12", name: "Sound", crest: "◈", color: "#dc8c1e",
        dept: "Atmosphere",
        desc: "Positional audio engine — ambient layers, event triggers, \
               and music state machine keyed to narrative tension.",
        status: "built", wire: "AUD", ch: 3,
        bindings: &[
            (52, "master_volume", 0.0, 1.0),
            (53, "reverb_mix",    0.0, 1.0),
        ],
    },

    // ── Dept IV: PIPELINE SYSTEMS (Dynamics / Ch4) ───────────────────────────
    Seed {
        id: "GEN-13", name: "Logic", crest: "◈", color: "#20c8d0",
        dept: "Dynamics",
        desc: "Boolean gate evaluator and rule engine. Reads LGC wire packets \
               and propagates truth-values through the condition graph.",
        status: "in-progress", wire: "LGC", ch: 4,
        bindings: &[
            (54, "gate_threshold", 0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-14", name: "Simulation", crest: "◈", color: "#30e060",
        dept: "Dynamics",
        desc: "Physics sub-step manager — rigid body, fluid, and cloth \
               simulation budget allocation per frame.",
        status: "in-progress", wire: "DAT", ch: 4,
        bindings: &[
            (55, "sim_budget",  0.5, 8.0),
            (56, "gravity_mul", 0.0, 2.0),
        ],
    },
    Seed {
        id: "GEN-15", name: "Forge", crest: "◈", color: "#ff6400",
        dept: "Dynamics",
        desc: "Asset pipeline — bakes, converts, and streams game-ready assets \
               from raw source capsules in the Vault.",
        status: "built", wire: "AST", ch: 4,
        bindings: &[
            (57, "batch_size",  1.0, 64.0),
            (58, "compression", 0.0, 1.0),
        ],
    },
    Seed {
        id: "GEN-16", name: "Network", crest: "◈", color: "#c8d8ff",
        dept: "Dynamics",
        desc: "Event bus and inter-process communication layer. Routes EVT packets \
               between Genesis, Library, and external services.",
        status: "built", wire: "EVT", ch: 4,
        bindings: &[
            (59, "tick_rate", 20.0, 128.0),
            (60, "jitter_ms",  0.0, 50.0),
        ],
    },
];

/// Write all 16 module JSON files to `dir` if they don't already exist.
/// Existing files are left untouched — user edits are preserved.
pub fn ensure_module_manifests(dir: impl AsRef<Path>) {
    let dir = dir.as_ref();

    if let Err(e) = std::fs::create_dir_all(dir) {
        warn!(dir = %dir.display(), error = %e, "Could not create modules directory");
        return;
    }

    let mut written = 0u32;
    for seed in MODULES {
        let path = dir.join(format!("{}.json", seed.id));
        if path.exists() {
            continue; // never overwrite hand-edited manifests
        }
        let json = build_json(seed);
        match std::fs::write(&path, json.as_bytes()) {
            Ok(_)  => { written += 1; }
            Err(e) => { warn!(path = %path.display(), error = %e, "Failed to write module manifest"); }
        }
    }

    if written > 0 {
        info!(count = written, dir = %dir.display(), "Module manifests seeded");
    }
}

fn build_json(s: &Seed) -> String {
    let bindings: Vec<String> = s.bindings.iter().map(|(cc, param, min, max)| {
        format!(
            concat!(
                "    {{\n",
                "      \"channel\": {ch},\n",
                "      \"control_type\": \"fader\",\n",
                "      \"midi_cc\": {cc},\n",
                "      \"parameter\": \"{param}\",\n",
                "      \"scale_min\": {min},\n",
                "      \"scale_max\": {max}\n",
                "    }}",
            ),
            ch    = s.ch,
            cc    = cc,
            param = param,
            min   = min,
            max   = max,
        )
    }).collect();

    format!(
        concat!(
            "{{\n",
            "  \"id\": \"{id}\",\n",
            "  \"name\": \"{name}\",\n",
            "  \"crest\": \"{crest}\",\n",
            "  \"color\": \"{color}\",\n",
            "  \"department\": \"{dept}\",\n",
            "  \"description\": \"{desc}\",\n",
            "  \"implementation_status\": \"{status}\",\n",
            "  \"primary_wire_out\": \"{wire}\",\n",
            "  \"lifecycle\": \"active\",\n",
            "  \"capacity\": 16,\n",
            "  \"assets\": {{\n",
            "    \"icon\": \"\",\n",
            "    \"preview_image\": \"\",\n",
            "    \"banner\": \"\",\n",
            "    \"crest_svg\": \"\",\n",
            "    \"splash_screen\": null,\n",
            "    \"audio_preview\": null,\n",
            "    \"video_preview\": null\n",
            "  }},\n",
            "  \"containers\": [],\n",
            "  \"traktor_map\": [\n",
            "{bindings}\n",
            "  ]\n",
            "}}",
        ),
        id       = s.id,
        name     = s.name,
        crest    = s.crest,
        color    = s.color,
        dept     = s.dept,
        desc     = s.desc,
        status   = s.status,
        wire     = s.wire,
        bindings = bindings.join(",\n"),
    )
}
