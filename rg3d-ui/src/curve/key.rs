use crate::{
    core::{pool::Handle, uuid::Uuid},
    draw::{CommandTexture, Draw, DrawingContext},
    message::{CurveKeyMessage, MessageData, MessageDirection, UiMessage, UiMessageData},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub enum Kind {
    Constant,
    Linear,
    Cubic {
        left_tangent: f32,
        right_tangent: f32,
    },
}

#[derive(Clone)]
pub struct CurveKey<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    kind: Kind,
    id: Uuid,
}

crate::define_widget_deref!(CurveKey<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CurveKey<M, C> {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let screen_bounds = self.screen_bounds();
        drawing_context.push_rect_filled(&screen_bounds, None);
        drawing_context.commit(screen_bounds, self.foreground(), CommandTexture::None, None);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.direction() == MessageDirection::ToWidget && message.destination() == self.handle
        {
            if let UiMessageData::CurveKey(CurveKeyMessage::Sync(key)) = message.data() {}
        }
    }
}

pub struct CurveKeyBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    id: Uuid,
}

impl<M: MessageData, C: Control<M, C>> CurveKeyBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            id: Default::default(),
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn build(self, ui: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let key = CurveKey {
            widget: self.widget_builder.build(),
            kind: Kind::Linear,
            id: self.id,
        };
        ui.add_node(UINode::CurveKey(key))
    }
}
