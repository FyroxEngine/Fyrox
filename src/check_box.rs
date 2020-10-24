use crate::message::{MessageData, MessageDirection};
use crate::{
    border::BorderBuilder,
    brush::Brush,
    core::{color::Color, pool::Handle},
    message::{CheckBoxMessage, UiMessage, UiMessageData, WidgetMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, NodeHandleMapping, Thickness, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct CheckBox<M: MessageData, C: Control<M, C>> {
    pub widget: Widget<M, C>,
    pub checked: Option<bool>,
    pub check_mark: Handle<UINode<M, C>>,
}

crate::define_widget_deref!(CheckBox<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for CheckBox<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.check_mark);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Widget(ref msg) => {
                match msg {
                    WidgetMessage::MouseDown { .. } => {
                        if message.destination() == self.handle()
                            || self.widget.has_descendant(message.destination(), ui)
                        {
                            ui.capture_mouse(self.handle());
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        if message.destination() == self.handle()
                            || self.widget.has_descendant(message.destination(), ui)
                        {
                            ui.release_mouse_capture();

                            if let Some(value) = self.checked {
                                // Invert state if it is defined.
                                ui.send_message(CheckBoxMessage::checked(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    Some(!value),
                                ));
                            } else {
                                // Switch from undefined state to checked.
                                ui.send_message(CheckBoxMessage::checked(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    Some(true),
                                ));
                            }
                        }
                    }
                    _ => (),
                }
            }
            UiMessageData::CheckBox(ref msg)
                if message.direction() == MessageDirection::ToWidget
                    && message.destination() == self.handle() =>
            {
                if let CheckBoxMessage::Check(value) = *msg {
                    if self.checked != value {
                        self.checked = value;

                        ui.send_message(message.reverse());

                        if self.check_mark.is_some() {
                            match value {
                                None => {
                                    ui.send_message(WidgetMessage::background(
                                        self.check_mark,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::opaque(30, 30, 80)),
                                    ));
                                }
                                Some(value) => {
                                    ui.send_message(WidgetMessage::background(
                                        self.check_mark,
                                        MessageDirection::ToWidget,
                                        Brush::Solid(Color::opaque(200, 200, 200)),
                                    ));
                                    ui.send_message(WidgetMessage::visibility(
                                        self.check_mark,
                                        MessageDirection::ToWidget,
                                        value,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.check_mark == handle {
            self.check_mark = Handle::NONE;
        }
    }
}

pub struct CheckBoxBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    checked: Option<bool>,
    check_mark: Option<Handle<UINode<M, C>>>,
}

impl<M: MessageData, C: Control<M, C>> CheckBoxBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            checked: Some(false),
            check_mark: None,
        }
    }

    pub fn checked(mut self, value: Option<bool>) -> Self {
        self.checked = value;
        self
    }

    pub fn with_check_mark(mut self, check_mark: Handle<UINode<M, C>>) -> Self {
        self.check_mark = Some(check_mark);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let check_mark = self.check_mark.unwrap_or_else(|| {
            BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(Brush::Solid(Color::opaque(200, 200, 200)))
                    .with_margin(Thickness::uniform(1.0)),
            )
            .with_stroke_thickness(Thickness::uniform(0.0))
            .build(ctx)
        });
        ctx[check_mark].set_visibility(self.checked.unwrap_or(true));

        let cb = CheckBox {
            widget: self
                .widget_builder
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(Brush::Solid(Color::opaque(60, 60, 60)))
                            .with_child(check_mark),
                    )
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ctx),
                )
                .build(),
            checked: self.checked,
            check_mark,
        };
        ctx.add_node(UINode::CheckBox(cb))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        check_box::CheckBoxBuilder,
        core::math::vec2::Vec2,
        message::{CheckBoxMessage, MessageDirection},
        node::StubNode,
        widget::WidgetBuilder,
        UserInterface,
    };

    #[test]
    fn check_box() {
        let mut ui = UserInterface::<(), StubNode>::new(Vec2::new(1000.0, 1000.0));

        assert_eq!(ui.poll_message(), None);

        let check_box = CheckBoxBuilder::new(WidgetBuilder::new()).build(&mut ui.build_ctx());

        assert_eq!(ui.poll_message(), None);

        // Check messages
        let input_message =
            CheckBoxMessage::checked(check_box, MessageDirection::ToWidget, Some(true));

        ui.send_message(input_message.clone());

        // This message that we just send.
        assert_eq!(ui.poll_message(), Some(input_message.clone()));
        // We must get response from check box.
        assert_eq!(ui.poll_message(), Some(input_message.reverse()));
    }
}
