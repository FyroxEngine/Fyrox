use crate::error::PluginResult;
use myth_vault::VaultRegistry;
use myth_wire::{WirePacket, WireType};
use std::sync::Arc;

/// A myth-os plugin — an instrument that attaches to a Vault and
/// processes WirePackets.
///
/// Core instruments (built-in) implement this trait using compile-time ATOMs.
/// External plugins implement this trait using runtime ATOMs.
/// The registry cannot tell the difference — both are `dyn MythPlugin`.
///
/// # Heraldry
/// Plugins carry a Glyph that inherits the Crest of the instrument they extend.
/// The `heraldry_symbol()` return value is used by the registry for symbolic
/// routing and by the Vault Portal for access control. See: Order of the
/// Quantum Quill skill for Portal access rules.
///
/// # Closed System
/// Plugins interact with the world ONLY through WirePackets.
/// `process()` receives one packet and returns zero or more packets.
/// Direct mutation of simulation state is not possible by design.
pub trait MythPlugin: Send + Sync + 'static {
    // ── Identity ──────────────────────────────────────────────────────────────

    /// Stable machine identifier. Must be unique in the registry.
    /// Convention: kebab-case, e.g. `"myth-terrain"`, `"user-erosion"`.
    fn id(&self) -> &str;

    /// Human-readable display name.
    fn name(&self) -> &str;

    /// Semver triple. Core instruments use (0, 1, 0) until API is stable.
    fn version(&self) -> (u32, u32, u32);

    /// Wire types this plugin consumes. The registry routes only matching packets.
    fn wire_in(&self) -> &[WireType];

    /// Wire types this plugin emits. Declared for routing — not enforced at runtime
    /// but adapters depend on it to decide whether to subscribe.
    fn wire_out(&self) -> &[WireType];

    /// Heraldry symbol for this plugin — used for routing and Vault access.
    /// Format: `"Glyph:<symbol>↑<parent-crest>"`, e.g. `"Glyph:Erosion↑Atlas"`.
    /// Return empty string if heraldry is not yet assigned.
    fn heraldry_symbol(&self) -> &str { "" }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    /// Called when the plugin is registered. Receives a handle to the Vault.
    /// Store the handle if the plugin needs to read or write capsule data.
    fn on_attach(&mut self, vault: Arc<VaultRegistry>) -> PluginResult<()>;

    /// Called when the plugin is unregistered or the system shuts down.
    fn on_detach(&mut self) -> PluginResult<()>;

    // ── Work ──────────────────────────────────────────────────────────────────

    /// Process one incoming WirePacket. Return zero or more output packets.
    ///
    /// Only called for packet wire types declared in `wire_in()`.
    /// Must not block — offload heavy work to a background task and emit
    /// a result packet when done.
    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>>;

    /// Advance the plugin by `delta_ms` milliseconds.
    ///
    /// Called every simulation tick. Default is a no-op — override only for
    /// plugins that need time-driven output independent of incoming packets
    /// (e.g. a weather system that emits ENR packets on a cycle).
    fn tick(&mut self, delta_ms: u64) -> PluginResult<Vec<WirePacket>> {
        let _ = delta_ms;
        Ok(vec![])
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    /// Declare the UI slots this plugin wants.
    ///
    /// Called by `PluginRegistry::negotiate_layout()` after registration.
    /// The registry checks the active UIGenesis and either grants or denies
    /// each request. Denial includes the occupant's heraldry so the plugin
    /// can make an informed counter-offer.
    ///
    /// Default implementation requests no UI slots (headless/background plugin).
    fn layout_request(&self) -> crate::layout::LayoutRequest {
        crate::layout::LayoutRequest::default()
    }
}
