use crate::ui_scene::commands::{UiCommand, UiSceneContext};
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    gui::{UiNode, UserInterface},
};

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

impl UiCommand for MoveWidgetCommand {
    fn name(&mut self, _context: &UiSceneContext) -> String {
        "Move Widget".to_owned()
    }

    fn execute(&mut self, context: &mut UiSceneContext) {
        let position = self.swap();
        self.set_position(context.ui, position);
    }

    fn revert(&mut self, context: &mut UiSceneContext) {
        let position = self.swap();
        self.set_position(context.ui, position);
    }
}
