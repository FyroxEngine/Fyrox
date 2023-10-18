//! A set of useful scripts that can be used to in your game.

use crate::camera::FlyingCameraController;
use fyrox::script::constructor::ScriptConstructorContainer;

pub mod camera;

/// Registers every script from the crate in the given constructor container. Use it, if you want to register all
/// available scripts at once. Typical usage could be like this:
///
/// ```rust
/// # use fyrox::{
/// #     core::pool::Handle,
/// #     plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
/// #     scene::Scene,
/// # };
/// #
/// # struct GameConstructor;
/// #
/// # impl PluginConstructor for GameConstructor {
///   // This is PluginConstructor::register method of your GameConstructor.
///   fn register(&self, context: PluginRegistrationContext) {
///       fyrox_scripts::register(&context.serialization_context.script_constructors)
///   }
/// #   fn create_instance(
/// #       &self,
/// #       _scene_path: Option<&str>,
/// #       _context: PluginContext,
/// #   ) -> Box<dyn Plugin> {
/// #       unimplemented!()
/// #   }
/// # }
/// ```
pub fn register(container: &ScriptConstructorContainer) {
    container.add::<FlyingCameraController>("Fyrox Flying Camera Controller");
}
