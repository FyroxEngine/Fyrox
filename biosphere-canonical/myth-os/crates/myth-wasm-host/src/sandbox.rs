//! Sandbox configuration for Wasmtime.
//! Plugins get NO capabilities by default — no filesystem, no network,
//! no clock, no random. Everything comes through wire packets only.

use wasmtime::{Config, Engine};

/// Build a capability-restricted Wasmtime engine for plugin execution.
pub fn sandboxed_engine() -> Result<Engine, wasmtime::Error> {
    let mut config = Config::new();

    // Enable fuel-based execution limiting (prevents infinite loops)
    config.consume_fuel(true);

    // Cranelift JIT — fast enough for simulation ticks
    config.strategy(wasmtime::Strategy::Cranelift);

    // No WASI — plugins have zero ambient capabilities
    // All I/O goes through myth-wire packet passing only

    // Trap on OOB memory access rather than UB
    config.wasm_memory64(false);

    Engine::new(&config)
}
