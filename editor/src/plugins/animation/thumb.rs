// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Simple overlay "thumb" with a line that show playback position of an animation.
//! It is made as a separate widget to be able to draw it on top of curve editor,
//! dope sheet and time ruler.

use crate::fyrox::{
    core::{
        algebra::{Point2, Vector2},
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
        BuildContext, Control, UiNode, UserInterface,
    },
};
use fyrox::core::pool::NodeVariant;

use fyrox::gui::curve::CurveTransformCell;
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
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
#[reflect(derived_type = "UiNode")]
pub struct Thumb {
    widget: Widget,
    #[visit(skip)]
    #[reflect(hidden)]
    transform: CurveTransformCell,
    position: f32,
}

impl NodeVariant<UiNode> for Thumb {}

define_widget_deref!(Thumb);

impl Thumb {
    fn local_to_view(&self, x: f32) -> f32 {
        self.transform
            .curve_to_local()
            .transform_point(&Point2::new(x, 0.0))
            .x
    }
}

uuid_provider!(Thumb = "820ba009-54e0-4050-ba7e-28f1f5b40429");

impl Control for Thumb {
    fn draw(&self, ctx: &mut DrawingContext) {
        self.transform.set_bounds(self.screen_bounds());
        self.transform.update_transform();
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
            &self.material,
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
                        self.transform.set_scale(Vector2::new(*zoom, 1.0));
                    }
                    ThumbMessage::ViewPosition(position) => {
                        self.transform.set_position(Vector2::new(*position, 0.0));
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
                .with_foreground(ctx.style.property(Style::BRUSH_BRIGHT))
                .build(ctx),
            transform: CurveTransformCell::default(),
            position: 0.0,
        };

        ctx.add_node(UiNode::new(ruler))
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::animation::thumb::ThumbBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| ThumbBuilder::new(WidgetBuilder::new()).build(ctx));
    }
}
