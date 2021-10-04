use crate::{
    core::{algebra::Vector2, math::Rect, pool::Handle, scope_profile},
    draw::{CommandTexture, Draw, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UiNode, UserInterface, BRUSH_PRIMARY,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct Border {
    widget: Widget,
    stroke_thickness: Thickness,
}

crate::define_widget_deref!(Border);

impl Control for Border {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let margin_x = self.stroke_thickness.left + self.stroke_thickness.right;
        let margin_y = self.stroke_thickness.top + self.stroke_thickness.bottom;

        let size_for_child = Vector2::new(available_size.x - margin_x, available_size.y - margin_y);
        let mut desired_size = Vector2::default();

        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, size_for_child);
            let child = ui.nodes.borrow(*child_handle);
            let child_desired_size = child.desired_size();
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

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let rect_for_child = Rect::new(
            self.stroke_thickness.left,
            self.stroke_thickness.top,
            final_size.x - (self.stroke_thickness.right + self.stroke_thickness.left),
            final_size.y - (self.stroke_thickness.bottom + self.stroke_thickness.top),
        );

        for child_handle in self.widget.children() {
            ui.arrange_node(*child_handle, &rect_for_child);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        DrawingContext::push_rect_filled(drawing_context, &bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.background(),
            CommandTexture::None,
            None,
        );

        drawing_context.push_rect_vary(&bounds, self.stroke_thickness);
        drawing_context.commit(
            self.clip_bounds(),
            self.widget.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

impl Border {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            stroke_thickness: Thickness::uniform(1.0),
        }
    }
}

pub struct BorderBuilder {
    pub widget_builder: WidgetBuilder,
    pub stroke_thickness: Option<Thickness>,
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

    pub fn build_border(mut self) -> Border {
        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(BRUSH_PRIMARY);
        }
        Border {
            widget: self.widget_builder.build(),
            stroke_thickness: self
                .stroke_thickness
                .unwrap_or_else(|| Thickness::uniform(1.0)),
        }
    }

    pub fn build(self, ctx: &mut BuildContext<'_>) -> Handle<UiNode> {
        ctx.add_node(UiNode::new(self.build_border()))
    }
}
