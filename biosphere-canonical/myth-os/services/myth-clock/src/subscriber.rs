use crossbeam_channel::Receiver;
use uuid::Uuid;
use crate::tick::Tick;

/// A unique ID for a clock subscriber.
/// myth-daw gets one. Theatre gets one. Each vault Core gets one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(pub Uuid);

impl SubscriberId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SubscriberId {
    fn default() -> Self { Self::new() }
}

impl std::fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sub:{}", &self.0.to_string()[..8])
    }
}

/// A subscriber's receiving end of the clock broadcast.
///
/// Each subsystem (DAW, Theatre, Core Engine, Genesis Protocol) holds one of
/// these. Every frame the MythClock broadcasts a Tick to all subscribers
/// simultaneously via crossbeam channels.
pub struct ClockSubscriber {
    pub id:       SubscriberId,
    pub name:     String,
    pub receiver: Receiver<Tick>,
}

impl ClockSubscriber {
    /// Block until the next tick arrives.
    pub fn recv(&self) -> Result<Tick, crossbeam_channel::RecvError> {
        self.receiver.recv()
    }

    /// Non-blocking — returns None if no tick is ready yet.
    pub fn try_recv(&self) -> Option<Tick> {
        self.receiver.try_recv().ok()
    }
}
