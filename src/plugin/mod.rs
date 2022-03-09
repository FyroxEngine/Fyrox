//! Everything related to plugins. See [`Plugin`] docs for more info.

#![warn(missing_docs)]

use crate::{
    core::pool::Handle,
    core::uuid::Uuid,
    engine::{resource_manager::ResourceManager, SerializationContext},
    event::Event,
    renderer::Renderer,
    scene::{Scene, SceneContainer},
};
use std::sync::Arc;

/// Contains plugin environment for the registration stage.
pub struct PluginRegistrationContext {
    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: Arc<SerializationContext>,
}

/// Contains plugin environment.
pub struct PluginContext<'a> {
    /// `true` if  the plugin running under the editor, `false` - otherwise.
    pub is_in_editor: bool,

    /// A reference to scene container of the engine. You can add new scenes from [`Plugin`] methods
    /// by using [`SceneContainer::add`].
    ///
    /// # Important notes
    ///
    /// Do not clear this container when running your plugin in the editor, otherwise you'll get
    /// panic. Every scene that was added in the container while "play mode" in the editor was
    /// active will be removed when you leave play mode.
    pub scenes: &'a mut SceneContainer,

    /// A reference to the resource manager, it can be used to load various resources and manage
    /// them. See [`ResourceManager`] docs for more info.
    pub resource_manager: &'a ResourceManager,

    /// A reference to the renderer, it can be used to add custom render passes (for example to
    /// render custom effects and so on).
    pub renderer: &'a mut Renderer,

    /// The time (in seconds) that passed since last call of a method in which the context was
    /// passed.
    pub dt: f32,

    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: Arc<SerializationContext>,
}

/// Plugin is a convenient interface that allow you to extend engine's functionality.
///
/// # Engine vs Framework
///
/// There are two completely different approaches that could be used to use the engine: you either
/// use the engine in "true" engine mode, or use it in framework mode. The "true" engine mode fixes
/// high-level structure of your game and forces you to implement game logic inside plugins and
/// scripts. The framework mode provides low-level access to engine details and leaves implementation
/// details to you.
///
/// By default the engine, if used alone, **completely ignores** every plugin, it calls a few methods
/// ([`Plugin::on_register`], [`Plugin::on_standalone_init`]) and does not call any other methods.
/// The plugins are meant to be used only in "true" engine mode. If you're using the engine alone
/// (without the editor, executor, and required project structure), it means that you're using the
/// engine in **framework** mode and you're able to setup your project as you want.
///
/// The plugins managed either by `Executor` or the editor (`Fyroxed`). The first one is a small
/// framework that calls all methods of the plugin as it needs to be, `Executor` is used to build
/// final binary of your game. The editor is also able to use plugins, it manages them in special
/// way that guarantees some invariants.
///
/// # Interface details
///
/// There is one confusing part in the plugin interface: two methods that looks like they're doing
/// the same thing - [`Plugin::on_standalone_init`] and [`Plugin::on_enter_play_mode`]. However
/// there is one major difference in the two. The first method is called when the plugin is running
/// in a standalone mode (in game executor, which is final binary of your game). The second is used
/// in the editor and called when the editor enters "play mode".
///
/// The "play mode" is special and should be described a bit more. The editor is able to edit
/// scenes, there could be only one scene opened at a time. However your game could use multiple
/// scenes (for example one for game menu and one per game level). This fact leads to a problem:
/// how the game will know which scene is currently edited and requested for "play mode"?
/// [`Plugin::on_enter_play_mode`] solves the problem by providing you the handle to the active
/// scene, in this method you should force your game to use provided scene.
///
/// # Static vs dynamic plugins
///
/// Every plugin must be linked statically to ensure that everything is memory safe. There was some
/// long research about hot reloading and dynamic plugins (in DLLs) and it turned out that they're
/// not guaranteed to be memory safe because Rust does not have stable ABI. When a plugin compiled
/// into DLL, Rust compiler is free to reorder struct members in any way it needs to. It is not
/// guaranteed that two projects that uses the same library will have compatible ABI. This fact
/// indicates that you either have to use static linking of your plugins or provide C interface
/// to every part of the engine and "communicate" with plugin using C interface with C ABI (which
/// is standardized and guaranteed to be compatible). The main problem with C interface is
/// boilerplate code and the need to mark every structure "visible" through C interface with
/// `#[repr(C)]` attribute which is not always easy and even possible (because some structures could
/// be re-exported from dependencies). These are the main reasons why the engine uses static plugins.
///
/// # Example
///
/// ```rust
/// use fyrox::{
///     core::{pool::Handle, uuid::Uuid},
///     plugin::{Plugin, PluginContext, PluginRegistrationContext},
///     scene::Scene,
///     event::Event
/// };
/// use std::str::FromStr;
///
/// struct MyPlugin {}
///
/// impl Plugin for MyPlugin {
///     fn on_register(&mut self, context: PluginRegistrationContext) {
///         // The method is called when the plugin was just registered in the engine.
///         // Register your scripts here using `context`.
///         // The implementation is optional.
///     }
///
///     fn on_standalone_init(&mut self, context: PluginContext) {
///         // The method is called when the plugin is running in standalone mode (editor-less).
///         // The implementation is optional.
///     }
///
///     fn on_enter_play_mode(&mut self, scene: Handle<Scene>, context: PluginContext) {
///         // The method is called when the plugin is running inside the editor and it enters
///         // "play mode".
///         // The implementation is optional.
///     }
///
///     fn on_leave_play_mode(&mut self, context: PluginContext) {
///         // The method is called when the plugin is running inside the editor and it leaves
///         // "play mode".
///         // The implementation is optional.
///     }
///
///     fn on_unload(&mut self, context: &mut PluginContext) {
///         // The method is called when the game/editor is about to shutdown.
///         // The implementation is optional.
///     }
///
///     fn update(&mut self, context: &mut PluginContext) {
///         // The method is called on every frame, it is guaranteed to have fixed update rate.
///         // The implementation is optional.
///     }
///
///     fn id(&self) -> Uuid {
///         // The method must return persistent type id.
///         // Use https://www.uuidgenerator.net/ to generate one.
///         Uuid::from_str("b9302812-81a7-48a5-89d2-921774d94943").unwrap()
///     }
///
///     fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) {
///         // The method is called when the main window receives an event from the OS.
///     }
/// }
/// ```
pub trait Plugin: 'static {
    /// The method is called when the plugin was just registered in the engine. The main use of the
    /// method is to register scripts and custom scene graph nodes in [`SerializationContext`].
    fn on_register(&mut self, #[allow(unused_variables)] context: PluginRegistrationContext) {}

    /// The method is called when the plugin is registered in game executor. It is guaranteed to be
    /// called once.
    ///
    /// # Important notes
    ///
    /// The method is **not** called if the plugin is running in the editor! Use
    /// [`Self::on_enter_play_mode`] instead.
    fn on_standalone_init(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// The method is called if the plugin running in the editor and the editor enters play mode.
    ///
    /// # Important notes
    ///
    /// The method replaces [`Self::on_standalone_init`] when the plugin runs in the editor! Use
    /// the method to obtain a handle to the scene being edited in the editor.
    fn on_enter_play_mode(
        &mut self,
        #[allow(unused_variables)] scene: Handle<Scene>,
        #[allow(unused_variables)] context: PluginContext,
    ) {
    }

    /// The method is called when the plugin is running inside the editor and it leaves
    /// "play mode".
    fn on_leave_play_mode(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// The method is called when the game/editor is about to shutdown.
    fn on_unload(&mut self, #[allow(unused_variables)] context: &mut PluginContext) {}

    /// Updates the plugin internals at fixed rate (see [`PluginContext::dt`] parameter for more
    /// info).
    fn update(&mut self, #[allow(unused_variables)] context: &mut PluginContext) {}

    /// The method must return persistent type id. The id is used for serialization, the engine
    /// saves the id into file (scene in most cases) and when you loading file it re-creates
    /// correct plugin using the id.
    ///
    /// # Important notes
    ///
    /// Do **not** use [`Uuid::new_v4`] or any other [`Uuid`] methods that generates ids, ids
    /// generated using these methods are **random** and are not suitable for serialization!
    ///
    /// # How to obtain UUID
    ///
    /// Use https://www.uuidgenerator.net/ to generate one.
    fn id(&self) -> Uuid;

    /// The method is called when the main window receives an event from the OS. The main use of
    /// the method is to respond to some external events, for example an event from keyboard or
    /// gamepad. See [`Event`] docs for more info.
    fn on_os_event(
        &mut self,
        #[allow(unused_variables)] event: &Event<()>,
        #[allow(unused_variables)] context: PluginContext,
    ) {
    }
}
