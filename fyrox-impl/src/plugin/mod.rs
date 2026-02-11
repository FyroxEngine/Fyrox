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

pub mod dylib;
pub mod error;

use crate::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        define_as_any_trait, dyntype::DynTypeConstructorContainer, log::Log, pool::Handle,
        reflect::Reflect, variable::try_inherit_properties, visitor::error::VisitError,
        visitor::Visit,
    },
    engine::{
        input::InputState, task::TaskPoolHandler, ApplicationLoopController, GraphicsContext,
        PerformanceStatistics, ScriptProcessor, SerializationContext,
    },
    event::Event,
    graph::NodeMapping,
    gui::{
        constructor::WidgetConstructorContainer,
        inspector::editors::PropertyEditorDefinitionContainer, message::UiMessage, UiContainer,
        UserInterface,
    },
    plugin::error::{GameError, GameResult},
    resource::model::Model,
    scene::{graph::NodePool, navmesh, Scene, SceneContainer, SceneLoader},
};
use std::path::PathBuf;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A wrapper for various plugin types.
pub enum PluginContainer {
    /// Statically linked plugin. Such plugins are meant to be used in final builds, to maximize
    /// performance of the game.
    Static(Box<dyn Plugin>),
    /// Dynamically linked plugin. Such plugins are meant to be used in development mode for rapid
    /// prototyping.
    Dynamic(Box<dyn DynamicPlugin>),
}

/// Abstraction over different kind of plugins that can be reloaded on the fly (whatever it mean).
/// The instance is polled by engine with `is_reload_needed_now()` time to time. if it returns true,
/// then engine serializes current plugin state, then calls `unload()` and then calls `load()`
pub trait DynamicPlugin {
    /// returns human-redable short description of the plugin
    fn display_name(&self) -> String;

    /// engine polls is time to time to determine if it's time to reload plugin
    fn is_reload_needed_now(&self) -> bool;

    /// panics if not loaded
    fn as_loaded_ref(&self) -> &dyn Plugin;

    /// panics if not loaded
    fn as_loaded_mut(&mut self) -> &mut dyn Plugin;

    /// returns false if something bad happends during `reload`.
    /// has no much use except prevention of error spamming
    fn is_loaded(&self) -> bool;

    /// called before saving state and detaching related objects
    fn prepare_to_reload(&mut self) {}

    /// called after plugin-related objects are detached
    /// `fill_and_register` callback exposes plugin instance to engine to register constructors and restore the state
    /// callback approach allows plugins to do some necessary actions right after plugin is registed
    fn reload(
        &mut self,
        fill_and_register: &mut dyn FnMut(&mut dyn Plugin) -> Result<(), String>,
    ) -> Result<(), String>;
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
    /// A container with constructors for dynamic types. See [`DynTypeConstructorContainer`] for more
    /// info.
    pub dyn_type_constructors: &'a Arc<DynTypeConstructorContainer>,
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

    /// A container with constructors for dynamic types. See [`DynTypeConstructorContainer`] for more
    /// info.
    pub dyn_type_constructors: &'a Arc<DynTypeConstructorContainer>,

    /// Performance statistics from the last frame.
    pub performance_statistics: &'a PerformanceStatistics,

    /// Amount of time (in seconds) that passed from creation of the engine. Keep in mind, that
    /// this value is **not** guaranteed to match real time. A user can change delta time with
    /// which the engine "ticks" and this delta time affects elapsed time.
    pub elapsed_time: f32,

    /// Script processor is used to run script methods in a strict order.
    pub script_processor: &'a ScriptProcessor,

    /// Special field that associates the main application event loop (not game loop) with OS-specific
    /// windows. It also can be used to alternate control flow of the application. `None` if the
    /// engine is running in headless mode.
    pub loop_controller: ApplicationLoopController<'b>,

    /// Task pool for asynchronous task management.
    pub task_pool: &'a mut TaskPoolHandler,

    /// A stored state of most common input events. It is used a "shortcut" in cases where event-based
    /// approach is too verbose. It may be useful in simple scenarios where you just need to know
    /// if a button (on keyboard, mouse) was pressed and do something.
    ///
    /// **Important:** this structure does not track from which device the corresponding event has
    /// come from, if you have more than one keyboard and/or mouse, use event-based approach instead!
    pub input_state: &'a InputState,
}

impl<'a, 'b> PluginContext<'a, 'b> {
    /// Spawns an asynchronous task that tries to load a user interface from the given path.
    /// When the task is completed, the specified callback is called that can be used to
    /// modify the UI. The loaded UI must be registered in the engine, otherwise it will be
    /// discarded.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use fyrox_impl::{
    /// #     core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    /// #     event::Event,
    /// #     plugin::{error::GameResult, Plugin, PluginContext, PluginRegistrationContext},
    /// #     scene::Scene,
    /// # };
    /// # use std::str::FromStr;
    ///
    /// #[derive(Default, Visit, Reflect, Debug)]
    /// #[reflect(non_cloneable)]
    /// struct MyGame {}
    ///
    /// impl Plugin for MyGame {
    ///     fn init(&mut self, _scene_path: Option<&str>, mut ctx: PluginContext) -> GameResult {
    ///         ctx.load_ui("data/my.ui", |result, game: &mut MyGame, mut ctx| {
    ///             // The loaded UI must be registered in the engine.
    ///             *ctx.user_interfaces.first_mut() = result?;
    ///             Ok(())
    ///         });
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn load_ui<U, P, C>(&mut self, path: U, callback: C)
    where
        U: Into<PathBuf>,
        P: Plugin,
        for<'c, 'd> C: Fn(Result<UserInterface, VisitError>, &mut P, &mut PluginContext<'c, 'd>) -> GameResult
            + 'static,
    {
        self.task_pool.spawn_plugin_task(
            UserInterface::load_from_file(
                path.into(),
                self.widget_constructors.clone(),
                self.dyn_type_constructors.clone(),
                self.resource_manager.clone(),
            ),
            callback,
        );
    }

    /// Tries to load a game scene at the given path.
    ///
    /// This method has a special flag `is_derived` which dictates how to load the scene:
    ///
    /// - `false` - the scene is loaded as-is and returned to the caller. Use this option if you
    ///   don't want to use built-in "saved game" system. See the next option for details.
    /// - `true`, then the requested scene is loaded and a new model resource is
    ///   registered in the resource manager that references the scene source file. Then the loaded
    ///   scene is _cloned_ and all its nodes links to their originals in the source file. Then this
    ///   cloned and processed scene ("derived") is returned. This process essentially links the
    ///   scene to its source file, so when the derived scene is saved to disk, it does not save all
    ///   its content, but only changes from the original scene. Derived scenes are used to create
    ///   saved games. Keep in mind, if you've created a derived scene and saved it, you must load
    ///   this saved game with `is_derived` set to `false`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use fyrox_impl::{
    /// #     core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    /// #     event::Event,
    /// #     plugin::{error::GameResult, Plugin, PluginContext, PluginRegistrationContext},
    /// #     scene::Scene,
    /// # };
    /// # use std::str::FromStr;
    /// #
    /// #[derive(Default, Visit, Reflect, Debug)]
    /// #[reflect(non_cloneable)]
    /// struct MyGame {}
    ///
    /// impl MyGame {
    ///     fn on_scene_loading_result(
    ///         &mut self,
    ///         result: Result<(Scene, Vec<u8>), VisitError>,
    ///         ctx: &mut PluginContext,
    ///     ) -> GameResult {
    ///         let (scene, _raw_data) = result?;
    ///         // Register the scene.
    ///         ctx.scenes.add(scene);
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl Plugin for MyGame {
    ///     fn init(&mut self, scene_path: Option<&str>, mut ctx: PluginContext) -> GameResult {
    ///         ctx.load_scene(
    ///             scene_path.unwrap_or("data/scene.rgs"),
    ///             false, // See the docs for details.
    ///             |result, game: &mut MyGame, ctx| game.on_scene_loading_result(result, ctx),
    ///         );
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn load_scene<U, P, C>(&mut self, path: U, is_derived: bool, callback: C)
    where
        U: Into<PathBuf>,
        P: Plugin,
        for<'c, 'd> C: Fn(
                Result<(Scene, Vec<u8>), VisitError>,
                &mut P,
                &mut PluginContext<'c, 'd>,
            ) -> GameResult
            + 'static,
    {
        let path = path.into();

        let serialization_context = self.serialization_context.clone();
        let dyn_type_constructors = self.dyn_type_constructors.clone();
        let resource_manager = self.resource_manager.clone();
        let uuid = resource_manager.find::<Model>(&path).resource_uuid();
        let io = resource_manager.resource_io();

        self.task_pool.spawn_plugin_task(
            async move {
                match SceneLoader::from_file(
                    path.clone(),
                    io.as_ref(),
                    serialization_context,
                    dyn_type_constructors,
                    resource_manager.clone(),
                )
                .await
                {
                    Ok((loader, data)) => Ok((loader.finish().await, data)),
                    Err(e) => Err(e),
                }
            },
            move |result, plugin, ctx| {
                match result {
                    Ok((mut scene, data)) => {
                        if is_derived {
                            let model = ctx.resource_manager.find_uuid::<Model>(uuid);
                            // Create a resource, that will point to the scene we've loaded the
                            // scene from and force scene nodes to inherit data from them.
                            let data = Model {
                                mapping: NodeMapping::UseHandles,
                                // We have to create a full copy of the scene, because otherwise
                                // some methods (`Base::root_resource` in particular) won't work
                                // correctly.
                                scene: scene.clone_one_to_one().0,
                            };
                            model.header().state.commit_ok(data);

                            for (handle, node) in scene.graph.pair_iter_mut() {
                                node.set_inheritance_data(handle, model.clone());
                            }

                            // Reset modified flags in every inheritable property of the scene.
                            // Except nodes, they're inherited in a separate place.
                            (&mut scene as &mut dyn Reflect).apply_recursively_mut(
                                &mut |object| {
                                    let type_id = (*object).type_id();
                                    if type_id != TypeId::of::<NodePool>() {
                                        object.as_inheritable_variable_mut(&mut |variable| {
                                            if let Some(variable) = variable {
                                                variable.reset_modified_flag();
                                            }
                                        });
                                    }
                                },
                                &[
                                    TypeId::of::<UntypedResource>(),
                                    TypeId::of::<navmesh::Container>(),
                                ],
                            )
                        } else {
                            // Take scene data from the source scene.
                            if let Some(source_asset) =
                                scene.graph[scene.graph.get_root()].root_resource()
                            {
                                let source_asset_ref = source_asset.data_ref();
                                let source_scene_ref = &source_asset_ref.scene;
                                Log::verify(try_inherit_properties(
                                    &mut scene,
                                    source_scene_ref,
                                    &[
                                        TypeId::of::<NodePool>(),
                                        TypeId::of::<UntypedResource>(),
                                        TypeId::of::<navmesh::Container>(),
                                    ],
                                ));
                            }
                        }

                        callback(Ok((scene, data)), plugin, ctx)
                    }
                    Err(error) => callback(Err(error), plugin, ctx),
                }
            },
        );
    }
}

define_as_any_trait!(PluginAsAny => Plugin);

impl dyn Plugin {
    /// Performs downcasting to a particular type.
    pub fn cast<T: Plugin>(&self) -> Option<&T> {
        PluginAsAny::as_any(self).downcast_ref::<T>()
    }

    /// Performs downcasting to a particular type.
    pub fn cast_mut<T: Plugin>(&mut self) -> Option<&mut T> {
        PluginAsAny::as_any_mut(self).downcast_mut::<T>()
    }
}

/// Plugin is a convenient interface that allow you to extend engine's functionality.
///
/// # Example
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::{pool::Handle}, core::visitor::prelude::*, core::reflect::prelude::*,
/// #     plugin::{Plugin, PluginContext, PluginRegistrationContext, error::GameResult},
/// #     scene::Scene,
/// #     event::Event
/// # };
/// # use std::str::FromStr;
///
/// #[derive(Default, Visit, Reflect, Debug)]
/// #[reflect(non_cloneable)]
/// struct MyPlugin {}
///
/// impl Plugin for MyPlugin {
///     fn on_deinit(&mut self, context: PluginContext) -> GameResult {
///         // The method is called when the plugin is disabling.
///         // The implementation is optional.
///         Ok(())
///     }
///
///     fn update(&mut self, context: &mut PluginContext) -> GameResult {
///         // The method is called on every frame, it is guaranteed to have fixed update rate.
///         // The implementation is optional.
///         Ok(())
///     }
///
///     fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) -> GameResult {
///         // The method is called when the main window receives an event from the OS.
///         Ok(())
///     }
/// }
/// ```
///
/// # Error Handling
///
/// Every plugin method returns [`GameResult`] (which is a simple wrapper over `Result<(), GameError>`),
/// this helps to reduce the amount of boilerplate code related to error handling. There are a number
/// of errors that can be automatically handled via `?` operator. All supported error types listed
/// in [`error::GameError`] enum.
///
/// The following code snippet shows the most common use cases for error handling:
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::{err, pool::Handle, reflect::prelude::*, visitor::prelude::*},
/// #     event::Event,
/// #     graph::SceneGraph,
/// #     plugin::{error::GameResult, Plugin, PluginContext, PluginRegistrationContext},
/// #     scene::{node::Node, Scene},
/// # };
/// # use std::str::FromStr;
/// #[derive(Default, Visit, Reflect, Debug)]
/// #[reflect(non_cloneable)]
/// struct MyPlugin {
///     scene: Handle<Scene>,
///     player: Handle<Node>,
/// }
///
/// impl Plugin for MyPlugin {
///     fn update(&mut self, context: &mut PluginContext) -> GameResult {
///         // 1. This is the old approach.
///         match context.scenes.try_get(self.scene) {
///             Ok(scene) => match scene.graph.try_get(self.player) {
///                 Ok(player) => {
///                     println!("Player name is: {}", player.name());
///                 }
///                 Err(error) => {
///                     err!("Unable to borrow the player. Reason: {error}")
///                 }
///             },
///             Err(error) => {
///                 err!("Unable to borrow the scene. Reason: {error}")
///             }
///         }
///
///         // 2. This is the same code as above, but with shortcuts for easier error handling.
///         // Message report is will be something like this:
///         // `An error occurred during update plugin method call. Reason: <error message>`.
///         let scene = context.scenes.try_get(self.scene)?;
///         let player = scene.graph.try_get(self.player)?;
///         println!("Player name is: {}", player.name());
///
///         Ok(())
///     }
/// }
/// ```
pub trait Plugin: PluginAsAny + Visit + Reflect {
    /// The method is called when the plugin constructor was just registered in the engine. The main
    /// use of this method is to register scripts and custom scene graph nodes in [`SerializationContext`].
    fn register(
        &self,
        #[allow(unused_variables)] context: PluginRegistrationContext,
    ) -> GameResult {
        Ok(())
    }

    /// This method is used to register property editors for your game types; to make them editable
    /// in the editor.
    fn register_property_editors(
        &self,
        #[allow(unused_variables)] editors: Arc<PropertyEditorDefinitionContainer>,
    ) {
    }

    /// This method is used to initialize your plugin.
    fn init(
        &mut self,
        #[allow(unused_variables)] scene_path: Option<&str>,
        #[allow(unused_variables)] context: PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// This method is called when your plugin was re-loaded from a dynamic library. It could be used
    /// to restore some runtime state, that cannot be serialized. This method is called **only for
    /// dynamic plugins!** It is guaranteed to be called after all plugins were constructed, so the
    /// cross-plugins interactions are possible.
    fn on_loaded(&mut self, #[allow(unused_variables)] context: PluginContext) -> GameResult {
        Ok(())
    }

    /// The method is called before plugin will be disabled. It should be used for clean up, or some
    /// additional actions.
    fn on_deinit(&mut self, #[allow(unused_variables)] context: PluginContext) -> GameResult {
        Ok(())
    }

    /// Updates the plugin internals at fixed rate (see [`PluginContext::dt`] parameter for more
    /// info).
    fn update(&mut self, #[allow(unused_variables)] context: &mut PluginContext) -> GameResult {
        Ok(())
    }

    /// called after all Plugin and Script updates
    fn post_update(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// The method is called when the main window receives an event from the OS. The main use of
    /// the method is to respond to some external events, for example an event from keyboard or
    /// gamepad. See [`Event`] docs for more info.
    fn on_os_event(
        &mut self,
        #[allow(unused_variables)] event: &Event<()>,
        #[allow(unused_variables)] context: PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// The method is called when a graphics context was successfully created. It could be useful
    /// to catch the moment when it was just created and do something in response.
    fn on_graphics_context_initialized(
        &mut self,
        #[allow(unused_variables)] context: PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// The method is called before the actual frame rendering. It could be useful to render off-screen
    /// data (render something to texture, that can be used later in the main frame).
    fn before_rendering(
        &mut self,
        #[allow(unused_variables)] context: PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// The method is called when the current graphics context was destroyed.
    fn on_graphics_context_destroyed(
        &mut self,
        #[allow(unused_variables)] context: PluginContext,
    ) -> GameResult {
        Ok(())
    }

    /// The method will be called when there is any message from a user interface (UI) instance
    /// of the engine. Use `ui_handle` parameter to find out from which UI the message has come
    /// from.
    fn on_ui_message(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
        #[allow(unused_variables)] message: &UiMessage,
        #[allow(unused_variables)] ui_handle: Handle<UserInterface>,
    ) -> GameResult {
        Ok(())
    }

    /// This method is called when a game error has occurred, allowing you to perform some
    /// specific action to react to it (for example - to show an error message UI in your game).
    ///
    /// ## Important notes
    ///
    /// This method is called at the end of the current frame, and before that, the engine collects
    /// all the errors into a queue and then processes them one by one. This means that this method
    /// won't be called immediately when an error was returned by any of your plugin or script methods,
    /// but instead the processing will be delayed to the end of the frame.
    ///
    /// The error passed by a reference here instead of by-value, because there could be multiple
    /// plugins that can handle the error. This might seem counterintuitive, but remember that
    /// [`GameError`] can occur during script execution, which is not a part of a plugin and its
    /// methods executed separately, outside the plugin routines.
    ///
    /// ## Error handling
    ///
    /// This method should return `true` if the error was handled and no logging is needed, otherwise
    /// it should return `false` and in this case, the error will be logged by the engine. When
    /// `true` is returned by the plugin, the error won't be passed to any other plugins. By default,
    /// this method returns `false`, which means that it does not handle any errors and the engine
    /// will log the errors as usual.
    fn on_game_error(
        &mut self,
        #[allow(unused_variables)] context: &mut PluginContext,
        #[allow(unused_variables)] error: &GameError,
    ) -> bool {
        false
    }
}
