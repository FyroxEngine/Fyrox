//! A set of useful scripts that can be used to in your game.

use crate::camera::FlyingCameraController;
use fyrox::script::constructor::ScriptConstructorContainer;

pub mod camera;

/// Registers every script from the crate in the given constructor container. Use it, if you want to register all
/// available scripts at once. Typical usage could be like this:
///
/// ```rust,no_run
/// # use fyrox::{
/// #     core::pool::Handle, core::visitor::prelude::*, core::reflect::prelude::*,
/// #     plugin::{Plugin, PluginContext, PluginRegistrationContext},
/// #     scene::Scene,
/// # };
/// #
/// # #[derive(Visit, Reflect, Debug)]
/// # struct Game;
/// #
/// # impl Plugin for Game {
///   // This is PluginConstructor::register method of your GameConstructor.
///   fn register(&self, context: PluginRegistrationContext) {
///       fyrox_scripts::register(&context.serialization_context.script_constructors)
///   }
/// # }
/// ```
pub fn register(container: &ScriptConstructorContainer) {
    container.add::<FlyingCameraController>("Fyrox Flying Camera Controller");
}
