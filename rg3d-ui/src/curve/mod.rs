use crate::canvas::CanvasBuilder;
use crate::{
    core::{
        curve::{Curve, CurveKey},
        pool::Handle,
        uuid::Uuid,
    },
    curve::key::CurveKeyBuilder,
    draw::DrawingContext,
    message::{
        CurveEditorMessage, MessageData, MessageDirection, UiMessage, UiMessageData, WidgetMessage,
    },
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

pub mod key;

#[derive(Clone)]
pub struct CurveEditor<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    canvas: Handle<UINode<M, C>>,
    keys: HashMap<Uuid, Handle<UINode<M, C>>>,
}

crate::define_widget_deref!(CurveEditor<M, C>);

fn make_key_view<M: MessageData, C: Control<M, C>>(
    key: &CurveKey,
    ctx: &mut BuildContext<M, C>,
) -> Handle<UINode<M, C>> {
    CurveKeyBuilder::new(WidgetBuilder::new())
        .with_id(key.id)
        .build(ctx)
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CurveEditor<M, C> {
    fn draw(&self, _drawing_context: &mut DrawingContext) {}

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        if message.direction() == MessageDirection::ToWidget && message.destination() == self.handle
        {
            if let UiMessageData::CurveEditor(CurveEditorMessage::Sync(curve)) = message.data() {
                let self_keys = self.keys.clone();
                if curve.keys().len() < self_keys.len() {
                    // A key was deleted.
                    for (key_id, key_handle) in self_keys.iter() {
                        if curve.keys().iter().all(|k| &k.id != key_id) {
                            ui.send_message(WidgetMessage::remove(
                                *key_handle,
                                MessageDirection::ToWidget,
                            ));

                            self.keys.remove(key_id);
                        }
                    }
                } else if curve.keys().len() > self.keys.len() {
                    // A key was added.
                    for key in curve.keys() {
                        if !self.keys.contains_key(&key.id) {
                            let key_view = make_key_view(key, &mut ui.build_ctx());
                            self.keys.insert(key.id, key_view);
                        }
                    }
                }

                // Sync values.
            }
        }
    }
}

pub struct CurveEditorBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    curve: Curve,
}

impl<M: MessageData, C: Control<M, C>> CurveEditorBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            curve: Default::default(),
        }
    }

    pub fn with_curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let keys = self
            .curve
            .keys()
            .iter()
            .map(|k| (k.id, make_key_view(k, ctx)))
            .collect::<HashMap<_, _>>();

        let canvas =
            CanvasBuilder::new(WidgetBuilder::new().with_children(keys.iter().map(|(_, h)| h)))
                .build(ctx);

        let editor = CurveEditor {
            widget: self.widget_builder.with_child(canvas).build(),
            keys,
            canvas,
        };
        ctx.add_node(UINode::CurveEditor(editor))
    }
}
