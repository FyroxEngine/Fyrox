use crate::{
    core::{
        pool::Handle,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    UINode,
    draw::{
        CommandKind,
        DrawingContext,
    },
    Thickness,
    UserInterface,
    widget::{
        Widget,
        WidgetBuilder,
    },
    draw::CommandTexture,
    Control,
    ControlTemplate,
    UINodeContainer,
    Builder,
};
use std::{
    any::Any,
    collections::HashMap,
};

pub struct Border {
    widget: Widget,
    stroke_thickness: Thickness,
}

pub struct BorderBuilder {
    widget_builder: WidgetBuilder,
    stroke_thickness: Option<Thickness>,
}

impl BorderBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            stroke_thickness: None,
        }
    }

    pub fn with_stroke_thickness(mut self, stroke_thickness: Thickness) -> Self {
        self.stroke_thickness = Some(stroke_thickness);
        self
    }
}

impl Builder for BorderBuilder {
    fn build(self, ui: &mut dyn UINodeContainer) -> Handle<UINode> {
        let style = self.widget_builder.style.clone();

        let mut border = Border {
            widget: self.widget_builder.build(),
            stroke_thickness: self.stroke_thickness.unwrap_or_else(|| Thickness::uniform(1.0)),
        };

        if let Some(style) = style {
            border.apply_style(style);
        }

        ui.add_node(Box::new(border))
    }
}

impl Control for Border {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn raw_copy(&self) -> Box<dyn Control> {
        Box::new(Self {
            widget: *self.widget.raw_copy().downcast::<Widget>().unwrap_or_else(|_| panic!()),
            stroke_thickness: self.stroke_thickness,
        })
    }

    fn resolve(&mut self, _: &ControlTemplate, _: &HashMap<Handle<UINode>, Handle<UINode>>) {}

    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let margin_x = self.stroke_thickness.left + self.stroke_thickness.right;
        let margin_y = self.stroke_thickness.top + self.stroke_thickness.bottom;

        let size_for_child = Vec2::new(
            available_size.x - margin_x,
            available_size.y - margin_y,
        );
        let mut desired_size = Vec2::ZERO;

        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).measure(ui, size_for_child);
            let child = ui.nodes.borrow(*child_handle).widget();
            let child_desired_size = child.desired_size.get();
            if child_desired_size.x > desired_size.x {
                desired_size.x = child_desired_size.x;
            }
            if child_desired_size.y > desired_size.y {
                desired_size.y = child_desired_size.y;
            }
        }

        desired_size.x += margin_x;
        desired_size.y += margin_y;

        desired_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let rect_for_child = Rect::new(
            self.stroke_thickness.left, self.stroke_thickness.top,
            final_size.x - (self.stroke_thickness.right + self.stroke_thickness.left),
            final_size.y - (self.stroke_thickness.bottom + self.stroke_thickness.top),
        );

        for child_handle in self.widget.children.iter() {
            ui.node(*child_handle).arrange(ui, &rect_for_child);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.get_screen_bounds();
        drawing_context.push_rect_filled(&bounds, None, self.widget.background());
        drawing_context.push_rect_vary(&bounds, self.stroke_thickness, self.widget.foreground());
        drawing_context.commit(CommandKind::Geometry, CommandTexture::None);
    }

    fn set_property(&mut self, name: &str, value: &dyn Any) {
        match name {
            Self::STROKE_THICKNESS => if let Some(value) = value.downcast_ref() {
                self.stroke_thickness = *value;
            },
            _ => ()
        }
    }

    fn get_property(&self, name: &str) -> Option<&dyn Any> {
        match name {
            Self::STROKE_THICKNESS => Some(&self.stroke_thickness),
            _ => None
        }
    }
}

impl Border {
    pub const STROKE_THICKNESS: &'static str = "StrokeThickness";

    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            stroke_thickness: Thickness::uniform(1.0),
        }
    }

    pub fn set_stroke_thickness(&mut self, thickness: Thickness) -> &mut Self {
        self.stroke_thickness = thickness;
        self
    }
}