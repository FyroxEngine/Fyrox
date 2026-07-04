// agent-engine — Headless BioSpark Agent Workforce binary (~60 lines)
//
// Wires: ForgeModule (tick loop) + WirePacket channel drain.
// No business logic — wiring only.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use myth_wire::WirePacket;
use forge::ForgeModule;

fn main() {
    println!("╔══════════════════════════════════════════╗");
    println!("║  BioSpark Agent Engine — Headless        ║");
    println!("║  Xyrona Prime Guild Council — 9 Agents  ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    let (tx, rx) = mpsc::channel::<WirePacket>();
    let mut forge = ForgeModule::new(tx);

    println!("[ENGINE] Council initialized. Starting tick loop (100ms/tick).");
    println!("[ENGINE] Press Ctrl+C to stop.");
    println!();

    loop {
        forge.run_tick();
        thread::sleep(Duration::from_millis(100));

        while let Ok(packet) = rx.try_recv() {
            eprintln!("[ENGINE] wire_type={:?}  tick={}", packet.wire_type, packet.tick);
        }
    }
}
