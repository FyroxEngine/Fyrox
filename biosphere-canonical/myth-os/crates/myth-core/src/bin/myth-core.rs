// myth-core binary — the headless Core supervisor process.
//
// This is intentionally thin. All logic lives in the lib. This file boots
// the atoms, waits for Ctrl-C, then shuts down cleanly.
//
// Ports / addresses are hardcoded for the initial build. They will move to
// a config file when the Theater transport layer is wired in.

use myth_core::atoms::{
    audit::CoreAudit,
    bus::BusRouter,
    clock::ClockSignal,
    failsafe::EmergencyFailsafe,
    health::HealthMonitor,
    interrupt::InterruptSequencer,
    socket::SocketManager,
    state_machine::StateMachineSupervisor,
    thread_pool::ThreadOrchestrator,
};
use myth_core::signal::SignalKind;
use myth_wire::MythId;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("myth_core=debug".parse()?)
                .add_directive("myth_vault=info".parse()?)
                .add_directive("myth_quill=info".parse()?),
        )
        .init();

    let core_id = MythId::new();
    info!(id = %core_id, "MYTH-CORE BOOTING");

    // ── Atoms ─────────────────────────────────────────────────────────────────
    let (bus, _root_rx) = BusRouter::new(4096);
    let bus = Arc::new(bus);

    let clock     = ClockSignal::new(60, bus.sender());
    let _threads  = ThreadOrchestrator::new(available_cpus());
    let _state    = Arc::new(StateMachineSupervisor::default());
    let _health   = Arc::new(HealthMonitor::default());
    let _failsafe = Arc::new(EmergencyFailsafe::default());
    let interrupts = Arc::new(InterruptSequencer::default());
    let sockets   = SocketManager::new("127.0.0.1:7700");
    let audit     = CoreAudit::open("data/myth-core/log")?;

    audit.log(&core_id, "CORE_BOOT");

    // Shared atomic tick counter — written by clock, read by health endpoint.
    let tick_counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

    // ── Spawn clock ────────────────────────────────────────────────────────────
    let tick_writer = Arc::clone(&tick_counter);
    let mut clock_rx = bus.subscribe();
    let clock_handle = tokio::spawn(clock.run());

    // Increment tick counter from the bus.
    let tick_tracker = tokio::spawn(async move {
        loop {
            match clock_rx.recv().await {
                Ok(_) => { tick_writer.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
    });

    // ── Health TCP endpoint — 127.0.0.1:7701 — responds with JSON clock state ──
    //
    // Response: {"ok":true,"tick":<u64>,"tempo_bpm":120.0,"beat":<0.0-1.0>}
    // Library's CoreStatus reads this to feed FrameContext to the Theatre mixer.
    let tick_reader = Arc::clone(&tick_counter);
    let health_handle = tokio::spawn(async move {
        match tokio::net::TcpListener::bind("127.0.0.1:7701").await {
            Ok(listener) => {
                info!("HEALTH endpoint on 127.0.0.1:7701");
                loop {
                    if let Ok((mut stream, _)) = listener.accept().await {
                        use tokio::io::AsyncWriteExt;
                        let tick   = tick_reader.load(std::sync::atomic::Ordering::Relaxed);
                        // Beat: fractional position within a 120 BPM bar.
                        // At 120 BPM a bar = 2 s = 120 ticks (60 Hz).
                        let beat   = (tick % 120) as f32 / 120.0;
                        let msg    = format!(
                            "{{\"ok\":true,\"tick\":{},\"tempo_bpm\":120.0,\"beat\":{:.4}}}\n",
                            tick, beat
                        );
                        let _ = stream.write_all(msg.as_bytes()).await;
                    }
                }
            }
            Err(e) => error!("HEALTH bind failed: {e}"),
        }
    });

    // ── Heartbeat log — proof-of-life every 60 s ───────────────────────────────
    let heartbeat_handle = tokio::spawn(async {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            info!("MYTH-CORE HEARTBEAT — nominal");
        }
    });

    // ── Interrupt drain ────────────────────────────────────────────────────────
    let interrupts_clone = Arc::clone(&interrupts);
    let mut bus_rx = bus.subscribe();
    let dispatch_handle = tokio::spawn(async move {
        loop {
            match bus_rx.recv().await {
                Ok(sig) => {
                    use myth_core::signal::SignalPriority;
                    if sig.priority == SignalPriority::Critical {
                        interrupts_clone.push(sig.clone());
                    }
                    if matches!(sig.kind, SignalKind::Shutdown) {
                        info!("Shutdown signal on bus");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    error!("Bus lagged by {n} messages");
                }
                Err(_) => break,
            }
        }
    });

    info!(addr = %sockets.bind_addr, "MYTH-CORE ONLINE");

    // ── Graceful shutdown ──────────────────────────────────────────────────────
    signal::ctrl_c().await?;
    info!("Ctrl-C — shutting down");
    audit.log(&core_id, "CORE_SHUTDOWN");

    clock_handle.abort();
    tick_tracker.abort();
    dispatch_handle.abort();
    heartbeat_handle.abort();
    health_handle.abort();

    Ok(())
}

fn available_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
