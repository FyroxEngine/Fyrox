//! The WASM ABI contract.
//!
//! Plugin .wasm files must export these functions. Plugin Foundry generates
//! the correct stubs automatically — plugin authors never write this by hand.
//!
//! All data crosses the WASM boundary as JSON-encoded bytes written into
//! linear memory. The host allocates, the guest reads/writes, the host frees.
//!
//! Export names (what the .wasm must export):
//!   myth_alloc(size: i32) -> i32          — allocate `size` bytes, return ptr
//!   myth_free(ptr: i32, size: i32)        — free previously allocated region
//!   myth_plugin_id() -> i32               — ptr to null-terminated id string
//!   myth_plugin_name() -> i32             — ptr to null-terminated name string
//!   myth_wire_in() -> i32                 — ptr to JSON array of WireType codes
//!   myth_wire_out() -> i32                — ptr to JSON array of WireType codes
//!   myth_heraldry() -> i32                — ptr to heraldry string
//!   myth_process(packet_ptr: i32, packet_len: i32) -> i32
//!                                         — ptr to JSON array of output WirePackets
//!   myth_tick(delta_ms: i64) -> i32       — ptr to JSON array (may be empty)

pub const EXPORT_ALLOC:      &str = "myth_alloc";
pub const EXPORT_FREE:       &str = "myth_free";
pub const EXPORT_ID:         &str = "myth_plugin_id";
pub const EXPORT_NAME:       &str = "myth_plugin_name";
pub const EXPORT_WIRE_IN:    &str = "myth_wire_in";
pub const EXPORT_WIRE_OUT:   &str = "myth_wire_out";
pub const EXPORT_HERALDRY:   &str = "myth_heraldry";
pub const EXPORT_PROCESS:    &str = "myth_process";
pub const EXPORT_TICK:       &str = "myth_tick";

/// Wasmtime fuel limit per process() call.
/// Prevents infinite loops from hanging the simulation tick.
/// 10 million fuel units ≈ ~10ms of simple computation at typical JIT speeds.
pub const PROCESS_FUEL_LIMIT: u64 = 10_000_000;
