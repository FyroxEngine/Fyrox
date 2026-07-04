/// myth-clock — Genesis Sequence Demo
///
/// Simulates the OS clock booting, three subsystems subscribing,
/// then the Genesis Protocol firing and crystallising.
///
/// The temperature output mirrors the ATOMS engine log:
///   temp:0.68 [SETTLING] → temp:0.38 [CRYSTALLISING]
///
/// Run: cargo run --example genesis_sequence -p myth-clock

use myth_clock::{MythClock, ClockSubscriber};
use myth_clock::tick::ClockPhase;
use mythos::seed_pool::AtomPool;
use std::thread;

fn main() {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║     BIOSPHERES-OS — GENESIS SEQUENCE DEMO       ║");
    println!("║     CPU_Scheduler_ATOM  →  Game_Tick_ATOM       ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    // ── Load the atom pool ────────────────────────────────────────────────────
    let seeds_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()   // services/myth-clock → services/
        .parent().unwrap()   // services/ → workspace root
        .join("assets/atoms/seeds.toml");

    match AtomPool::load(&seeds_path) {
        Ok(pool) => {
            let stats = pool.stats();
            println!("  ⚗  Atom pool loaded — {}", stats);
            println!("  ⚗  Top bond candidates (by resonance):");
            for (a, b, score) in pool.bond_candidates().iter().take(5) {
                println!("       [{score}] {} ↔ {}", a.id, b.id);
            }
            println!();
        }
        Err(e) => println!("  ⚠  Atom pool unavailable: {e}\n"),
    }

    // ── Boot the OS clock at 30fps (demo rate) ───────────────────────────────
    let mut clock = MythClock::new(30.0);

    // ── Three subsystems subscribe during boot ────────────────────────────────
    let daw_sub      = clock.subscribe("myth-daw Transport");
    let theatre_sub  = clock.subscribe("BioSpark Theatre");
    let genesis_sub  = clock.subscribe("Genesis Protocol");

    println!("▶  OS clock booted — {} subscribers registered\n", clock.subscriber_count());

    // ── Fire genesis and start BEFORE spawning threads ───────────────────────
    clock.begin_genesis();
    clock.start();
    println!("  ⚡ Genesis Protocol initiated — soup planted, cooling begins\n");

    // ── Spawn listener threads (simulate each subsystem) ─────────────────────
    let daw_thread = thread::spawn(move || listen("myth-daw", daw_sub, 90));
    let theatre_thread = thread::spawn(move || listen("Theatre", theatre_sub, 90));

    // Genesis Protocol thread — watches temperature and reports phase changes
    let genesis_thread = thread::spawn(move || {
        let mut last_phase = ClockPhase::Booting;
        for _ in 0..90 {
            match genesis_sub.recv() {
                Ok(tick) => {
                    if tick.phase != last_phase {
                        println!("\n  ✦ Genesis Protocol: phase → {:?}", tick.phase);
                        last_phase = tick.phase;
                    }
                    if tick.frame % 6 == 0 {
                        let temp_bar = (tick.temperature * 20.0) as usize;
                        let bar = "█".repeat(temp_bar) + &"░".repeat(20 - temp_bar);
                        println!("  Genesis  tick:{:03}  temp:{:.2}  [{}]  {:?}",
                            tick.frame, tick.temperature, bar, tick.phase);
                    }
                }
                Err(_) => break,
            }
        }
        println!("\n  ✦ Genesis Protocol: world is stable");
    });

    // ── Run the clock for 3 seconds (90 frames at 30fps) ─────────────────────
    for _ in 0..90 {
        clock.tick();
        thread::sleep(std::time::Duration::from_millis(33));
    }
    clock.stop();

    // ── Wait for all threads ──────────────────────────────────────────────────
    daw_thread.join().ok();
    theatre_thread.join().ok();
    genesis_thread.join().ok();

    println!("\n══════════════════════════════════════════════════");
    println!("  Final frame    : {}", clock.frame());
    println!("  Elapsed        : {:.2}s", clock.elapsed_secs());
    println!("  Temperature    : {:.3}", clock.temperature());
    println!("  Phase          : {:?}", clock.phase());
    println!("══════════════════════════════════════════════════");
    println!("\n  The show can run on time. 🚀\n");
}

fn listen(name: &str, sub: ClockSubscriber, frames: u64) {
    let mut count = 0u64;
    loop {
        match sub.recv() {
            Ok(tick) => {
                count += 1;
                if count >= frames { break; }
                // Each subsystem just counts beats — real impl would do work here
                let _ = tick.beats_at_bpm(120.0);
            }
            Err(_) => break,
        }
    }
    println!("  {} received {} ticks", name, count);
}
