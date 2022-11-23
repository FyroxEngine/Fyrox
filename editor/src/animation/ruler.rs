use fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        color::Color,
        math::round_to_step,
        pool::Handle,
    },
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::{FormattedText, FormattedTextBuilder},
        message::{MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, PartialEq)]
pub enum RulerMessage {
    Zoom(f32),
    ViewPosition(f32),
    Value(f32),
}

impl RulerMessage {
    define_constructor!(RulerMessage:Zoom => fn zoom(f32), layout: false);
    define_constructor!(RulerMessage:ViewPosition => fn view_position(f32), layout: false);
    define_constructor!(RulerMessage:Value => fn value(f32), layout: false);
}

#[derive(Clone)]
pub struct Ruler {
    widget: Widget,
    zoom: f32,
    view_position: f32,
    text: RefCell<FormattedText>,
    value: f32,
}

define_widget_deref!(Ruler);

impl Ruler {
    // It could be done without matrices, which is indeed faster, but I don't care.
    fn view_matrix(&self) -> Matrix3<f32> {
        Matrix3::new_nonuniform_scaling_wrt_point(
            &Vector2::new(self.zoom, 1.0),
            &Point2::from(self.actual_local_size().scale(0.5)),
        ) * Matrix3::new_translation(&Vector2::new(self.view_position, 0.0))
    }

    fn local_to_view(&self, x: f32) -> f32 {
        self.view_matrix().transform_point(&Point2::new(x, 0.0)).x
    }

    fn view_to_local(&self, x: f32) -> f32 {
        self.view_matrix()
            .try_inverse()
            .unwrap_or_default()
            .transform_point(&Point2::new(x, 0.0))
            .x
    }
}

impl Control for Ruler {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, ctx: &mut DrawingContext) {
        let local_bounds = self.bounding_rect();

        // Add clickable rectangle first.
        ctx.push_rect_filled(&local_bounds, None);
        ctx.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            None,
        );

        // Then draw the rest.
        let step_size_x = 50.0 / self.zoom.clamp(0.001, 1000.0);

        let left_local_bound = round_to_step(self.view_to_local(0.0), step_size_x);
        let right_local_bound = round_to_step(
            self.view_to_local(local_bounds.position.x + local_bounds.size.x),
            step_size_x,
        );

        let range = right_local_bound - left_local_bound;
        let steps = ((range / step_size_x).ceil()) as usize;

        for nx in 0..=steps {
            let k = nx as f32 / steps as f32;
            let x = self.local_to_view(left_local_bound + k * range);
            ctx.push_line(
                Vector2::new(x, local_bounds.size.y * 0.5),
                Vector2::new(x, local_bounds.size.y),
                1.0,
            );
        }

        // Draw values.
        let mut text = self.text.borrow_mut();

        for nx in 0..=steps {
            let k = nx as f32 / steps as f32;
            let x = left_local_bound + k * range;
            text.set_text(format!("{:.1}", x)).build();
            let vx = self.local_to_view(x);
            ctx.draw_text(self.clip_bounds(), Vector2::new(vx + 1.0, 0.0), &text);
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<RulerMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    RulerMessage::Zoom(zoom) => {
                        self.zoom = *zoom;
                    }
                    RulerMessage::ViewPosition(position) => {
                        self.view_position = *position;
                    }
                    RulerMessage::Value(value) => {
                        if value.ne(&self.value) {
                            self.value = *value;
                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::MouseDown { pos, button }) = message.data() {
            if message.direction() == MessageDirection::FromWidget && *button == MouseButton::Left {
                let local_click_pos = self.screen_to_local(*pos);
                let value = self.view_to_local(local_click_pos.x);
                ui.send_message(RulerMessage::value(
                    self.handle,
                    MessageDirection::ToWidget,
                    value,
                ));
            }
        }
    }
}

pub struct RulerBuilder {
    widget_builder: WidgetBuilder,
    value: f32,
}

impl RulerBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: 0.0,
        }
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let ruler = Ruler {
            widget: self.widget_builder.build(),
            zoom: 1.0,
            view_position: 0.0,
            text: RefCell::new(FormattedTextBuilder::new(ctx.default_font()).build()),
            value: self.value,
        };

        ctx.add_node(UiNode::new(ruler))
    }
}
