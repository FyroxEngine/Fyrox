use crate::{
    core::{
        pool::Handle,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    gui::{
        UINode,
        widget::{
            Widget,
            WidgetBuilder,
        },
        UserInterface,
        draw::DrawingContext,
        Control,
        ControlTemplate,
        UINodeContainer,
        Builder
    }
};
use std::collections::HashMap;

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

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {

    }

    fn measure_override(&self, ui: &UserInterface, _available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::new(
            std::f32::INFINITY,
            std::f32::INFINITY,
        );

        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).measure(ui, size_for_child);
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

    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

impl Canvas {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget
        }
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
}

impl Builder for CanvasBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        ui.add_node(Box::new(Canvas {
            widget: self.widget_builder.build()
        }))
    }
}