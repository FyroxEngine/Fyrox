// myth-plugin — Plugin and Addon trait system for the myth-os / BioSpark ecosystem.
//
// Everything above the Vault is either a plugin or an addon.
// Core instruments (built-in) and external plugins implement the same traits.
// The registry cannot tell them apart — both are `dyn MythPlugin`.
//
// Dependency rule: myth-wire, myth-vault, thiserror, tracing only.
// No Bevy. No egui. No audio. No tokio.
// Plugins that need those deps take them in their own Cargo.toml.

pub mod addon;
pub mod error;
pub mod layout;
pub mod plugin;
pub mod registry;

pub use addon::MythAddon;
pub use error::{PluginError, PluginResult};
pub use layout::{
    DeniedSlot, GrantedSlot, LayoutGrant, LayoutRequest, SlotRequest, SlotType, Visibility,
};
pub use plugin::MythPlugin;
pub use registry::PluginRegistry;
