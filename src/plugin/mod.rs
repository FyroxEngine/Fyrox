//! Everything related to plugins. See [`Plugin`] docs for more info.

#![warn(missing_docs)]

use crate::{
    core::{pool::Handle, uuid::Uuid},
    engine::{resource_manager::ResourceManager, SerializationContext},
    event::Event,
    event_loop::ControlFlow,
    gui::{message::UiMessage, UserInterface},
    renderer::Renderer,
    scene::{Scene, SceneContainer},
    window::Window,
};
use fyrox_sound::engine::SoundEngine;
use std::sync::Mutex;
use std::{any::Any, sync::Arc};

/// Plugin constructor is a first step of 2-stage plugin initialization. It is responsible for plugin script
/// registration and for creating actual plugin instance.
///
/// # Details
///
/// Why there is a need in 2-state initialization? The editor requires it, it is interested only in plugin
/// scripts so editor does not create any plugin instances, it just uses [Self::register] to obtain information
/// about scripts.  
pub trait PluginConstructor {
    /// The method is called when the plugin constructor was just registered in the engine. The main use of the
    /// method is to register scripts and custom scene graph nodes in [`SerializationContext`].
    fn register(&self, #[allow(unused_variables)] context: PluginRegistrationContext) {}

    /// The method is called when the engine creates plugin instances. It allows to create initialized plugin
    /// instance.
    ///
    /// # Important notes
    ///
    /// `override_scene` is a handle to an override scene that is currently active. It is used only in editor
    /// when you enter play mode, on other cases it is `Handle::NONE`.
    fn create_instance(
        &self,
        #[allow(unused_variables)] override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin>;
}

/// Contains plugin environment for the registration stage.
pub struct PluginRegistrationContext<'a> {
    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: &'a Arc<SerializationContext>,
}

/// A small wrapper that provides limited access to inner sound engine.
pub struct SoundEngineHelper<'a> {
    pub(crate) engine: &'a Arc<Mutex<SoundEngine>>,
}

impl<'a> SoundEngineHelper<'a> {
    /// Sets master gain of the sound engine. Can be used to control overall gain of all sound
    /// scenes at once.
    pub fn set_sound_gain(&mut self, gain: f32) {
        self.engine.lock().unwrap().set_master_gain(gain);
    }

    /// Returns master gain of the sound engine.
    pub fn sound_gain(&self) -> f32 {
        self.engine.lock().unwrap().master_gain()
    }
}

/// Contains plugin environment.
pub struct PluginContext<'a> {
    /// A reference to scene container of the engine. You can add new scenes from [`Plugin`] methods
    /// by using [`SceneContainer::add`].
    pub scenes: &'a mut SceneContainer,

    /// A reference to the resource manager, it can be used to load various resources and manage
    /// them. See [`ResourceManager`] docs for more info.
    pub resource_manager: &'a ResourceManager,

    /// A reference to user interface instance.
    pub user_interface: &'a mut UserInterface,

    /// A reference to the renderer, it can be used to add custom render passes (for example to
    /// render custom effects and so on).
    pub renderer: &'a mut Renderer,

    /// The time (in seconds) that passed since last call of a method in which the context was
    /// passed.
    pub dt: f32,

    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: &'a Arc<SerializationContext>,

    /// A reference to the main application window.
    pub window: &'a Window,

    /// Sound engine allows you to change global sound parameters, such as master gain, etc.
    pub sound_engine: SoundEngineHelper<'a>,
}

/// Base plugin automatically implements type casting for plugins.
pub trait BasePlugin: Any + 'static {
    /// Returns a reference to Any trait. It is used for type casting.
    fn as_any(&self) -> &dyn Any;

    /// Returns a reference to Any trait. It is used for type casting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> BasePlugin for T
where
    T: Any + Plugin + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl dyn Plugin {
    /// Performs downcasting to a particular type.
    pub fn cast<T: Plugin>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    pub fn cast_mut<T: Plugin>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}

/// Plugin is a convenient interface that allow you to extend engine's functionality.
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
///     core::{pool::Handle, uuid::{uuid,Uuid}},
///     plugin::{Plugin, PluginContext, PluginRegistrationContext},
///     scene::Scene,
///     event::Event
/// };
/// use std::str::FromStr;
/// use fyrox::event_loop::ControlFlow;
///
/// #[derive(Default)]
/// struct MyPlugin {}
///
/// impl Plugin for MyPlugin {
///     fn on_deinit(&mut self, context: PluginContext) {
///         // The method is called when the plugin is disabling.
///         // The implementation is optional.
///     }
///
///     fn update(&mut self, context: &mut PluginContext, control_flow: &mut ControlFlow) {
///         // The method is called on every frame, it is guaranteed to have fixed update rate.
///         // The implementation is optional.
///     }
///
///     fn id(&self) -> Uuid {
///         // The method must return persistent type id.
///         // Use https://www.uuidgenerator.net/ to generate one.
///         uuid!("b9302812-81a7-48a5-89d2-921774d94943")
///     }
///
///     fn on_os_event(&mut self, event: &Event<()>, context: PluginContext, control_flow: &mut ControlFlow) {
///         // The method is called when the main window receives an event from the OS.
///     }
/// }
/// ```
pub trait Plugin: BasePlugin {
    /// The method is called before plugin will be disabled. It should be used for clean up, or some
    /// additional actions.
    fn on_deinit(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// Updates the plugin internals at fixed rate (see [`PluginContext::dt`] parameter for more
    /// info).
    fn update(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
        #[allow(unused_variables)] control_flow: &mut ControlFlow,
    ) {
    }

    /// The method must return persistent type id. It is used to link scripts and plugins, it is
    /// possible to have multiple plugins and each script instance must be able to find correct
    /// plugin, it is done by comparing UUIDs.
    ///
    /// # Important notes
    ///
    /// Do **not** use [`Uuid::new_v4`] or any other [`Uuid`] methods that generates ids, id must
    /// be persistent until application running.
    ///
    /// # How to obtain UUID
    ///
    /// Use <https://www.uuidgenerator.net/> to generate one.
    fn id(&self) -> Uuid;

    /// The method is called when the main window receives an event from the OS. The main use of
    /// the method is to respond to some external events, for example an event from keyboard or
    /// gamepad. See [`Event`] docs for more info.
    fn on_os_event(
        &mut self,
        #[allow(unused_variables)] event: &Event<()>,
        #[allow(unused_variables)] context: PluginContext,
        #[allow(unused_variables)] control_flow: &mut ControlFlow,
    ) {
    }

    /// The method will be called when there is any message from main user interface instance
    /// of the engine.
    fn on_ui_message(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
        #[allow(unused_variables)] message: &UiMessage,
        #[allow(unused_variables)] control_flow: &mut ControlFlow,
    ) {
    }
}
