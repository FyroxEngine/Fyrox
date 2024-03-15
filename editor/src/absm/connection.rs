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
use crate::{absm::segment::Segment, utils::fetch_node_screen_center};
use std::ops::{Deref, DerefMut};

const PICKED_BRUSH: Brush = Brush::Solid(Color::opaque(100, 100, 100));
const NORMAL_BRUSH: Brush = Brush::Solid(Color::opaque(80, 80, 80));

#[derive(Debug, Clone, Visit, Reflect, ComponentProvider)]
pub struct Connection {
    widget: Widget,
    pub segment: Segment,
    pub source_node: Handle<UiNode>,
    pub dest_node: Handle<UiNode>,
}

define_widget_deref!(Connection);

pub fn draw_connection(
    drawing_context: &mut DrawingContext,
    source: Vector2<f32>,
    dest: Vector2<f32>,
    clip_bounds: Rect<f32>,
    brush: Brush,
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
    drawing_context.commit(clip_bounds, brush, CommandTexture::None, None);
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
                        PICKED_BRUSH.clone(),
                    ));
                }
                WidgetMessage::MouseLeave => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        NORMAL_BRUSH.clone(),
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
        let canvas_ref = &ctx[canvas];

        let connection = Connection {
            widget: self
                .widget_builder
                .with_foreground(NORMAL_BRUSH)
                .with_clip_to_bounds(false)
                .build(),
            segment: Segment {
                source: self.source_socket,
                source_pos: canvas_ref
                    .screen_to_local(fetch_node_screen_center(self.source_socket, ctx)),
                dest: self.dest_socket,
                dest_pos: canvas_ref
                    .screen_to_local(fetch_node_screen_center(self.dest_socket, ctx)),
            },
            source_node: self.source_node,
            dest_node: self.dest_node,
        };

        ctx.add_node(UiNode::new(connection))
    }
}
