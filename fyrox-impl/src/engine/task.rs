//! Asynchronous task handler. See [`TaskPoolHandler`] for more info and usage examples.

use crate::plugin::PluginContainer;
use crate::{
    core::{
        pool::Handle,
        task::{AsyncTask, AsyncTaskResult, TaskPool},
        uuid::Uuid,
    },
    plugin::{Plugin, PluginContext},
    scene::{node::Node, Scene},
    script::{ScriptContext, ScriptTrait},
};
use fxhash::FxHashMap;
use std::sync::Arc;

pub(crate) type NodeTaskHandlerClosure = Box<
    dyn for<'a, 'b, 'c> Fn(
        Box<dyn AsyncTaskResult>,
        &mut dyn ScriptTrait,
        &mut ScriptContext<'a, 'b, 'c>,
    ),
>;

pub(crate) type PluginTaskHandler = Box<
    dyn for<'a, 'b> Fn(
        Box<dyn AsyncTaskResult>,
        &'a mut [PluginContainer],
        &mut PluginContext<'a, 'b>,
    ),
>;

pub(crate) struct NodeTaskHandler {
    pub(crate) scene_handle: Handle<Scene>,
    pub(crate) node_handle: Handle<Node>,
    pub(crate) script_index: usize,
    pub(crate) closure: NodeTaskHandlerClosure,
}

/// Asynchronous task handler is used as an executor for async functions (tasks), that in addition to them
/// has a closure, that will be called when a task is finished. The main use case for such tasks is to
/// off-thread a heavy task to one of background threads (from a thread pool on PC, or a microtask on
/// WebAssembly) and when it is done, incorporate its result in your game's state. A task and its
/// "on-complete" closure could be pretty much anything: procedural world generation + adding a
/// generated scene to the engine, asset loading + its instantiation, etc. It should be noted that
/// a task itself is executed asynchronously (in other thread), while a closure - synchronously,
/// just at the beginning of the next game loop iteration. This means, that you should never put
/// heavy tasks into the closure, otherwise it will result in quite notable stutters.
///
/// There are two main methods - [`TaskPoolHandler::spawn_plugin_task`] and [`TaskPoolHandler::spawn_script_task`].
/// They are somewhat similar, but the main difference between them is that the first one operates
/// on per plugin basis and the latter operates on scene node basis. This means that in case of
/// [`TaskPoolHandler::spawn_plugin_task`], it will accept an async task and when it is finished, it
/// will give you a result of the task and access to the plugin from which it was called, so you can
/// do some actions with the result. [`TaskPoolHandler::spawn_script_task`] does somewhat the same, but
/// on a scene node basis - when a task is done, the "on-complete" closure will be provided with a
/// wide context, allowing you to modify the caller's node state. See the docs for the respective
/// methods for more info.
pub struct TaskPoolHandler {
    task_pool: Arc<TaskPool>,
    plugin_task_handlers: FxHashMap<Uuid, PluginTaskHandler>,
    node_task_handlers: FxHashMap<Uuid, NodeTaskHandler>,
}

impl TaskPoolHandler {
    pub(crate) fn new(task_pool: Arc<TaskPool>) -> Self {
        Self {
            task_pool,
            plugin_task_handlers: Default::default(),
            node_task_handlers: Default::default(),
        }
    }

    /// Spawns a task represented by the `future`, that does something and then adds the result to
    /// a plugin it was called from using the `on_complete` closure.
    ///
    /// ## Example
    ///
    /// ```rust ,no_run
    /// # use fyrox_impl::plugin::{Plugin, PluginContext};
    /// # use fyrox_impl::core::visitor::prelude::*;
    /// # use fyrox_impl::core::reflect::prelude::*;
    /// # use std::{fs::File, io::Read};
    ///
    /// #[derive(Visit, Reflect, Debug)]
    /// struct MyGame {
    ///     data: Option<Vec<u8>>,
    /// }
    ///
    /// impl MyGame {
    ///     pub fn new(context: PluginContext) -> Self {
    ///         context.task_pool.spawn_plugin_task(
    ///             // Emulate heavy task by reading a potentially large file. The game will be fully
    ///             // responsive while it runs.
    ///             async move {
    ///                 let mut file = File::open("some/file.txt").unwrap();
    ///                 let mut data = Vec::new();
    ///                 file.read_to_end(&mut data).unwrap();
    ///                 data
    ///             },
    ///             // This closure is called when the future above has finished, but not immediately - on
    ///             // the next update iteration.
    ///             |data, game: &mut MyGame, _context| {
    ///                 // Store the data in the game instance.
    ///                 game.data = Some(data);
    ///             },
    ///         );
    ///
    ///         // Immediately return the new game instance with empty data.
    ///         Self { data: None }
    ///     }
    /// }
    ///
    /// impl Plugin for MyGame {
    ///     fn update(&mut self, _context: &mut PluginContext) {
    ///         // Do something with the data.
    ///         if let Some(data) = self.data.take() {
    ///             println!("The data is: {:?}", data);
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn spawn_plugin_task<F, T, P, C>(&mut self, future: F, on_complete: C)
    where
        F: AsyncTask<T>,
        T: AsyncTaskResult,
        P: Plugin,
        for<'a, 'b> C: Fn(T, &mut P, &mut PluginContext<'a, 'b>) + 'static,
    {
        let task_id = self.task_pool.spawn_with_result(future);
        self.plugin_task_handlers.insert(
            task_id,
            Box::new(move |result, plugins, context| {
                let plugin = plugins
                    .iter_mut()
                    .find_map(|p| p.cast_mut::<P>())
                    .expect("Plugin must be present!");
                let typed = result.downcast::<T>().expect("Types must match!");
                on_complete(*typed, plugin, context)
            }),
        );
    }

    /// Spawns a task represented by the `future`, that does something and then adds the result to
    /// a scene node's script using the `on_complete` closure. This method could be used to off-thread some
    /// heavy work from usual update routine (for example - pathfinding).
    ///
    /// ## Examples
    ///
    /// ```rust ,no_run
    /// # use fyrox_impl::{
    /// #     core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*, impl_component_provider},
    /// #     resource::model::{Model, ModelResourceExtension},
    /// #     script::{ScriptContext, ScriptTrait},
    /// # };
    /// # use fyrox_core::uuid_provider;
    /// #
    /// #[derive(Reflect, Visit, Default, Debug, Clone)]
    /// struct MyScript;
    ///
    /// # impl_component_provider!(MyScript);
    /// # uuid_provider!(MyScript = "f5ded79e-6101-4e23-b20d-48cbdb25d87a");
    ///
    /// impl ScriptTrait for MyScript {
    ///     fn on_start(&mut self, ctx: &mut ScriptContext) {
    ///         ctx.task_pool.spawn_script_task(
    ///             ctx.scene_handle,
    ///             ctx.handle,
    ///             ctx.script_index,
    ///             // Request loading of some heavy asset. It does not actually does the loading in the
    ///             // same routine, since asset loading itself is asynchronous, but we can't block the
    ///             // current thread on all support platforms to wait until the loading is done. So we
    ///             // have to use this approach to load assets on demand. Since every asset implements
    ///             // Future trait, it can be used directly as a future. Alternatively, you can use async
    ///             // move { } block here.
    ///             ctx.resource_manager.request::<Model>("path/to/model.fbx"),
    ///             // This closure will executed only when the upper future is done and only on the next
    ///             // update iteration.
    ///             |result, script: &mut MyScript, ctx| {
    ///                 if let Ok(model) = result {
    ///                     model.instantiate(&mut ctx.scene);
    ///                 }
    ///             },
    ///         );
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn spawn_script_task<F, T, C, S>(
        &mut self,
        scene_handle: Handle<Scene>,
        node_handle: Handle<Node>,
        script_index: usize,
        future: F,
        on_complete: C,
    ) where
        F: AsyncTask<T>,
        T: AsyncTaskResult,
        for<'a, 'b, 'c> C: Fn(T, &mut S, &mut ScriptContext<'a, 'b, 'c>) + 'static,
        S: ScriptTrait,
    {
        let task_id = self.task_pool.spawn_with_result(future);
        self.node_task_handlers.insert(
            task_id,
            NodeTaskHandler {
                scene_handle,
                node_handle,
                script_index,
                closure: Box::new(move |result, script, context| {
                    let script = script
                        .as_any_ref_mut()
                        .downcast_mut::<S>()
                        .expect("Types must match");
                    let result = result.downcast::<T>().expect("Types must match");
                    on_complete(*result, script, context);
                }),
            },
        );
    }

    /// Returns a reference to the underlying, low level task pool, that could be used to for special
    /// cases.
    #[inline]
    pub fn inner(&self) -> &Arc<TaskPool> {
        &self.task_pool
    }

    #[inline]
    pub(crate) fn pop_plugin_task_handler(&mut self, id: Uuid) -> Option<PluginTaskHandler> {
        self.plugin_task_handlers.remove(&id)
    }

    #[inline]
    pub(crate) fn pop_node_task_handler(&mut self, id: Uuid) -> Option<NodeTaskHandler> {
        self.node_task_handlers.remove(&id)
    }
}
