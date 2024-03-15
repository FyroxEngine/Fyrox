//! Simple overlay "thumb" with a line that show playback position of an animation.
//! It is made as a separate widget to be able to draw it on top of curve editor,
//! dope sheet and time ruler.

use crate::fyrox::{
    core::{
        algebra::{Matrix3, Point2, Vector2},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid_provider,
        visitor::prelude::*,
    },
    gui::{
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface, BRUSH_BRIGHT,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub enum ThumbMessage {
    Zoom(f32),
    ViewPosition(f32),
    Position(f32),
}

impl ThumbMessage {
    define_constructor!(ThumbMessage:Zoom => fn zoom(f32), layout: false);
    define_constructor!(ThumbMessage:ViewPosition => fn view_position(f32), layout: false);
    define_constructor!(ThumbMessage:Position => fn position(f32), layout: false);
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct Thumb {
    widget: Widget,
    zoom: f32,
    view_position: f32,
    position: f32,
}

define_widget_deref!(Thumb);

impl Thumb {
    fn view_matrix(&self) -> Matrix3<f32> {
        Matrix3::new_nonuniform_scaling_wrt_point(
            &Vector2::new(self.zoom, 1.0),
            &Point2::from(self.actual_local_size().scale(0.5)),
        ) * Matrix3::new_translation(&Vector2::new(self.view_position, 0.0))
    }

    fn local_to_view(&self, x: f32) -> f32 {
        self.view_matrix().transform_point(&Point2::new(x, 0.0)).x
    }
}

uuid_provider!(Thumb = "820ba009-54e0-4050-ba7e-28f1f5b40429");

impl Control for Thumb {
    fn draw(&self, ctx: &mut DrawingContext) {
        let local_bounds = self.bounding_rect();

        let half_width = 5.0;
        let view_position = self.local_to_view(self.position);
        let origin = Vector2::new(view_position, 0.0);

        ctx.push_triangle_filled([
            origin - Vector2::new(half_width, 0.0),
            origin + Vector2::new(half_width, 0.0),
            origin + Vector2::new(0.0, 2.0 * half_width),
        ]);
        ctx.push_line(origin, origin + Vector2::new(0.0, local_bounds.h()), 1.0);
        ctx.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<ThumbMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    ThumbMessage::Zoom(zoom) => {
                        self.zoom = *zoom;
                    }
                    ThumbMessage::ViewPosition(position) => {
                        self.view_position = *position;
                    }
                    ThumbMessage::Position(value) => {
                        if value.ne(&self.position) {
                            self.position = *value;
                            ui.send_message(message.reverse());
                        }
                    }
                }
            }
        }
    }
}

pub struct ThumbBuilder {
    widget_builder: WidgetBuilder,
}

impl ThumbBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let ruler = Thumb {
            widget: self
                .widget_builder
                .with_hit_test_visibility(false)
                .with_foreground(BRUSH_BRIGHT)
                .build(),
            zoom: 1.0,
            view_position: 0.0,
            position: 0.0,
        };

        ctx.add_node(UiNode::new(ruler))
    }
}
