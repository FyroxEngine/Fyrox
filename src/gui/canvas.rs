use crate::core::{
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::{
    UINode,
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface,
    draw::DrawingContext,
    Control,
};

/// Allows user to directly set position and size of a node
pub struct Canvas {
    widget: Widget,
}

impl Control for Canvas {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }

    fn measure_override(&self, ui: &UserInterface, _available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::new(
            std::f32::INFINITY,
            std::f32::INFINITY,
        );

        for child_handle in self.widget.children.iter() {
            ui.get_node(*child_handle).measure(ui, size_for_child);
        }

        Vec2::ZERO
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        for child_handle in self.widget.children.iter() {
            let child = ui.nodes.borrow(*child_handle);
            child.arrange(ui, &Rect::new(
                child.widget().desired_local_position.get().x,
                child.widget().desired_local_position.get().y,
                child.widget().desired_size.get().x,
                child.widget().desired_size.get().y));
        }

        final_size
    }

    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        self.widget.draw(drawing_context)
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
        ui.add_node(Canvas {
            widget: self.widget_builder.build()
        })
    }
}
