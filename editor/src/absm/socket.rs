use fyrox::{
    animation::machine::node::PoseNodeDefinition,
    core::{algebra::Vector2, color::Color, pool::Handle},
    gui::{
        brush::Brush,
        define_constructor, define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::{MessageDirection, MouseButton, UiMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

const PICKED_BRUSH: Brush = Brush::Solid(Color::opaque(170, 170, 170));
const NORMAL_BRUSH: Brush = Brush::Solid(Color::opaque(120, 120, 120));

#[derive(Debug, Clone, PartialEq)]
pub enum SocketMessage {
    // Occurs when user clicks on socket and starts dragging it.
    StartDragging,
}

impl SocketMessage {
    define_constructor!(SocketMessage:StartDragging => fn start_dragging(), layout: false);
}

#[derive(Copy, Clone, PartialEq, Hash, Debug)]
pub enum SocketDirection {
    Input,
    Output,
}

#[derive(Clone, Debug)]
pub struct Socket {
    widget: Widget,
    click_position: Option<Vector2<f32>>,
    pub parent_node: Handle<PoseNodeDefinition>,
    pub direction: SocketDirection,
}

define_widget_deref!(Socket);

const RADIUS: f32 = 8.0;

impl Control for Socket {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, _ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        Vector2::new(RADIUS * 2.0, RADIUS * 2.0)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        drawing_context.push_circle(bounds.center(), bounds.size.x / 2.0 - 1.0, 16, Color::WHITE);
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, pos } => {
                    if *button == MouseButton::Left {
                        self.click_position = Some(*pos);

                        ui.capture_mouse(self.handle());

                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseUp { button, .. } => {
                    if *button == MouseButton::Left {
                        self.click_position = None;

                        ui.release_mouse_capture();

                        message.set_handled(true);
                    }
                }
                WidgetMessage::MouseMove { pos, .. } => {
                    if let Some(click_position) = self.click_position {
                        if click_position.metric_distance(pos) >= 5.0 {
                            ui.send_message(SocketMessage::start_dragging(
                                self.handle(),
                                MessageDirection::FromWidget,
                            ));

                            self.click_position = None;
                        }
                    }
                }
                WidgetMessage::MouseLeave => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        NORMAL_BRUSH,
                    ));
                }
                WidgetMessage::MouseEnter => {
                    ui.send_message(WidgetMessage::foreground(
                        self.handle(),
                        MessageDirection::ToWidget,
                        PICKED_BRUSH,
                    ));
                }
                _ => (),
            }
        }
    }
}

pub struct SocketBuilder {
    widget_builder: WidgetBuilder,
    parent_node: Handle<PoseNodeDefinition>,
    direction: SocketDirection,
}

impl SocketBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            parent_node: Default::default(),
            direction: SocketDirection::Input,
        }
    }

    pub fn with_parent_node(mut self, parent_node: Handle<PoseNodeDefinition>) -> Self {
        self.parent_node = parent_node;
        self
    }

    pub fn with_direction(mut self, direction: SocketDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let socket = Socket {
            widget: self.widget_builder.with_foreground(NORMAL_BRUSH).build(),
            click_position: Default::default(),
            parent_node: self.parent_node,
            direction: self.direction,
        };

        ctx.add_node(UiNode::new(socket))
    }
}
