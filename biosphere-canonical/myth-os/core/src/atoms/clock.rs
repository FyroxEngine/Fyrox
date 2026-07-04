// CORE-ATOM-01: Clock Signal — nanosecond-precision tick generator
use mythos::signal::BusSignal;
use mythos::identity::MythId;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use tracing::trace;

pub struct ClockSignal {
    tick_rate: Duration,
    tx: broadcast::Sender<BusSignal>,
    id: MythId,
}

impl ClockSignal {
    pub fn new(tick_hz: u64, tx: broadcast::Sender<BusSignal>) -> Self {
        Self {
            tick_rate: Duration::from_nanos(1_000_000_000 / tick_hz),
            tx,
            id: MythId::new(),
        }
    }

    /// Drives the clock; runs until the runtime shuts down.
    pub async fn run(self) {
        let mut ticker = interval(self.tick_rate);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let signal = BusSignal::tick(self.id.clone());
            trace!("TICK");
            let _ = self.tx.send(signal);
        }
    }
}
