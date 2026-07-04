// CORE-ATOM-01: Clock Signal — nanosecond-precision simulation tick generator.
//
// The clock is the heartbeat of the entire ecosystem. Every other atom that
// needs to act on time subscribes to the bus and filters for SignalKind::Tick.
// The tick rate is set at construction and does not change at runtime.

use crate::signal::BusSignal;
use myth_wire::MythId;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration, MissedTickBehavior};
use tracing::trace;

pub struct ClockSignal {
    tick_rate: Duration,
    tx: broadcast::Sender<BusSignal>,
    id: MythId,
}

impl ClockSignal {
    /// Create a clock that fires `tick_hz` ticks per second.
    pub fn new(tick_hz: u64, tx: broadcast::Sender<BusSignal>) -> Self {
        Self {
            tick_rate: Duration::from_nanos(1_000_000_000 / tick_hz),
            tx,
            id: MythId::new(),
        }
    }

    /// Drive the clock loop. Runs until the runtime shuts down.
    /// Missing ticks are skipped (never queued up) to stay in real time.
    pub async fn run(self) {
        let mut ticker = interval(self.tick_rate);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            trace!("TICK");
            let _ = self.tx.send(BusSignal::tick(self.id.clone()));
        }
    }
}
