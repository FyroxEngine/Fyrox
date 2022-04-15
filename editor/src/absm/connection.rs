use crate::{
    absm::{
        canvas::AbsmCanvas,
        segment::{Segment, SegmentMessage},
    },
    utils::{fetch_node_screen_center, fetch_node_screen_center_ui},
    MessageDirection,
};
use fyrox::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    gui::{
        brush::Brush,
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::UiMessage,
        widget::WidgetMessage,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
pub struct Connection {
    widget: Widget,
    segment: Segment,
    source_node: Handle<UiNode>,
    dest_node: Handle<UiNode>,
}

define_widget_deref!(Connection);

pub fn draw_connection(
    drawing_context: &mut DrawingContext,
    source: Vector2<f32>,
    dest: Vector2<f32>,
    clip_bounds: Rect<f32>,
    brush: Brush,
) {
    drawing_context.push_line(source, dest, 2.0);
    drawing_context.commit(clip_bounds, brush, CommandTexture::None, None);
}

impl Control for Connection {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if message.destination() == self.source_node || message.destination() == self.dest_node {
            if let Some(WidgetMessage::DesiredPosition(_)) = message.data() {
                if let Some(parent_canvas) =
                    ui.try_borrow_by_criteria_up(self.handle(), |n| n.has_component::<AbsmCanvas>())
                {
                    let canvas_ref = parent_canvas.query_component::<AbsmCanvas>().unwrap();

                    let source_pos = canvas_ref
                        .screen_to_local(fetch_node_screen_center_ui(self.segment.source, ui));
                    ui.send_message(SegmentMessage::source_position(
                        self.handle(),
                        MessageDirection::ToWidget,
                        source_pos,
                    ));

                    let dest_pos = canvas_ref
                        .screen_to_local(fetch_node_screen_center_ui(self.segment.dest, ui));
                    ui.send_message(SegmentMessage::dest_position(
                        self.handle(),
                        MessageDirection::ToWidget,
                        dest_pos,
                    ));
                }
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
                .with_preview_messages(true)
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
