pub mod audit;         // CORE-16 — immutable system transition log
pub mod bus;           // CORE-02 — zero-copy async broadcast router
pub mod clock;         // CORE-01 — nanosecond-precision tick generator
pub mod failsafe;      // CORE-15 — microsecond module isolation
pub mod health;        // CORE-13 — continuous heartbeat diagnostics
pub mod interrupt;     // CORE-04 — priority-ordered signal queue
pub mod socket;        // CORE-03 — TCP connection management
pub mod state_machine; // CORE-05 — node lifecycle state supervisor
pub mod thread_pool;   // CORE-06 — Tokio task pool orchestrator
