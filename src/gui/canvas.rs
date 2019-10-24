use rg3d_core::{
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::{
    node::UINode,
    widget::{
        Widget,
        WidgetBuilder,
        AsWidget,
    },
    Layout,
    UserInterface,
    Draw,
    draw::DrawingContext,
    Update
};


/// Allows user to directly set position and size of a node
pub struct Canvas {
    widget: Widget,
}

impl AsWidget for Canvas {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            widget: Default::default()
        }
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Update for Canvas {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl Layout for Canvas {
    fn measure_override(&self, ui: &UserInterface, _available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::new(
            std::f32::INFINITY,
            std::f32::INFINITY,
        );

        for child_handle in self.widget.children.iter() {
            ui.measure(*child_handle, size_for_child);
        }

        Vec2::ZERO
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        for child_handle in self.widget.children.iter() {
            let child = ui.nodes.borrow(*child_handle).widget();
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

impl Draw for Canvas {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
    }
}

pub struct CanvasBuilder {
    widget_builder: WidgetBuilder,
}

impl CanvasBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
        }
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        ui.add_node(UINode::Canvas(Canvas {
            widget: self.widget_builder.build()
        }))
    }
}
