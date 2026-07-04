use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy_egui::egui;

use mandelbulb::{
    field::generate as gen_field,
    output::write_and_emit as field_write,
    params::{AtomManifest, FormulaSlot, KernelParams, MandelbulbParams, WorldScope},
};
use void_sculptor::{
    params::{ExtractionMode, SculptorConfig, SculptorInput},
    sculpt,
};

use crate::{orbit::Orbit, Mode, ViewerState};

// ── Shared pipeline state (main thread ↔ worker thread) ──────────────────────

#[derive(Default)]
pub struct Shared {
    pub progress: String,
    pub result:   Option<Result<PipelineOutput, String>>,
}

pub struct PipelineOutput {
    pub raw_path:   PathBuf,
    pub obj_path:   PathBuf,
    pub resolution: u32,
}

// ── Resource ─────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct GenState {
    pub world_id:     String,
    pub resolution:   u32,
    pub power:        f32,
    pub max_iter:     u32,
    pub bailout:      f32,
    pub smooth_passes: u32,
    pub output_dir:   String,

    pub shared:  Arc<Mutex<Shared>>,
    pub running: bool,
    pub msg:     String,
}

impl Default for GenState {
    fn default() -> Self {
        Self {
            world_id:     "test-world".into(),
            resolution:   64,
            power:        8.0,
            max_iter:     12,
            bailout:      2.0,
            smooth_passes: 1,
            output_dir:   "viewer_output".into(),
            shared:  Arc::new(Mutex::new(Shared::default())),
            running: false,
            msg:     String::new(),
        }
    }
}

// ── UI panel ─────────────────────────────────────────────────────────────────

/// Draw the Generate side panel. Returns true if the user clicked Run.
pub fn panel(ui: &mut egui::Ui, state: &mut GenState) -> bool {
    ui.label("World ID:");
    ui.text_edit_singleline(&mut state.world_id);

    ui.add_space(4.0);
    ui.label("Resolution (voxels/axis):");
    ui.add(egui::Slider::new(&mut state.resolution, 16..=256).text("³"));
    let voxels = state.resolution as u64 * state.resolution as u64 * state.resolution as u64;
    ui.label(
        egui::RichText::new(format!("{:.1}M voxels", voxels as f64 / 1_000_000.0))
            .weak()
            .small(),
    );

    ui.add_space(4.0);
    ui.separator();
    ui.label("Mandelbulb parameters:");

    ui.add(egui::Slider::new(&mut state.power, 1.0..=16.0).text("Power"));
    ui.add(egui::Slider::new(&mut state.max_iter, 4..=30).text("Iterations"));
    ui.add(egui::Slider::new(&mut state.bailout, 0.5..=100.0).text("Bailout").logarithmic(true));

    ui.add_space(4.0);
    ui.separator();
    ui.label("Sculptor:");
    ui.add(egui::Slider::new(&mut state.smooth_passes, 0..=5).text("Smooth passes"));

    ui.add_space(4.0);
    ui.separator();
    ui.label("Output dir:");
    ui.text_edit_singleline(&mut state.output_dir);
    ui.label(
        egui::RichText::new(format!("→ {}/{}/", state.output_dir, state.world_id))
            .weak()
            .small(),
    );

    ui.add_space(8.0);

    if state.running {
        ui.horizontal(|ui| {
            ui.spinner();
            if !state.msg.is_empty() {
                ui.label(&state.msg.clone());
            }
        });
        false
    } else {
        ui.add_enabled(!state.world_id.is_empty(), egui::Button::new("▶  Run pipeline"))
            .clicked()
    }
}

// ── Pipeline launch ───────────────────────────────────────────────────────────

pub fn launch(state: &mut GenState) {
    let shared       = state.shared.clone();
    let world_id     = state.world_id.clone();
    let resolution   = state.resolution;
    let power        = state.power;
    let max_iter     = state.max_iter;
    let bailout      = state.bailout;
    let smooth_passes = state.smooth_passes;
    let output_dir   = PathBuf::from(&state.output_dir).join(&world_id);

    state.running = true;
    state.msg     = "Starting…".into();

    std::thread::spawn(move || {
        let progress = |msg: &str| {
            if let Ok(mut lock) = shared.lock() {
                lock.progress = msg.to_string();
            }
        };
        let finish = |r: Result<PipelineOutput, String>| {
            if let Ok(mut lock) = shared.lock() {
                lock.result   = Some(r);
                lock.progress = String::new();
            }
        };

        if let Err(e) = std::fs::create_dir_all(&output_dir) {
            finish(Err(format!("Cannot create output dir: {e}")));
            return;
        }

        // ── Build KernelParams directly (bypass ATOM system for viewer) ──────
        let params = KernelParams {
            formula_chain: vec![FormulaSlot::default()],
            mandelbulb: MandelbulbParams {
                power,
                max_iterations: max_iter,
                bailout,
                julia_offset: Vec3::ZERO,
            },
            mandelbox:     None,
            material_seed: 0x1F2E_3D4C,
            resonance_mod: 0.0,
            resonance_hz:  440.0,
            scope: WorldScope {
                world_id:     world_id.clone(),
                resonance_hz: 440.0,
                resolution,
            },
            atom_manifest: viewer_manifest(),
        };

        // ── Genesis (mandelbulb) ──────────────────────────────────────────────
        progress(&format!("Genesis: generating {}³ scalar field…", resolution));
        let field = gen_field(&params);

        progress("Genesis: writing .raw to disk…");
        match field_write(&field, &params, &output_dir) {
            Err(e) => { finish(Err(e.to_string())); return; }
            Ok(_)  => {}
        }
        let raw_path = output_dir.join(format!("{world_id}.raw"));

        // ── Void Sculptor ─────────────────────────────────────────────────────
        progress("Sculptor: extracting iso-surface…");
        let input = SculptorInput::new(
            world_id.clone(),
            raw_path.clone(),
            [resolution, resolution, resolution],
            field.iso_threshold,
            params.clone(),
            params.atom_manifest.clone(),
            params.material_seed,
            params.resonance_mod,
        )
        .with_config(SculptorConfig {
            mode:               ExtractionMode::PropagatingContours,
            auto_fallback:      true,
            min_triangle_count: 8,
            smooth_passes,
            min_island_size:    4,
        });

        match sculpt(&input, &output_dir) {
            Ok(_)  => {
                let obj_path = output_dir.join(format!("{world_id}.obj"));
                finish(Ok(PipelineOutput { raw_path, obj_path, resolution }));
            }
            Err(e) => finish(Err(e.to_string())),
        }
    });
}

// ── System: poll worker, auto-load results ────────────────────────────────────

pub fn check_status(
    mut gen:    ResMut<GenState>,
    mut viewer: ResMut<ViewerState>,
    mut orbit:  ResMut<Orbit>,
) {
    // Mirror progress message every frame while running
    if gen.running {
        let progress = gen.shared.try_lock().ok().map(|l| l.progress.clone());
        if let Some(p) = progress {
            if !p.is_empty() {
                gen.msg = p;
            }
        }
    }

    // Check for a completed result
    let result = {
        let Ok(mut lock) = gen.shared.try_lock() else { return };
        lock.result.take()
    };

    if let Some(outcome) = result {
        gen.running = false;
        match outcome {
            Ok(output) => {
                gen.msg = String::new();
                // Auto-load raw field
                viewer.raw_path    = Some(output.raw_path);
                viewer.field_res   = output.resolution;
                viewer.raw_pending = true;
                // Auto-load mesh
                viewer.obj_path    = Some(output.obj_path);
                viewer.obj_pending = true;
                // Switch to mesh view + reset camera
                viewer.mode    = Mode::Mesh;
                viewer.status  = "Pipeline complete.".into();
                orbit.radius   = 3.5;
                orbit.yaw      = 0.4;
                orbit.pitch    = 0.3;
            }
            Err(e) => {
                gen.msg       = format!("Error: {e}");
                viewer.status = format!("Pipeline error: {e}");
            }
        }
    }
}

// ── Minimal manifest (viewer bypasses ATOM routing) ──────────────────────────

fn viewer_manifest() -> AtomManifest {
    AtomManifest {
        void_phase:  ["SYS_01".into(), "SYS_02".into(), "SYS_03".into(), "SYS_04".into()],
        spark_phase: ["NAV_01".into(), "NAV_02".into(), "NAV_03".into(), "NAV_04".into()],
        law_phase:   ["COG_01".into(), "COG_02".into(), "COG_03".into(), "COG_04".into()],
        bloom_phase: ["MED_01".into(), "MED_02".into(), "MED_03".into(), "MED_04".into()],
    }
}
