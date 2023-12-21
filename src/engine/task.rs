#![allow(missing_docs)] // TODO

use crate::{
    core::{pool::Handle, task::TaskPool, uuid::Uuid},
    plugin::{Plugin, PluginContext},
    scene::{node::Node, Scene},
    script::ScriptContext,
};
use fxhash::FxHashMap;
use std::{any::Any, future::Future, sync::Arc};

pub type NodeTaskHandlerClosure =
    Box<dyn for<'a, 'b, 'c> Fn(Box<dyn Any + Send>, &mut ScriptContext<'a, 'b, 'c>)>;

pub type PluginTaskHandler = Box<
    dyn for<'a, 'b> Fn(Box<dyn Any + Send>, &'a mut [Box<dyn Plugin>], &mut PluginContext<'a, 'b>),
>;

pub struct NodeTaskHandler {
    pub scene_handle: Handle<Scene>,
    pub node_handle: Handle<Node>,
    pub closure: NodeTaskHandlerClosure,
}

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

    #[inline]
    pub fn spawn_plugin_task<F, T, P, C>(&mut self, future: F, on_complete: C)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
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
                let typed =
                    Box::<dyn Any + Send>::downcast::<T>(result).expect("Types must match!");
                on_complete(*typed, plugin, context)
            }),
        );
    }

    #[inline]
    pub fn spawn_node_task<F, T, C>(
        &mut self,
        scene_handle: Handle<Scene>,
        node_handle: Handle<Node>,
        future: F,
        on_complete: C,
    ) where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        for<'a, 'b, 'c> C: Fn(T, &mut ScriptContext<'a, 'b, 'c>) + 'static,
    {
        let task_id = self.task_pool.spawn_with_result(future);
        self.node_task_handlers.insert(
            task_id,
            NodeTaskHandler {
                scene_handle,
                node_handle,
                closure: Box::new(move |result, context| {
                    let typed =
                        Box::<dyn Any + Send>::downcast::<T>(result).expect("Types must match");
                    on_complete(*typed, context)
                }),
            },
        );
    }

    #[inline]
    pub fn inner(&self) -> &Arc<TaskPool> {
        &self.task_pool
    }

    #[inline]
    pub(crate) fn pop_plugin_task(&mut self) -> Option<(Box<dyn Any + Send>, PluginTaskHandler)> {
        self.task_pool.next_task_result().and_then(|result| {
            self.plugin_task_handlers
                .remove(&result.id)
                .map(|handler| (result.payload, handler))
        })
    }

    #[inline]
    pub(crate) fn pop_node_task(&mut self) -> Option<(Box<dyn Any + Send>, NodeTaskHandler)> {
        self.task_pool.next_task_result().and_then(|result| {
            self.node_task_handlers
                .remove(&result.id)
                .map(|handler| (result.payload, handler))
        })
    }
}
