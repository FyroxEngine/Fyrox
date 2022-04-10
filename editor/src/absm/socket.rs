use fyrox::animation::machine::node::PoseNodeDefinition;
use fyrox::{
    core::{algebra::Vector2, color::Color, pool::Handle},
    gui::{
        define_widget_deref,
        draw::{CommandTexture, Draw, DrawingContext},
        message::UiMessage,
        widget::{Widget, WidgetBuilder},
        BuildContext, Control, UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub struct Socket {
    widget: Widget,
    #[allow(dead_code)] // TODO
    parent_node: Handle<PoseNodeDefinition>,
}

define_widget_deref!(Socket);

const RADIUS: f32 = 7.0;

impl Control for Socket {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
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
}

pub struct SocketBuilder {
    widget_builder: WidgetBuilder,
    parent_node: Handle<PoseNodeDefinition>,
}

impl SocketBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            parent_node: Default::default(),
        }
    }

    pub fn with_parent_node(mut self, parent_node: Handle<PoseNodeDefinition>) -> Self {
        self.parent_node = parent_node;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let socket = Socket {
            widget: self.widget_builder.build(),
            parent_node: self.parent_node,
        };

        ctx.add_node(UiNode::new(socket))
    }
}
