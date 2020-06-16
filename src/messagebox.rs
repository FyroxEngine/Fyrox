use std::ops::{Deref, DerefMut};
use crate::{
    draw::DrawingContext,
    window::{
        Window,
        WindowBuilder,
        WindowTitle,
    },
    grid::{GridBuilder, Row, Column},
    node::UINode,
    Control,
    UserInterface,
    widget::{Widget, WidgetBuilder},
    message::{
        UiMessage,
        OsEvent,
        UiMessageData,
        ButtonMessage,
        MessageBoxMessage,
        WindowMessage,
    },
    core::{
        pool::Handle,
        math::vec2::Vec2,
        math::Rect,
    },
    text::TextBuilder,
    button::ButtonBuilder,
    stack_panel::StackPanelBuilder,
    Orientation,
    Thickness,
    HorizontalAlignment,
    NodeHandleMapping,
    BuildContext,
    message::TextMessage
};

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum MessageBoxResult {
    Ok,
    No,
    Yes,
    Cancel,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
    YesNoCancel,
}

pub struct MessageBox<M: 'static, C: 'static + Control<M, C>> {
    window: Window<M, C>,
    buttons: MessageBoxButtons,
    ok_yes: Handle<UINode<M, C>>,
    no: Handle<UINode<M, C>>,
    cancel: Handle<UINode<M, C>>,
    text: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for MessageBox<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for MessageBox<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for MessageBox<M, C> {
    fn clone(&self) -> Self {
        Self {
            window: self.window.clone(),
            buttons: self.buttons,
            ok_yes: self.ok_yes,
            no: self.no,
            cancel: self.cancel,
            text: self.text,
        }
    }
}

// Message box extends Window widget so it delegates most of calls
// to inner window.
impl<M: 'static, C: 'static + Control<M, C>> Control<M, C> for MessageBox<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::MessageBox(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.window.resolve(node_map);
        self.ok_yes = *node_map.get(&self.ok_yes).unwrap();
        self.no = *node_map.get(&self.no).unwrap();
        self.cancel = *node_map.get(&self.cancel).unwrap();
        self.text = *node_map.get(&self.text).unwrap();
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        self.window.arrange(ui, final_rect);
    }

    fn is_measure_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_measure_valid(ui)
    }

    fn is_arrange_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_arrange_valid(ui)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        self.window.measure(ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32) {
        self.window.update(dt);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.window.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.ok_yes {
                        let result = match self.buttons {
                            MessageBoxButtons::Ok => MessageBoxResult::Ok,
                            MessageBoxButtons::YesNo => MessageBoxResult::Yes,
                            MessageBoxButtons::YesNoCancel => MessageBoxResult::Yes,
                        };
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::MessageBox(MessageBoxMessage::Close(result)),
                            destination: self.handle(),
                        });
                    } else if message.destination == self.cancel {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::MessageBox(MessageBoxMessage::Close(MessageBoxResult::Cancel)),
                            destination: self.handle(),
                        });
                    } else if message.destination == self.no {
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::MessageBox(MessageBoxMessage::Close(MessageBoxResult::No)),
                            destination: self.handle(),
                        });
                    }
                }
            }
            UiMessageData::MessageBox(msg) => {
                match msg {
                    MessageBoxMessage::Open { title, text } => {
                        if let Some(title) = title {
                            ui.send_message(UiMessage {
                                handled: false,
                                data: UiMessageData::Window(WindowMessage::Title(WindowTitle::Text(title.clone()))),
                                destination: self.handle(),
                            });
                        }

                        if let Some(text) = text {
                            ui.send_message(TextMessage::text(self.text, text.clone()));
                        }

                        ui.send_message(WindowMessage::open_modal(self.handle()));
                    }
                    MessageBoxMessage::Close(_) => {
                        // Translate message box message into window message.
                        ui.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::Window(WindowMessage::Close),
                            destination: self.handle(),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, event: &OsEvent) {
        self.window.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.window.remove_ref(handle)
    }
}

pub struct MessageBoxBuilder<'b, M: 'static, C: 'static + Control<M, C>> {
    window_builder: WindowBuilder<M, C>,
    buttons: MessageBoxButtons,
    text: &'b str,
}

impl<'a, 'b, M: 'static, C: 'static + Control<M, C>> MessageBoxBuilder<'b, M, C> {
    pub fn new(window_builder: WindowBuilder<M, C>) -> Self {
        Self {
            window_builder,
            buttons: MessageBoxButtons::Ok,
            text: "",
        }
    }

    pub fn with_text(mut self, text: &'b str) -> Self {
        self.text = text;
        self
    }

    pub fn with_buttons(mut self, buttons: MessageBoxButtons) -> Self {
        self.buttons = buttons;
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let ok_yes;
        let mut no = Default::default();
        let mut cancel = Default::default();
        let text;
        let content = match self.buttons {
            MessageBoxButtons::Ok => {
                GridBuilder::new(WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(4.0)))
                            .with_text(self.text)
                            .with_wrap(true)
                            .build(ctx);
                        text
                    })
                    .with_child({
                        ok_yes = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(80.0)
                            .on_row(1)
                            .with_horizontal_alignment(HorizontalAlignment::Center))
                            .with_text("OK")
                            .build(ctx);
                        ok_yes
                    }))
                    .add_row(Row::stretch())
                    .add_row(Row::strict(25.0))
                    .add_column(Column::stretch())
                    .build(ctx)
            }
            MessageBoxButtons::YesNo => {
                GridBuilder::new(WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(WidgetBuilder::new())
                            .with_text(self.text)
                            .with_wrap(true)
                            .build(ctx);
                        text
                    })
                    .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                        .with_horizontal_alignment(HorizontalAlignment::Right)
                        .on_row(1)
                        .with_child({
                            ok_yes = ButtonBuilder::new(WidgetBuilder::new()
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)))
                                .with_text("Yes")
                                .build(ctx);
                            ok_yes
                        })
                        .with_child({
                            no = ButtonBuilder::new(WidgetBuilder::new()
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)))
                                .with_text("No")
                                .build(ctx);
                            no
                        }))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx)))
                    .add_row(Row::stretch())
                    .add_row(Row::strict(25.0))
                    .add_column(Column::stretch())
                    .build(ctx)
            }
            MessageBoxButtons::YesNoCancel => {
                GridBuilder::new(WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(WidgetBuilder::new())
                            .with_text(self.text)
                            .with_wrap(true)
                            .build(ctx);
                        text
                    })
                    .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                        .with_horizontal_alignment(HorizontalAlignment::Right)
                        .on_row(1)
                        .with_child({
                            ok_yes = ButtonBuilder::new(WidgetBuilder::new()
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)))
                                .with_text("Yes")
                                .build(ctx);
                            ok_yes
                        })
                        .with_child({
                            no = ButtonBuilder::new(WidgetBuilder::new()
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)))
                                .with_text("No")
                                .build(ctx);
                            no
                        })
                        .with_child({
                            cancel = ButtonBuilder::new(WidgetBuilder::new()
                                .with_width(80.0)
                                .with_margin(Thickness::uniform(1.0)))
                                .with_text("Cancel")
                                .build(ctx);
                            cancel
                        }))
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx)))
                    .add_row(Row::stretch())
                    .add_row(Row::strict(25.0))
                    .add_column(Column::stretch())
                    .build(ctx)
            }
        };

        if self.window_builder.widget_builder.min_size.is_none() {
            self.window_builder.widget_builder.min_size = Some(Vec2::new(200.0, 100.0));
        }

        let message_box = MessageBox {
            buttons: self.buttons,
            window: self.window_builder
                .with_content(content)
                .build_window(ctx),
            ok_yes,
            no,
            cancel,
            text,
        };

        let handle = ctx.add_node(UINode::MessageBox(message_box));

        // We must restrict picking because message box is modal.
        ctx.ui.push_picking_restriction(handle);

        handle
    }
}