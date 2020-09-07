use crate::{
    brush::Brush,
    core::{
        color::Color,
        math::{vec2::Vec2, Rect},
        pool::Handle,
        scope_profile,
    },
    draw::{CommandKind, CommandTexture, DrawingContext},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Thickness, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

pub struct Border<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    stroke_thickness: Thickness,
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Deref for Border<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> DerefMut for Border<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Clone for Border<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            stroke_thickness: self.stroke_thickness,
        }
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Control<M, C> for Border<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Border(self.clone())
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        scope_profile!();

        let margin_x = self.stroke_thickness.left + self.stroke_thickness.right;
        let margin_y = self.stroke_thickness.top + self.stroke_thickness.bottom;

        let size_for_child = Vec2::new(available_size.x - margin_x, available_size.y - margin_y);
        let mut desired_size = Vec2::ZERO;

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, size_for_child);
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

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        scope_profile!();

        let rect_for_child = Rect::new(
            self.stroke_thickness.left,
            self.stroke_thickness.top,
            final_size.x - (self.stroke_thickness.right + self.stroke_thickness.left),
            final_size.y - (self.stroke_thickness.bottom + self.stroke_thickness.top),
        );

        for child_handle in self.widget.children() {
            ui.node(*child_handle).arrange(ui, &rect_for_child);
        }

        final_size
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.widget.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            CommandKind::Geometry,
            self.widget.background(),
            CommandTexture::None,
        );

        drawing_context.push_rect_vary(&bounds, self.stroke_thickness);
        drawing_context.commit(
            CommandKind::Geometry,
            self.widget.foreground(),
            CommandTexture::None,
        );
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);
    }
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> Border<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget,
            stroke_thickness: Thickness::uniform(1.0),
        }
    }

    pub fn set_stroke_thickness(&mut self, thickness: Thickness) -> &mut Self {
        if self.stroke_thickness != thickness {
            self.stroke_thickness = thickness;
            self.widget.invalidate_layout();
        }
        self
    }
}

pub struct BorderBuilder<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> {
    pub widget_builder: WidgetBuilder<M, C>,
    pub stroke_thickness: Option<Thickness>,
}

impl<M: 'static + std::fmt::Debug, C: 'static + Control<M, C>> BorderBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            stroke_thickness: None,
        }
    }

    pub fn with_stroke_thickness(mut self, stroke_thickness: Thickness) -> Self {
        self.stroke_thickness = Some(stroke_thickness);
        self
    }

    pub fn build_border(mut self) -> Border<M, C> {
        if self.widget_builder.foreground.is_none() {
            self.widget_builder.foreground = Some(Brush::Solid(Color::opaque(100, 100, 100)));
        }
        Border {
            widget: self.widget_builder.build(),
            stroke_thickness: self
                .stroke_thickness
                .unwrap_or_else(|| Thickness::uniform(1.0)),
        }
    }

    pub fn build(self, ctx: &mut BuildContext<'_, M, C>) -> Handle<UINode<M, C>> {
        ctx.add_node(UINode::Border(self.build_border()))
    }
}
