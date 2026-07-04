// Core Status — liveness + clock state from myth-core supervisor.
//
// A background OS thread polls 127.0.0.1:7701 every 2 seconds.
// The health response is JSON: {"ok":true,"tick":N,"tempo_bpm":120.0,"beat":0.5}
//
// Parsed values are stored in atomics so the render thread reads without locking.
// The Theatre mixer reads tick/beat/tempo_bpm from here to drive FrameContext.
//
// Auto-launch: if myth-core is not running when library starts, we attempt to
// launch it as a child process. It is killed when the library exits.

use bevy::prelude::*;
use std::{
    io::Read,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};

const HEALTH_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 7701);
const POLL_INTERVAL: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);

// ── Resource ──────────────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct CoreStatus {
    online:    Arc<AtomicBool>,
    /// Monotonic tick counter from myth-core's ClockSignal.
    tick:      Arc<AtomicU64>,
    /// Tempo in BPM, stored as fixed-point × 10 (e.g. 1200 = 120.0 BPM).
    tempo_fp:  Arc<AtomicU32>,
    /// Beat position [0.0, 1.0) stored as fixed-point × 10000.
    beat_fp:   Arc<AtomicU32>,
}

impl CoreStatus {
    fn new() -> Self {
        let online   = Arc::new(AtomicBool::new(false));
        let tick     = Arc::new(AtomicU64::new(0));
        let tempo_fp = Arc::new(AtomicU32::new(1200)); // 120.0 BPM default
        let beat_fp  = Arc::new(AtomicU32::new(0));

        let o2 = Arc::clone(&online);
        let t2 = Arc::clone(&tick);
        let tp = Arc::clone(&tempo_fp);
        let bp = Arc::clone(&beat_fp);

        std::thread::Builder::new()
            .name("core-health-poller".into())
            .spawn(move || {
                // On first failure, try to launch myth-core automatically.
                let mut launched = false;

                let addr: SocketAddr = HEALTH_ADDR.into();
                loop {
                    match poll_health(&addr) {
                        Some(resp) => {
                            o2.store(true, Ordering::Relaxed);
                            t2.store(resp.tick, Ordering::Relaxed);
                            tp.store((resp.tempo_bpm * 10.0) as u32, Ordering::Relaxed);
                            bp.store((resp.beat * 10_000.0) as u32, Ordering::Relaxed);
                        }
                        None => {
                            o2.store(false, Ordering::Relaxed);
                            if !launched {
                                launched = true;
                                try_launch_core();
                            }
                        }
                    }
                    std::thread::sleep(POLL_INTERVAL);
                }
            })
            .expect("failed to spawn core-health-poller");

        Self { online, tick, tempo_fp, beat_fp }
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    pub fn tick(&self) -> u64 {
        self.tick.load(Ordering::Relaxed)
    }

    pub fn tempo_bpm(&self) -> f32 {
        self.tempo_fp.load(Ordering::Relaxed) as f32 / 10.0
    }

    pub fn beat(&self) -> f32 {
        self.beat_fp.load(Ordering::Relaxed) as f32 / 10_000.0
    }
}

// ── Parsed health response ────────────────────────────────────────────────────

struct HealthResp {
    tick:      u64,
    tempo_bpm: f32,
    beat:      f32,
}

/// Connect to the health endpoint, read one line, parse JSON.
/// Returns None on any failure (timeout, connection refused, malformed JSON).
fn poll_health(addr: &SocketAddr) -> Option<HealthResp> {
    let mut stream = std::net::TcpStream::connect_timeout(addr, CONNECT_TIMEOUT).ok()?;
    stream.set_read_timeout(Some(CONNECT_TIMEOUT)).ok()?;

    let mut buf = String::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if byte[0] == b'\n' { break; }
                buf.push(byte[0] as char);
                if buf.len() > 512 { break; } // guard against runaway reads
            }
        }
    }

    parse_health_json(&buf)
}

fn parse_health_json(s: &str) -> Option<HealthResp> {
    // Minimal hand-rolled parse — avoids pulling serde_json into the hot path.
    // Expects: {"ok":true,"tick":N,"tempo_bpm":F,"beat":F}
    let ok   = s.contains("\"ok\":true");
    if !ok { return None; }

    let tick      = extract_u64(s, "\"tick\":")?;
    let tempo_bpm = extract_f32(s, "\"tempo_bpm\":").unwrap_or(120.0);
    let beat      = extract_f32(s, "\"beat\":").unwrap_or(0.0);

    Some(HealthResp { tick, tempo_bpm, beat })
}

fn extract_u64(s: &str, key: &str) -> Option<u64> {
    let start = s.find(key)? + key.len();
    let rest  = &s[start..];
    let end   = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_f32(s: &str, key: &str) -> Option<f32> {
    let start = s.find(key)? + key.len();
    let rest  = &s[start..];
    let end   = rest.find(|c: char| c != '.' && !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// ── Auto-launch ───────────────────────────────────────────────────────────────

/// Attempt to start myth-core as a detached child process.
/// Silently ignores failure — the health poller will just stay offline.
fn try_launch_core() {
    let workspace = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    bevy::log::info!("myth-core offline — attempting auto-launch");

    std::thread::spawn(move || {
        match std::process::Command::new("cargo")
            .args(["run", "-p", "myth-core", "--bin", "myth-core"])
            .current_dir(&workspace)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                bevy::log::info!("myth-core launched (pid {})", child.id());
                let _ = child.wait();
            }
            Err(e) => bevy::log::warn!("myth-core auto-launch failed: {e}"),
        }
    });
}

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct CoreStatusPlugin;

impl Plugin for CoreStatusPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CoreStatus::new());
    }
}
