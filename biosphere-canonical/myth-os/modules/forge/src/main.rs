// forge standalone test binary (~40 lines)
// Runs 100 ticks and prints lifecycle transitions + EmergenceReport snapshots.

use std::sync::mpsc;
use myth_wire::WirePacket;
use forge::ForgeModule;

fn main() {
    let (tx, rx) = mpsc::channel::<WirePacket>();
    let mut module = ForgeModule::new(tx);

    println!("=== BioSpark Forge — Headless Simulation ===");
    println!("Council: 9 Xyrona Prime guild agents initialized");
    println!();

    let mut last_phase = format!("{}", module.council().phase());

    for tick in 0..100 {
        module.run_tick();

        let phase = format!("{}", module.council().phase());
        if phase != last_phase {
            println!("  [tick {:>3}] Phase transition: {} → {}", tick, last_phase, phase);
            last_phase = phase;
        }

        if tick % 10 == 0 {
            while let Ok(packet) = rx.try_recv() {
                let council = module.council();
                println!(
                    "  [tick {:>3}] wire={:?}  soul={:.3} kΩ  resonance={:.4}  stability={:.2}",
                    tick,
                    packet.wire_type,
                    council.total_soul_weight.0,
                    council.total_world_resonance,
                    council.emergence_report().system_stability,
                );
            }
        }
    }

    println!();
    println!("Simulation complete. 100 ticks elapsed.");
    println!(
        "Final soul weight: {}   World resonance: {:.4}",
        module.council().total_soul_weight,
        module.council().total_world_resonance,
    );
}
