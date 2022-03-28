use fyrox::{
    animation::machine::transition::TransitionDefinition,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

const PICKED_BRUSH: Brush = Brush::Solid(Color::opaque(120, 120, 120));
const NORMAL_BRUSH: Brush = Brush::Solid(Color::opaque(80, 80, 80));

#[derive(Clone, Debug)]
pub struct Transition {
    widget: Widget,
    pub source: Handle<UiNode>,
    source_pos: Vector2<f32>,
    pub dest: Handle<UiNode>,
    dest_pos: Vector2<f32>,
    pub model_handle: Handle<TransitionDefinition>,
}

define_widget_deref!(Transition);

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionMessage {
    SourcePosition(Vector2<f32>),
    DestPosition(Vector2<f32>),
}

impl TransitionMessage {
    define_constructor!(TransitionMessage:SourcePosition => fn source_position(Vector2<f32>), layout: false);
    define_constructor!(TransitionMessage:DestPosition => fn dest_position(Vector2<f32>), layout: false);
}

pub fn draw_transition(
    drawing_context: &mut DrawingContext,
    clip_bounds: Rect<f32>,
    brush: Brush,
    source_pos: Vector2<f32>,
    dest_pos: Vector2<f32>,
) {
    drawing_context.push_line(source_pos, dest_pos, 5.0);

    let axis = (dest_pos - source_pos).normalize();
    let center = (dest_pos + source_pos).scale(0.5);
    let perp = Vector2::new(axis.y, -axis.x).normalize();

    let size = 20.0;

    drawing_context.push_triangle_filled([
        center + axis.scale(size),
        center + perp.scale(size * 0.5),
        center - perp.scale(size * 0.5),
    ]);

    drawing_context.commit(clip_bounds, brush, CommandTexture::None, None);
}

impl Control for Transition {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        draw_transition(
            drawing_context,
            self.clip_bounds(),
            self.foreground(),
            self.source_pos,
            self.dest_pos,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<TransitionMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    TransitionMessage::SourcePosition(pos) => {
                        self.source_pos = *pos;
                    }
                    TransitionMessage::DestPosition(pos) => {
                        self.dest_pos = *pos;
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
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

pub struct TransitionBuilder {
    widget_builder: WidgetBuilder,
    source: Handle<UiNode>,
    dest: Handle<UiNode>,
}

impl TransitionBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            source: Default::default(),
            dest: Default::default(),
        }
    }

    pub fn with_source(mut self, source: Handle<UiNode>) -> Self {
        self.source = source;
        self
    }

    pub fn with_dest(mut self, dest: Handle<UiNode>) -> Self {
        self.dest = dest;
        self
    }

    pub fn build(
        self,
        model_handle: Handle<TransitionDefinition>,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        fn fetch_node_position(handle: Handle<UiNode>, ctx: &BuildContext) -> Vector2<f32> {
            ctx.try_get_node(handle)
                .map(|node| node.actual_local_position() + node.actual_size().scale(0.5))
                .unwrap_or_default()
        }

        let transition = Transition {
            widget: self
                .widget_builder
                .with_foreground(NORMAL_BRUSH.clone())
                .with_clip_to_bounds(false)
                .build(),
            source: self.source,
            source_pos: fetch_node_position(self.source, ctx),
            dest: self.dest,
            dest_pos: fetch_node_position(self.dest, ctx),
            model_handle,
        };

        ctx.add_node(UiNode::new(transition))
    }
}
