use crate::gui::{
    draw::{CommandKind, DrawingContext},
    Thickness,
    UserInterface,
    Layout,
    Drawable,
    node::{UINode, UINodeKind},
    builder::CommonBuilderFields,
};

use rg3d_core::{
    color::Color,
    pool::{Handle},
    math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::gui::draw::CommandTexture;

#[derive(Debug)]
pub struct Border {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    stroke_thickness: Thickness,
    stroke_color: Color,
}

pub struct BorderBuilder {
    stroke_thickness: Option<Thickness>,
    stroke_color: Option<Color>,
    common: CommonBuilderFields,
}

impl Default for BorderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BorderBuilder {
    pub fn new() -> Self {
        Self {
            stroke_color: None,
            stroke_thickness: None,
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

    pub fn with_stroke_thickness(mut self, stroke_thickness: Thickness) -> Self {
        self.stroke_thickness = Some(stroke_thickness);
        self
    }

    pub fn with_stroke_color(mut self, color: Color) -> Self {
        self.stroke_color = Some(color);
        self
    }

    pub fn build(mut self, ui: &mut UserInterface) -> Handle<UINode> {
        let mut border = Border {
            owner_handle: Handle::NONE,
            stroke_thickness: Thickness {
                left: 1.0,
                right: 1.0,
                top: 1.0,
                bottom: 1.0,
            },
            stroke_color: Color::white(),
        };

        if let Some(stroke_color) = self.stroke_color {
            border.stroke_color = stroke_color;
        }
        if let Some(stroke_thickness) = self.stroke_thickness {
            border.stroke_thickness = stroke_thickness;
        }
        let handle = ui.add_node(UINode::new(UINodeKind::Border(border)));
        self.common.apply(ui, handle);
        handle
    }
}

impl Drawable for Border {
    fn draw(&mut self, drawing_context: &mut DrawingContext, bounds: &Rect<f32>, color: Color) {
        drawing_context.push_rect_filled(&bounds, None, color);
        drawing_context.push_rect_vary(&bounds, self.stroke_thickness, self.stroke_color);
        drawing_context.commit(CommandKind::Geometry, CommandTexture::None);
    }
}

impl Layout for Border {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        let margin_x = self.stroke_thickness.left + self.stroke_thickness.right;
        let margin_y = self.stroke_thickness.top + self.stroke_thickness.bottom;

        let size_for_child = Vec2::make(
            available_size.x - margin_x,
            available_size.y - margin_y,
        );
        let mut desired_size = Vec2::zero();

        if let Some(node) = ui.nodes.borrow(self.owner_handle) {
            for child_handle in node.children.iter() {
                ui.measure(*child_handle, size_for_child);

                if let Some(child) = ui.nodes.borrow(*child_handle) {
                    let child_desired_size = child.desired_size.get();
                    if child_desired_size.x > desired_size.x {
                        desired_size.x = child_desired_size.x;
                    }
                    if child_desired_size.y > desired_size.y {
                        desired_size.y = child_desired_size.y;
                    }
                }
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

        if let Some(node) = ui.nodes.borrow(self.owner_handle) {
            for child_handle in node.children.iter() {
                ui.arrange(*child_handle, &rect_for_child);
            }
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