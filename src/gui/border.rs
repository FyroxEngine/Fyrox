use rg3d_core::{
    color::Color,
    pool::Handle,
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::{
    draw::{
        CommandKind,
        DrawingContext
    },
    Thickness,
    UserInterface,
    Layout,
    Draw,
    node::UINode,
    widget::{
        Widget,
        WidgetBuilder,
        AsWidget
    },
    draw::CommandTexture,
    Update,
};

pub struct Border {
    widget: Widget,
    stroke_thickness: Thickness,
    stroke_color: Color,
}

impl AsWidget for Border {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }
}

impl Update for Border {
    fn update(&mut self, dt: f32) {
        self.widget.update(dt)
    }
}

pub struct BorderBuilder {
    widget_builder: WidgetBuilder,
    stroke_thickness: Option<Thickness>,
    stroke_color: Option<Color>,
}

impl BorderBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            stroke_color: None,
            stroke_thickness: None,
        }
    }

    pub fn with_stroke_thickness(mut self, stroke_thickness: Thickness) -> Self {
        self.stroke_thickness = Some(stroke_thickness);
        self
    }

    pub fn with_stroke_color(mut self, color: Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        ui.add_node(UINode::Border(Border {
            widget: self.widget_builder.build(),
            stroke_thickness: self.stroke_thickness.unwrap_or_else(|| Thickness::uniform(1.0)),
            stroke_color: self.stroke_color.unwrap_or(Color::WHITE),
        }))
    }
}

impl Draw for Border {
    fn draw(&mut self, drawing_context: &mut DrawingContext) {
        let bounds= self.widget.get_screen_bounds();
        drawing_context.push_rect_filled(&bounds, None, self.widget.color);
        drawing_context.push_rect_vary(&bounds, self.stroke_thickness, self.stroke_color);
        drawing_context.commit(CommandKind::Geometry, CommandTexture::None);
    }
}

impl Layout for Border {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let margin_x = self.stroke_thickness.left + self.stroke_thickness.right;
        let margin_y = self.stroke_thickness.top + self.stroke_thickness.bottom;

        let size_for_child = Vec2::new(
            available_size.x - margin_x,
            available_size.y - margin_y,
        );
        let mut desired_size = Vec2::ZERO;

        for child_handle in self.widget.children.iter() {
            ui.measure(*child_handle, size_for_child);
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
            ui.arrange(*child_handle, &rect_for_child);
        }

        final_size
    }
}

impl Border {
    pub fn set_stroke_thickness(&mut self, thickness: Thickness) -> &mut Self {
        self.stroke_thickness = thickness;
        self
    }

    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = color;
        self
    }
}