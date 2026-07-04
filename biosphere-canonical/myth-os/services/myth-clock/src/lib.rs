//! myth-clock — BioSpheres OS master heartbeat (CPU_Scheduler_ATOM)
//!
//! Every simulation layer slaves to this clock. myth-daw Transport, BioSpark
//! Theatre, Genesis Protocol world ticks, Sociomind agent steps — all of them
//! subscribe here and receive the same beat.
//!
//! The Genesis Protocol fires on a specific tick. The Interstellar Tour show
//! runs on this clock. When the planetarium dome opens, this is what's counting.

pub mod clock;
pub mod subscriber;
pub mod tick;

pub use clock::MythClock;
pub use subscriber::{ClockSubscriber, SubscriberId};
pub use tick::Tick;
