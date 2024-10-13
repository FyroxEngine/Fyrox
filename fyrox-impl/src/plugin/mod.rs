// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Everything related to plugins. See [`Plugin`] docs for more info.

#![warn(missing_docs)]

pub mod dynamic;

use crate::{
    asset::manager::ResourceManager,
    core::{
        pool::Handle, reflect::Reflect, visitor::{Visit, VisitError},
    },
    engine::{
        task::TaskPoolHandler, AsyncSceneLoader, GraphicsContext, PerformanceStatistics, ScriptProcessor, SerializationContext
    },
    event::Event,
    gui::{
        constructor::WidgetConstructorContainer,
        inspector::editors::PropertyEditorDefinitionContainer, message::UiMessage, UiContainer,
    },
    scene::{Scene, SceneContainer},
};
use std::{
    any::Any,
    ops::{Deref, DerefMut},
    path::Path,
    sync::Arc,
};
use winit::event_loop::EventLoopWindowTarget;

/// A wrapper for various plugin types.
pub enum PluginContainer {
    /// Statically linked plugin. Such plugins are meant to be used in final builds, to maximize
    /// performance of the game.
    Static(Box<dyn Plugin>),
    /// Dynamically linked plugin. Such plugins are meant to be used in development mode for rapid
    /// prototyping.
    Dynamic(Box<dyn AbstractDynamicPlugin>),
}

/// Abstraction over different kind of plugins that can be reloaded on the fly (whatever it mean).
/// The instance is polled by engine with `is_reload_needed_now()` time to time. if it returns true,
/// then engine serializes current plugin state, then calls `unload()` and then calls `load()`
pub trait AbstractDynamicPlugin {
    /// returns human-redable short description of the plugin
    fn display_name(&self) -> String;

    /// engine polls is time to time to determine if it's time to reload plugin
    fn is_reload_needed_now(&self) -> bool;

    /// panics if not loaded
    fn as_loaded_ref(&self) -> &dyn Plugin;

    /// panics if not loaded
    fn as_loaded_mut(&mut self) -> &mut dyn Plugin;

    /// returns false if something bad happends during `reload`.
    fn is_loaded(&self) -> bool;

    /// it should call `fill_and_register` function on the new fresh plugin instance as
    /// soon as it's created, before making `is_loaded` return true.
    fn reload(&mut self, fill_and_register: &mut dyn FnMut(&mut dyn Plugin) -> Result<(), String>) -> Result<(), String>;
}

impl Deref for PluginContainer {
    type Target = dyn Plugin;

    fn deref(&self) -> &Self::Target {
        match self {
            PluginContainer::Static(plugin) => &**plugin,
            PluginContainer::Dynamic(plugin) => plugin.as_loaded_ref(),
        }
    }
}

impl DerefMut for PluginContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PluginContainer::Static(plugin) => &mut **plugin,
            PluginContainer::Dynamic(plugin) => plugin.as_loaded_mut(),
        }
    }
}

/// Contains plugin environment for the registration stage.
pub struct PluginRegistrationContext<'a> {
    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: &'a Arc<SerializationContext>,
    /// A reference to serialization context of the engine. See [`WidgetConstructorContainer`] for more
    /// info.
    pub widget_constructors: &'a Arc<WidgetConstructorContainer>,
    /// A reference to the resource manager instance of the engine. Could be used to register resource loaders.
    pub resource_manager: &'a ResourceManager,
}

/// Contains plugin environment.
pub struct PluginContext<'a, 'b> {
    /// A reference to scene container of the engine. You can add new scenes from [`Plugin`] methods
    /// by using [`SceneContainer::add`].
    pub scenes: &'a mut SceneContainer,

    /// A reference to the resource manager, it can be used to load various resources and manage
    /// them. See [`ResourceManager`] docs for more info.
    pub resource_manager: &'a ResourceManager,

    /// A reference to user interface container of the engine. The engine guarantees that there's
    /// at least one user interface exists. Use `context.user_interfaces.first()/first_mut()` to
    /// get a reference to it.
    pub user_interfaces: &'a mut UiContainer,

    /// A reference to the graphics_context, it contains a reference to the window and the current renderer.
    /// It could be [`GraphicsContext::Uninitialized`] if your application is suspended (possible only on
    /// Android; it is safe to call [`GraphicsContext::as_initialized_ref`] or [`GraphicsContext::as_initialized_mut`]
    /// on every other platform).
    pub graphics_context: &'a mut GraphicsContext,

    /// The time (in seconds) that passed since last call of a method in which the context was
    /// passed. It has fixed value that is defined by a caller (in most cases it is `Executor`).
    pub dt: f32,

    /// A reference to time accumulator, that holds remaining amount of time that should be used
    /// to update a plugin. A caller splits `lag` into multiple sub-steps using `dt` and thus
    /// stabilizes update rate. The main use of this variable, is to be able to reset `lag` when
    /// you doing some heavy calculations in a your game loop (i.e. loading a new level) so the
    /// engine won't try to "catch up" with all the time that was spent in heavy calculation.
    pub lag: &'b mut f32,

    /// A reference to serialization context of the engine. See [`SerializationContext`] for more
    /// info.
    pub serialization_context: &'a Arc<SerializationContext>,

    /// A reference to serialization context of the engine. See [`WidgetConstructorContainer`] for more
    /// info.
    pub widget_constructors: &'a Arc<WidgetConstructorContainer>,

    /// Performance statistics from the last frame.
    pub performance_statistics: &'a PerformanceStatistics,

    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,

    /// Script processor is used to run script methods in a strict order.
    pub script_processor: &'a ScriptProcessor,

    /// Asynchronous scene loader. It is used to request scene loading. See [`AsyncSceneLoader`] docs
    /// for usage example.
    pub async_scene_loader: &'a mut AsyncSceneLoader,

    /// Special field that associates main application event loop (not game loop) with OS-specific
    /// windows. It also can be used to alternate control flow of the application.
    pub window_target: Option<&'b EventLoopWindowTarget<()>>,

    /// Task pool for asynchronous task management.
    pub task_pool: &'a mut TaskPoolHandler,
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
        BasePlugin::as_any(self).downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    pub fn cast_mut<T: Plugin>(&mut self) -> Option<&mut T> {
        BasePlugin::as_any_mut(self).downcast_mut::<T>()
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
/// # use fyrox_impl::{
/// #     core::{pool::Handle}, core::visitor::prelude::*, core::reflect::prelude::*,
/// #     plugin::{Plugin, PluginContext, PluginRegistrationContext},
/// #     scene::Scene,
/// #     event::Event
/// # };
/// # use std::str::FromStr;
///
/// #[derive(Default, Visit, Reflect, Debug)]
/// struct MyPlugin {}
///
/// impl Plugin for MyPlugin {
///     fn on_deinit(&mut self, context: PluginContext) {
///         // The method is called when the plugin is disabling.
///         // The implementation is optional.
///     }
///
///     fn update(&mut self, context: &mut PluginContext) {
///         // The method is called on every frame, it is guaranteed to have fixed update rate.
///         // The implementation is optional.
///     }
///
///     fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) {
///         // The method is called when the main window receives an event from the OS.
///     }
/// }
/// ```
pub trait Plugin: BasePlugin + Visit + Reflect {
    /// The method is called when the plugin constructor was just registered in the engine. The main
    /// use of this method is to register scripts and custom scene graph nodes in [`SerializationContext`].
    fn register(&self, #[allow(unused_variables)] context: PluginRegistrationContext) {}

    /// This method is used to register property editors for your game types; to make them editable
    /// in the editor.
    fn register_property_editors(&self) -> PropertyEditorDefinitionContainer {
        PropertyEditorDefinitionContainer::empty()
    }

    /// This method is used to initialize your plugin.
    fn init(
        &mut self,
        #[allow(unused_variables)] scene_path: Option<&str>,
        #[allow(unused_variables)] context: PluginContext,
    ) {
    }

    /// This method is called when your plugin was re-loaded from a dynamic library. It could be used
    /// to restore some runtime state, that cannot be serialized. This method is called **only for
    /// dynamic plugins!** It is guaranteed to be called after all plugins were constructed, so the
    /// cross-plugins interactions are possible.
    fn on_loaded(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// The method is called before plugin will be disabled. It should be used for clean up, or some
    /// additional actions.
    fn on_deinit(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// Updates the plugin internals at fixed rate (see [`PluginContext::dt`] parameter for more
    /// info).
    fn update(&mut self, #[allow(unused_variables)] context: &mut PluginContext) {}

    /// The method is called when the main window receives an event from the OS. The main use of
    /// the method is to respond to some external events, for example an event from keyboard or
    /// gamepad. See [`Event`] docs for more info.
    fn on_os_event(
        &mut self,
        #[allow(unused_variables)] event: &Event<()>,
        #[allow(unused_variables)] context: PluginContext,
    ) {
    }

    /// The method is called when a graphics context was successfully created. It could be useful
    /// to catch the moment when it was just created and do something in response.
    fn on_graphics_context_initialized(
        &mut self,
        #[allow(unused_variables)] context: PluginContext,
    ) {
    }

    /// The method is called before the actual frame rendering. It could be useful to render off-screen
    /// data (render something to texture, that can be used later in the main frame).
    fn before_rendering(&mut self, #[allow(unused_variables)] context: PluginContext) {}

    /// The method is called when the current graphics context was destroyed.
    fn on_graphics_context_destroyed(&mut self, #[allow(unused_variables)] context: PluginContext) {
    }

    /// The method will be called when there is any message from main user interface instance
    /// of the engine.
    fn on_ui_message(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
        #[allow(unused_variables)] message: &UiMessage,
    ) {
    }

    /// This method is called when the engine starts loading a scene from the given `path`. It could
    /// be used to "catch" the moment when the scene is about to be loaded; to show a progress bar
    /// for example. See [`AsyncSceneLoader`] docs for usage example.
    fn on_scene_begin_loading(
        &mut self,
        #[allow(unused_variables)] path: &Path,
        #[allow(unused_variables)] context: &mut PluginContext,
    ) {
    }

    /// This method is called when the engine finishes loading a scene from the given `path`. Use
    /// this method if you need do something with a newly loaded scene. See [`AsyncSceneLoader`] docs
    /// for usage example.
    fn on_scene_loaded(
        &mut self,
        #[allow(unused_variables)] path: &Path,
        #[allow(unused_variables)] scene: Handle<Scene>,
        #[allow(unused_variables)] data: &[u8],
        #[allow(unused_variables)] context: &mut PluginContext,
    ) {
    }

    /// This method is called when the engine finishes loading a scene from the given `path` with
    /// some error. This method could be used to report any issues to a user.
    fn on_scene_loading_failed(
        &mut self,
        #[allow(unused_variables)] path: &Path,
        #[allow(unused_variables)] error: &VisitError,
        #[allow(unused_variables)] context: &mut PluginContext,
    ) {
    }
}
