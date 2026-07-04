#![allow(dead_code, unused_imports, unused_variables)]

mod atoms;

use atoms::{
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
use mythos::{
    identity::MythId,
    signal::{SignalKind, SignalPriority},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("core=debug".parse()?)
                .add_directive("vault=info".parse()?)
                .add_directive("genesis=info".parse()?),
        )
        .init();

    let core_id = MythId::new();
    info!(id = %core_id, "CORE SUPERVISOR BOOTING");

    // ── Atoms ──────────────────────────────────────────────────────────────
    let (bus, _root_rx) = BusRouter::new(4096);
    let bus = Arc::new(bus);

    let clock     = ClockSignal::new(60, bus.sender());
    let _threads  = ThreadOrchestrator::new(num_cpus());
    let _state    = Arc::new(StateMachineSupervisor::default());
    let _health   = Arc::new(HealthMonitor::default());
    let _failsafe = Arc::new(EmergencyFailsafe::default());
    let interrupts = Arc::new(InterruptSequencer::default());
    let sockets    = SocketManager::new("127.0.0.1:7700");
    let audit      = CoreAudit::open("data/core/log")?;

    audit.log(&core_id, "CORE_BOOT");

    // ── Spawn clock on its own task ────────────────────────────────────────
    let clock_handle = tokio::spawn(clock.run());

    // ── Health endpoint — lightweight TCP ping on 127.0.0.1:7701 ─────────
    // The Library polls this every 5 s to light the CORE status pip.
    let health_handle = tokio::spawn(async {
        match tokio::net::TcpListener::bind("127.0.0.1:7701").await {
            Ok(listener) => {
                info!("HEALTH endpoint listening on 127.0.0.1:7701");
                loop {
                    if let Ok((mut stream, _)) = listener.accept().await {
                        use tokio::io::AsyncWriteExt;
                        let _ = stream.write_all(b"OK\n").await;
                    }
                }
            }
            Err(e) => error!("HEALTH endpoint failed to bind: {e}"),
        }
    });

    // ── Heartbeat — logs proof-of-life every 60 seconds ───────────────────
    let heartbeat_handle = tokio::spawn(async {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            info!("CORE HEARTBEAT — all systems nominal");
        }
    });

    // ── Spawn interrupt drain loop ─────────────────────────────────────────
    let interrupts_clone = Arc::clone(&interrupts);
    let mut bus_rx = bus.subscribe();

    let dispatch_handle = tokio::spawn(async move {
        loop {
            match bus_rx.recv().await {
                Ok(sig) => {
                    if sig.priority == SignalPriority::Critical {
                        interrupts_clone.push(sig.clone());
                    }
                    if matches!(sig.kind, SignalKind::Shutdown) {
                        info!("Shutdown signal received on bus");
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    error!("Bus receiver lagged by {} messages", n);
                }
                Err(_) => break,
            }
        }
    });

    info!(
        addr = %sockets.bind_addr,
        "CORE ONLINE — waiting for shutdown signal"
    );

    // ── Graceful shutdown on Ctrl-C ────────────────────────────────────────
    signal::ctrl_c().await?;
    info!("Ctrl-C received — shutting down Core");
    audit.log(&core_id, "CORE_SHUTDOWN");

    clock_handle.abort();
    dispatch_handle.abort();
    heartbeat_handle.abort();
    health_handle.abort();

    Ok(())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
