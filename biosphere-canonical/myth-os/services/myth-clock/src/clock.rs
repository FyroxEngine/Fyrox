use std::collections::HashMap;
use std::time::{Duration, Instant};
use crossbeam_channel::{bounded, Sender};
use tracing::{info, warn};

use crate::subscriber::{ClockSubscriber, SubscriberId};
use crate::tick::{ClockPhase, Tick};

/// BioSpheres OS master clock — CPU_Scheduler_ATOM.
///
/// One instance runs per OS. All subsystems subscribe and receive the same
/// Tick every frame. The Genesis Protocol uses temperature to know when the
/// world has crystallised. myth-daw Transport slaves its position to frame.
/// BioSpark Theatre composites on each tick. Sociomind agents step on each tick.
///
/// This is the heartbeat the Interstellar Tour runs on.
pub struct MythClock {
    sample_rate:  f64,
    frame:        u64,
    elapsed_secs: f64,
    phase:        ClockPhase,
    temperature:  f64,
    cooling_rate: f64,

    /// All registered subscribers — each gets a clone of every Tick.
    subscribers: HashMap<SubscriberId, Sender<Tick>>,
}

impl MythClock {
    /// Create a new clock at the given sample rate (frames per second).
    /// 60.0 for UI/render, 44100.0 for audio-accurate, 30.0 for demo.
    pub fn new(sample_rate: f64) -> Self {
        assert!(sample_rate > 0.0, "sample_rate must be positive");
        Self {
            sample_rate,
            frame:        0,
            elapsed_secs: 0.0,
            phase:        ClockPhase::Booting,
            temperature:  1.0,
            cooling_rate: 0.02, // matches ATOMS engine default cooling
            subscribers:  HashMap::new(),
        }
    }

    /// Register a new subscriber. Returns the receiving end.
    /// Call this during system boot before starting the tick loop.
    pub fn subscribe(&mut self, name: impl Into<String>) -> ClockSubscriber {
        let id = SubscriberId::new();
        let name = name.into();
        // Buffer of 4 — subscribers must keep up or ticks are dropped.
        let (tx, rx) = bounded(4);
        self.subscribers.insert(id, tx);
        info!("myth-clock: {} subscribed ({})", name, id);
        ClockSubscriber { id, name, receiver: rx }
    }

    /// Unregister a subscriber (e.g. when a vault unloads).
    pub fn unsubscribe(&mut self, id: SubscriberId) {
        if self.subscribers.remove(&id).is_some() {
            info!("myth-clock: {} unsubscribed", id);
        }
    }

    /// Transition out of Booting into Running.
    pub fn start(&mut self) {
        if self.phase != ClockPhase::Booting { return; }
        self.phase = ClockPhase::Running;
        info!("myth-clock: RUNNING at {:.1} fps — {} subscribers",
            self.sample_rate, self.subscribers.len());
    }

    pub fn pause(&mut self) {
        if self.phase == ClockPhase::Running {
            self.phase = ClockPhase::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.phase == ClockPhase::Paused {
            self.phase = ClockPhase::Running;
        }
    }

    /// Begin the Genesis Protocol cooling sequence.
    /// Temperature will drop from current value toward 0.0.
    /// When it hits the crystallisation threshold the phase changes.
    /// Safe to call whether the clock is Booting or already Running.
    pub fn begin_genesis(&mut self) {
        info!("myth-clock: Genesis Protocol initiated — cooling begins");
        self.temperature = 1.0;
        if self.phase == ClockPhase::Booting {
            self.phase = ClockPhase::Running;
        }
    }

    /// Advance one frame and broadcast to all subscribers.
    /// Call this in your OS tick loop.
    pub fn tick(&mut self) {
        if self.phase == ClockPhase::Paused || self.phase == ClockPhase::Stopped {
            return;
        }

        let delta = 1.0 / self.sample_rate;
        self.elapsed_secs += delta;
        self.frame += 1;

        // Cool the temperature each frame during genesis
        if self.temperature > 0.0 {
            self.temperature = (self.temperature - self.cooling_rate * delta).max(0.0);
            if self.temperature <= 0.38 && self.phase == ClockPhase::Running {
                self.phase = ClockPhase::Crystallising;
                info!("myth-clock: [CRYSTALLISING] — temp {:.3}", self.temperature);
            }
        }

        let t = Tick {
            frame:        self.frame,
            elapsed_secs: self.elapsed_secs,
            delta_secs:   delta,
            sample_rate:  self.sample_rate,
            temperature:  self.temperature,
            phase:        self.phase,
        };

        // Broadcast — drop disconnected subscribers
        let mut dead = Vec::new();
        for (id, tx) in &self.subscribers {
            if tx.try_send(t).is_err() {
                warn!("myth-clock: subscriber {} lagging or disconnected", id);
                dead.push(*id);
            }
        }
        for id in dead {
            self.subscribers.remove(&id);
        }
    }

    /// Run a blocking tick loop at the configured sample rate.
    /// Spawns in the current thread — use tokio::spawn for async contexts.
    pub fn run_blocking(&mut self) {
        self.start();
        let frame_duration = Duration::from_secs_f64(1.0 / self.sample_rate);
        let mut next_tick = Instant::now();

        loop {
            if self.phase == ClockPhase::Stopped {
                break;
            }
            self.tick();
            next_tick += frame_duration;
            let now = Instant::now();
            if next_tick > now {
                std::thread::sleep(next_tick - now);
            }
        }
    }

    pub fn stop(&mut self) {
        self.phase = ClockPhase::Stopped;
        info!("myth-clock: STOPPED at frame {}", self.frame);
    }

    pub fn frame(&self)        -> u64       { self.frame }
    pub fn elapsed_secs(&self) -> f64       { self.elapsed_secs }
    pub fn temperature(&self)  -> f64       { self.temperature }
    pub fn phase(&self)        -> ClockPhase { self.phase }
    pub fn subscriber_count(&self) -> usize { self.subscribers.len() }

    /// Set cooling rate. Default 0.02 matches the ATOMS engine.
    /// Lower = slower crystallisation, higher = faster.
    pub fn set_cooling_rate(&mut self, rate: f64) {
        self.cooling_rate = rate;
    }
}
