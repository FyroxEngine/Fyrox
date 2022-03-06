use crate::engine::SerializationContext;
use crate::{
    core::{inspect::Inspect, uuid::Uuid, visitor::Visit},
    engine::resource_manager::ResourceManager,
    gui::UserInterface,
    renderer::Renderer,
    scene::SceneContainer,
};
use std::sync::Arc;

pub struct PluginRegistrationContext {
    pub serialization_context: Arc<SerializationContext>,
}

pub struct PluginContext<'a> {
    /// `true` if  the plugin running under the editor, false - otherwise.
    pub is_in_editor: bool,
    pub scenes: &'a mut SceneContainer,
    pub ui: &'a mut UserInterface,
    pub resource_manager: &'a ResourceManager,
    pub renderer: &'a mut Renderer,
    pub dt: f32,
    pub serialization_context: Arc<SerializationContext>,
}

pub trait Plugin: Visit + Inspect + 'static {
    fn on_register(&mut self, context: PluginRegistrationContext);

    fn on_init(&mut self, context: PluginContext);

    fn on_unload(&mut self, context: &mut PluginContext);

    fn update(&mut self, context: &mut PluginContext);

    fn id(&self) -> Uuid;
}
