use crate::absm::{
    segment::{Segment, SegmentMessage},
    selectable::{Selectable, SelectableMessage},
};
use crate::utils::fetch_node_center;
use fyrox::{
    animation::machine::transition::TransitionDefinition,
    core::{algebra::Vector2, color::Color, math::Rect, pool::Handle},
    gui::{
        brush::Brush,
        define_widget_deref,
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

const PICKED_BRUSH: Brush = Brush::Solid(Color::opaque(100, 100, 100));
const NORMAL_BRUSH: Brush = Brush::Solid(Color::opaque(80, 80, 80));
const SELECTED_BRUSH: Brush = Brush::Solid(Color::opaque(120, 120, 120));

#[derive(Clone, Debug)]
pub struct Transition {
    widget: Widget,
    pub segment: Segment,
    pub model_handle: Handle<TransitionDefinition>,
    selectable: Selectable,
}

impl Transition {
    fn handle_selection_change(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::foreground(
            self.handle(),
            MessageDirection::ToWidget,
            if self.selectable.selected {
                SELECTED_BRUSH.clone()
            } else {
                NORMAL_BRUSH.clone()
            },
        ));
    }
}

define_widget_deref!(Transition);

pub fn draw_transition(
    drawing_context: &mut DrawingContext,
    clip_bounds: Rect<f32>,
    brush: Brush,
    source_pos: Vector2<f32>,
    dest_pos: Vector2<f32>,
) {
    drawing_context.push_line(source_pos, dest_pos, 4.0);

    let axis = (dest_pos - source_pos).normalize();
    let center = (dest_pos + source_pos).scale(0.5);
    let perp = Vector2::new(axis.y, -axis.x).normalize();

    let size = 18.0;

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
        } else if type_id == TypeId::of::<Selectable>() {
            Some(&self.selectable)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        draw_transition(
            drawing_context,
            self.clip_bounds(),
            self.foreground(),
            self.segment.source_pos,
            self.segment.dest_pos,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        self.selectable
            .handle_routed_message(self.handle(), ui, message);
        self.segment
            .handle_routed_message(self.handle(), ui, message);

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
                    self.handle_selection_change(ui);
                }
                _ => (),
            }
        } else if let Some(SelectableMessage::Select(_)) = message.data() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::FromWidget
            {
                self.handle_selection_change(ui);
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        // Check if any node has moved and sync ends accordingly.
        if message.destination() == self.segment.source
            || message.destination() == self.segment.dest
        {
            if let Some(WidgetMessage::DesiredPosition(_)) = message.data() {
                // Find other transitions sharing the same source and dest nodes (in both directions).
                for (i, transition_handle) in ui
                    .node(self.parent())
                    .children()
                    .iter()
                    .filter_map(|c| {
                        ui.node(*c).query_component::<Transition>().and_then(|t| {
                            if t.segment.source == self.segment.source
                                && t.segment.dest == self.segment.dest
                                || t.segment.source == self.segment.dest
                                    && t.segment.dest == self.segment.source
                            {
                                Some(*c)
                            } else {
                                None
                            }
                        })
                    })
                    .enumerate()
                {
                    if transition_handle == self.handle() {
                        if let (Some(source_state), Some(dest_state)) = (
                            ui.try_get_node(self.segment.source),
                            ui.try_get_node(self.segment.dest),
                        ) {
                            let source_pos = source_state.center();
                            let dest_pos = dest_state.center();

                            let delta = dest_pos - source_pos;
                            let offset = Vector2::new(delta.y, -delta.x)
                                .normalize()
                                .scale(15.0 * i as f32);

                            ui.send_message(SegmentMessage::source_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                source_pos + offset,
                            ));

                            ui.send_message(SegmentMessage::dest_position(
                                self.handle(),
                                MessageDirection::ToWidget,
                                dest_pos + offset,
                            ));
                        }
                    }
                }
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
        let transition = Transition {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_foreground(NORMAL_BRUSH.clone())
                .with_clip_to_bounds(false)
                .build(),
            segment: Segment {
                source: self.source,
                source_pos: fetch_node_center(self.source, ctx),
                dest: self.dest,
                dest_pos: fetch_node_center(self.dest, ctx),
            },
            model_handle,
            selectable: Selectable::default(),
        };

        ctx.add_node(UiNode::new(transition))
    }
}
