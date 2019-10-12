use crate::gui::{
    EventSource,
    Layout,
    UserInterface,
    node::{UINode, UINodeKind},
    builder::{
        CommonBuilderFields,
        GenericNodeBuilder,
    },
    event::UIEvent,
};

use rg3d_core::{
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};

pub struct Canvas {}

impl Canvas {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for Canvas {
    fn measure_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, _available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::make(
            std::f32::INFINITY,
            std::f32::INFINITY,
        );

        let node = ui.nodes.borrow(self_handle);
        for child_handle in node.children.iter() {
            ui.measure(*child_handle, size_for_child);
        }

        Vec2::zero()
    }

    fn arrange_override(&self, self_handle: Handle<UINode>, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let node = ui.nodes.borrow(self_handle);
        for child_handle in node.children.iter() {
            let child = ui.nodes.borrow(*child_handle);
            let final_rect = Some(Rect::new(
                child.desired_local_position.get().x,
                child.desired_local_position.get().y,
                child.desired_size.get().x,
                child.desired_size.get().y));

            if let Some(rect) = final_rect {
                ui.arrange(*child_handle, &rect);
            }
        }


        final_size
    }
}

pub struct CanvasBuilder {
    common: CommonBuilderFields
}

impl Default for CanvasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasBuilder {
    pub fn new() -> Self {
        Self {
            common: CommonBuilderFields::new()
        }
    }

    impl_default_builder_methods!();

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        GenericNodeBuilder::new(UINodeKind::Canvas(Canvas::new()), self.common).build(ui)
    }
}

impl EventSource for Canvas {
    fn emit_event(&mut self) -> Option<UIEvent> {
        None
    }
}