use crate::error::PluginResult;
use myth_wire::WirePacket;

/// A myth-os addon — a cross-instrument modifier that hooks into a plugin's
/// output stream after `process()` runs.
///
/// Addons are cross-instrument: the same addon binary can be attached to
/// multiple plugins simultaneously. They carry a Sigil — independent heraldry
/// with no required parent Crest.
///
/// # What addons can do
/// - Filter packets (remove unwanted output)
/// - Augment packets (add fields, adjust values)
/// - Split packets (turn one into many)
/// - Merge packets (collapse many into one)
///
/// # What addons cannot do
/// - Access the Vault directly (no handle provided — go through the plugin)
/// - Call `process()` on the plugin they are attached to
/// - Mutate the plugin's internal state
///
/// # Execution order
/// When multiple addons are attached to the same plugin, they run in
/// registration order. Each addon receives the output of the previous one.
pub trait MythAddon: Send + Sync + 'static {
    /// Stable machine identifier. Must be unique per plugin attachment.
    fn id(&self) -> &str;

    /// The plugin id this addon registers on.
    ///
    /// The registry validates this matches a registered plugin before accepting
    /// the addon. Use `"*"` to indicate the addon is compatible with any plugin
    /// (the caller is responsible for registering it on the correct target).
    fn target_plugin(&self) -> &str;

    /// Heraldry Sigil for this addon.
    /// Format: `"Sigil:<symbol>"`, e.g. `"Sigil:WaterFill"`.
    fn heraldry_symbol(&self) -> &str { "" }

    /// Called after the host plugin's `process()` returns.
    ///
    /// `source_packet` — the original packet that triggered the plugin.
    /// `plugin_output` — what the plugin returned. Modify and return.
    ///
    /// Return the (possibly modified) output. The next addon in the chain
    /// receives whatever this function returns.
    fn on_output(
        &self,
        source_packet: &WirePacket,
        plugin_output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>>;

    /// Called after the host plugin's `tick()` returns.
    ///
    /// Default is pass-through — override only if tick output needs modification.
    fn on_tick_output(
        &self,
        _delta_ms: u64,
        tick_output: Vec<WirePacket>,
    ) -> PluginResult<Vec<WirePacket>> {
        Ok(tick_output)
    }
}
