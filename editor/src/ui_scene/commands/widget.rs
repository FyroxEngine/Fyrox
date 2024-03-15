use crate::command::{CommandContext, CommandTrait};
use crate::fyrox::{
    core::{algebra::Vector2, log::Log, pool::Handle, reflect::Reflect},
    graph::SceneGraphNode,
    gui::{UiNode, UserInterface},
};
use crate::ui_scene::commands::UiSceneContext;

#[derive(Debug)]
pub struct MoveWidgetCommand {
    node: Handle<UiNode>,
    old_position: Vector2<f32>,
    new_position: Vector2<f32>,
}

impl MoveWidgetCommand {
    pub fn new(
        node: Handle<UiNode>,
        old_position: Vector2<f32>,
        new_position: Vector2<f32>,
    ) -> Self {
        Self {
            node,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector2<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, ui: &mut UserInterface, position: Vector2<f32>) {
        ui.node_mut(self.node).set_desired_local_position(position);
    }
}

impl CommandTrait for MoveWidgetCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Widget".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let position = self.swap();
        self.set_position(context.get_mut::<UiSceneContext>().ui, position);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let position = self.swap();
        self.set_position(context.get_mut::<UiSceneContext>().ui, position);
    }
}

#[derive(Debug)]
pub struct RevertWidgetPropertyCommand {
    path: String,
    handle: Handle<UiNode>,
    value: Option<Box<dyn Reflect>>,
}

impl RevertWidgetPropertyCommand {
    pub fn new(path: String, handle: Handle<UiNode>) -> Self {
        Self {
            path,
            handle,
            value: None,
        }
    }
}

impl CommandTrait for RevertWidgetPropertyCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        format!("Revert {} Property", self.path)
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let child = &mut context.get_mut::<UiSceneContext>().ui.node_mut(self.handle);
        self.value = child.revert_inheritable_property(&self.path);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        // If the property was modified, then simply set it to previous value to make it modified again.
        if let Some(old_value) = self.value.take() {
            let mut old_value = Some(old_value);
            context
                .get_mut::<UiSceneContext>()
                .ui
                .node_mut(self.handle)
                .as_reflect_mut(&mut |node| {
                    node.set_field_by_path(&self.path, old_value.take().unwrap(), &mut |result| {
                        if result.is_err() {
                            Log::err(format!(
                                "Failed to revert property {}. Reason: no such property!",
                                self.path
                            ))
                        }
                    });
                })
        }
    }
}
