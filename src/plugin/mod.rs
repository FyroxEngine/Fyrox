use crate::scene::Scene;
use crate::{
    core::uuid::Uuid,
    engine::{resource_manager::ResourceManager, SerializationContext},
    event::Event,
    renderer::Renderer,
    scene::SceneContainer,
};
use fyrox_core::pool::Handle;
use std::sync::Arc;

pub struct PluginRegistrationContext {
    pub serialization_context: Arc<SerializationContext>,
}

pub struct PluginContext<'a> {
    /// `true` if  the plugin running under the editor, false - otherwise.
    pub is_in_editor: bool,
    pub scenes: &'a mut SceneContainer,
    pub resource_manager: &'a ResourceManager,
    pub renderer: &'a mut Renderer,
    pub dt: f32,
    pub serialization_context: Arc<SerializationContext>,
}

pub trait Plugin: 'static {
    /// Called when plugin is first added to the engine.
    fn on_register(&mut self, context: PluginRegistrationContext);

    /// Called when the plugin is registered in game executor.
    ///
    /// # Important notes
    ///
    /// The method is **not** called if the plugin is running in the editor! Use
    /// [`Self::on_enter_play_mode`] instead.
    fn on_standalone_init(&mut self, context: PluginContext);

    /// Called if the plugin running in the editor and the editor enters play mode.
    /// The method replaces [`Self::on_standalone_init`] when the plugin runs in the editor.
    fn on_enter_play_mode(&mut self, scene: Handle<Scene>, context: PluginContext);

    /// Called if the plugin running in the editor and the editor leaves play mode.
    fn on_leave_play_mode(&mut self, context: PluginContext);

    fn on_unload(&mut self, context: &mut PluginContext);

    fn update(&mut self, context: &mut PluginContext);

    fn id(&self) -> Uuid;

    /// Called when there is an event from the OS.
    fn on_os_event(&mut self, _event: &Event<()>, _context: PluginContext) {}
}
