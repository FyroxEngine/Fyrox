use crate::formatted_text::WrapMode;
use crate::{
    button::ButtonBuilder,
    core::{algebra::Vector2, pool::Handle},
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, MessageBoxMessage, MessageDirection, OsEvent, TextMessage, UiMessage,
        UiMessageData, WindowMessage,
    },
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowTitle},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, RestrictionEntry,
    Thickness, UiNode, UserInterface,
};
use std::ops::{Deref, DerefMut};

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

#[derive(Clone)]
pub struct MessageBox {
    window: Window,
    buttons: MessageBoxButtons,
    ok_yes: Handle<UiNode>,
    no: Handle<UiNode>,
    cancel: Handle<UiNode>,
    text: Handle<UiNode>,
}

impl Deref for MessageBox {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for MessageBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

// Message box extends Window widget so it delegates most of calls
// to inner window.
impl Control for MessageBox {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.window.resolve(node_map);
        node_map.resolve(&mut self.ok_yes);
        node_map.resolve(&mut self.no);
        node_map.resolve(&mut self.cancel);
        node_map.resolve(&mut self.text);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32) {
        self.window.update(dt);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.ok_yes {
                    let result = match self.buttons {
                        MessageBoxButtons::Ok => MessageBoxResult::Ok,
                        MessageBoxButtons::YesNo => MessageBoxResult::Yes,
                        MessageBoxButtons::YesNoCancel => MessageBoxResult::Yes,
                    };
                    ui.send_message(MessageBoxMessage::close(
                        self.handle,
                        MessageDirection::ToWidget,
                        result,
                    ));
                } else if message.destination() == self.cancel {
                    ui.send_message(MessageBoxMessage::close(
                        self.handle(),
                        MessageDirection::ToWidget,
                        MessageBoxResult::Cancel,
                    ));
                } else if message.destination() == self.no {
                    ui.send_message(MessageBoxMessage::close(
                        self.handle(),
                        MessageDirection::ToWidget,
                        MessageBoxResult::No,
                    ));
                }
            }
            UiMessageData::MessageBox(msg) => {
                match msg {
                    MessageBoxMessage::Open { title, text } => {
                        if let Some(title) = title {
                            ui.send_message(WindowMessage::title(
                                self.handle(),
                                MessageDirection::ToWidget,
                                WindowTitle::Text(title.clone()),
                            ));
                        }

                        if let Some(text) = text {
                            ui.send_message(TextMessage::text(
                                self.text,
                                MessageDirection::ToWidget,
                                text.clone(),
                            ));
                        }

                        ui.send_message(WindowMessage::open_modal(
                            self.handle(),
                            MessageDirection::ToWidget,
                            true,
                        ));
                    }
                    MessageBoxMessage::Close(_) => {
                        // Translate message box message into window message.
                        ui.send_message(WindowMessage::close(
                            self.handle(),
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        self.window.remove_ref(handle)
    }
}

pub struct MessageBoxBuilder<'b> {
    window_builder: WindowBuilder,
    buttons: MessageBoxButtons,
    text: &'b str,
}

impl<'a, 'b> MessageBoxBuilder<'b> {
    pub fn new(window_builder: WindowBuilder) -> Self {
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

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let ok_yes;
        let mut no = Default::default();
        let mut cancel = Default::default();
        let text;
        let content = match self.buttons {
            MessageBoxButtons::Ok => GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(4.0)),
                        )
                        .with_text(self.text)
                        .with_wrap(WrapMode::Word)
                        .build(ctx);
                        text
                    })
                    .with_child({
                        ok_yes = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .with_width(80.0)
                                .on_row(1)
                                .with_horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .with_text("OK")
                        .build(ctx);
                        ok_yes
                    }),
            )
            .add_row(Row::stretch())
            .add_row(Row::strict(25.0))
            .add_column(Column::stretch())
            .build(ctx),
            MessageBoxButtons::YesNo => GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(WidgetBuilder::new())
                            .with_text(self.text)
                            .with_wrap(WrapMode::Word)
                            .build(ctx);
                        text
                    })
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_row(1)
                                .with_child({
                                    ok_yes = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(80.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Yes")
                                    .build(ctx);
                                    ok_yes
                                })
                                .with_child({
                                    no = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(80.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("No")
                                    .build(ctx);
                                    no
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
            )
            .add_row(Row::stretch())
            .add_row(Row::strict(25.0))
            .add_column(Column::stretch())
            .build(ctx),
            MessageBoxButtons::YesNoCancel => GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
                        text = TextBuilder::new(WidgetBuilder::new())
                            .with_text(self.text)
                            .with_wrap(WrapMode::Word)
                            .build(ctx);
                        text
                    })
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_row(1)
                                .with_child({
                                    ok_yes = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(80.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Yes")
                                    .build(ctx);
                                    ok_yes
                                })
                                .with_child({
                                    no = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(80.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("No")
                                    .build(ctx);
                                    no
                                })
                                .with_child({
                                    cancel = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(80.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Cancel")
                                    .build(ctx);
                                    cancel
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
            )
            .add_row(Row::stretch())
            .add_row(Row::strict(25.0))
            .add_column(Column::stretch())
            .build(ctx),
        };

        if self.window_builder.widget_builder.min_size.is_none() {
            self.window_builder.widget_builder.min_size = Some(Vector2::new(200.0, 100.0));
        }

        self.window_builder.widget_builder.handle_os_events = true;

        let is_open = self.window_builder.open;

        let message_box = MessageBox {
            buttons: self.buttons,
            window: self.window_builder.with_content(content).build_window(ctx),
            ok_yes,
            no,
            cancel,
            text,
        };

        let handle = ctx.add_node(UiNode::new(message_box));

        if is_open {
            // We must restrict picking because message box is modal.
            ctx.ui
                .push_picking_restriction(RestrictionEntry { handle, stop: true });
        }

        handle
    }
}
