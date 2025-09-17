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

use crate::fyrox::{
    core::{
        algebra::Vector2, color::Color, math::Rect, pool::Handle, reflect::prelude::*,
        type_traits::prelude::*, uuid_provider, visitor::prelude::*,
    },
    gui::{
        brush::Brush,
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use crate::plugins::absm::segment::Segment;
use crate::utils::fetch_node_screen_center;
use fyrox::core::pool::NodeVariant;

use fyrox::material::MaterialResource;
use std::ops::{Deref, DerefMut};

const PICKED_BRUSH: Brush = Brush::Solid(Color::opaque(100, 100, 100));
const NORMAL_BRUSH: Brush = Brush::Solid(Color::opaque(80, 80, 80));

#[derive(Debug, Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct Connection {
    widget: Widget,
    pub segment: Segment,
    pub source_node: Handle<UiNode>,
    pub dest_node: Handle<UiNode>,
}

impl NodeVariant<UiNode> for Connection {}

define_widget_deref!(Connection);

pub fn draw_connection(
    drawing_context: &mut DrawingContext,
    source: Vector2<f32>,
    dest: Vector2<f32>,
    clip_bounds: Rect<f32>,
    brush: Brush,
    material: &MaterialResource,
) {
    let k = 75.0;
    drawing_context.push_bezier(
        source,
        source + Vector2::new(k, 0.0),
        dest - Vector2::new(k, 0.0),
        dest,
        20,
        4.0,
    );
    drawing_context.commit(clip_bounds, brush, CommandTexture::None, material, None);
}

uuid_provider!(Connection = "c802b6fa-a5ef-4464-a097-749c731ffde0");

impl Control for Connection {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        draw_connection(
            drawing_context,
            self.segment.source_pos,
            self.segment.dest_pos,
            self.clip_bounds(),
            self.foreground(),
            &self.material,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.segment.handle_routed_message(self.handle(), message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseEnter => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        PICKED_BRUSH.clone().into(),
                    ));
                }
                WidgetMessage::MouseLeave => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        NORMAL_BRUSH.clone().into(),
                    ));
                }
                _ => (),
            }
        }
    }
}

pub struct ConnectionBuilder {
    widget_builder: WidgetBuilder,
    source_socket: Handle<UiNode>,
    source_node: Handle<UiNode>,
    dest_socket: Handle<UiNode>,
    dest_node: Handle<UiNode>,
}

impl ConnectionBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            source_socket: Default::default(),
            source_node: Default::default(),
            dest_socket: Default::default(),
            dest_node: Default::default(),
        }
    }

    pub fn with_source_socket(mut self, source: Handle<UiNode>) -> Self {
        self.source_socket = source;
        self
    }

    pub fn with_dest_socket(mut self, dest: Handle<UiNode>) -> Self {
        self.dest_socket = dest;
        self
    }

    pub fn with_source_node(mut self, source: Handle<UiNode>) -> Self {
        self.source_node = source;
        self
    }

    pub fn with_dest_node(mut self, dest: Handle<UiNode>) -> Self {
        self.dest_node = dest;
        self
    }

    pub fn build(self, canvas: Handle<UiNode>, ctx: &mut BuildContext) -> Handle<UiNode> {
        let canvas_ref = ctx.try_get_node(canvas).ok();

        let connection = Connection {
            widget: self
                .widget_builder
                .with_foreground(NORMAL_BRUSH.into())
                .with_clip_to_bounds(false)
                .build(ctx),
            segment: Segment {
                source: self.source_socket,
                source_pos: canvas_ref
                    .map(|c| c.screen_to_local(fetch_node_screen_center(self.source_socket, ctx)))
                    .unwrap_or_default(),
                dest: self.dest_socket,
                dest_pos: canvas_ref
                    .map(|c| c.screen_to_local(fetch_node_screen_center(self.dest_socket, ctx)))
                    .unwrap_or_default(),
            },
            source_node: self.source_node,
            dest_node: self.dest_node,
        };

        ctx.add_node(UiNode::new(connection))
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::absm::connection::ConnectionBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            ConnectionBuilder::new(WidgetBuilder::new()).build(Default::default(), ctx)
        });
    }
}
