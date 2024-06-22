//! Message box is a window that is used to show standard confirmation/information dialogues, for example, closing a document with
//! unsaved changes. It has a title, some text, and a fixed set of buttons (Yes, No, Cancel in different combinations). See
//! [`MessageBox`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    draw::DrawingContext,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, OsEvent, UiMessage},
    stack_panel::StackPanelBuilder,
    text::{TextBuilder, TextMessage},
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, Orientation, RestrictionEntry, Thickness, UiNode,
    UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use std::ops::{Deref, DerefMut};

/// A set of messages that can be used to communicate with message boxes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageBoxMessage {
    /// A message that can be used to open message box, and optionally change its title and/or text.
    Open {
        /// If [`Some`], a message box title will be set to the new value.
        title: Option<String>,
        /// If [`Some`], a message box text will be set to the new value.
        text: Option<String>,
    },
    /// A message that can be used to close a message box with some result. It can also be read to get the changes
    /// from the UI. See [`MessageBox`] docs for examples.
    Close(MessageBoxResult),
}

impl MessageBoxMessage {
    define_constructor!(
        /// Creates [`MessageBoxMessage::Open`] message.
        MessageBoxMessage:Open => fn open(title: Option<String>, text: Option<String>), layout: false
    );
    define_constructor!(
        /// Creates [`MessageBoxMessage::Close`] message.
        MessageBoxMessage:Close => fn close(MessageBoxResult), layout: false
    );
}

/// A set of possible reasons why a message box was closed.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum MessageBoxResult {
    /// `Ok` button was pressed. It can be emitted only if your message box was created with [`MessageBoxButtons::Ok`].
    Ok,
    /// `No` button was pressed. It can be emitted only if your message box was created with [`MessageBoxButtons::YesNo`] or
    /// [`MessageBoxButtons::YesNoCancel`].
    No,
    /// `Yes` button was pressed. It can be emitted only if your message box was created with [`MessageBoxButtons::YesNo`] or
    /// [`MessageBoxButtons::YesNoCancel`].
    Yes,
    /// `Cancel` button was pressed. It can be emitted only if your message box was created with [`MessageBoxButtons::YesNoCancel`].
    Cancel,
}

/// A fixed set of possible buttons in a message box.
#[derive(Copy, Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug, Visit, Reflect, Default)]
pub enum MessageBoxButtons {
    /// Only `Ok` button. It is typically used to show a message with results of some finished action.
    #[default]
    Ok,
    /// `Yes` and `No` buttons. It is typically used to show a message to ask a user if they are want to continue or not.
    YesNo,
    /// `Yes`, `No`, `Cancel` buttons. It is typically used to show a message to ask a user if they are want to confirm action,
    /// refuse, cancel the next action completely.
    YesNoCancel,
}

/// Message box is a window that is used to show standard confirmation/information dialogues, for example, closing a document with
/// unsaved changes. It has a title, some text, and a fixed set of buttons (Yes, No, Cancel in different combinations).
///
/// ## Examples
///
/// A simple message box with two buttons (Yes and No) and some text can be created like so:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     messagebox::{MessageBoxBuilder, MessageBoxButtons},
/// #     widget::WidgetBuilder,
/// #     window::WindowBuilder,
/// #     BuildContext, UiNode,
/// # };
/// #
/// fn create_message_box(ctx: &mut BuildContext) -> Handle<UiNode> {
///     MessageBoxBuilder::new(WindowBuilder::new(WidgetBuilder::new()))
///         .with_buttons(MessageBoxButtons::YesNo)
///         .with_text("Do you want to save your changes?")
///         .build(ctx)
/// }
/// ```
///
/// To "catch" the moment when any of the buttons will be clicked, you should listen for [`MessageBoxMessage::Close`] message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::UiMessage,
/// #     messagebox::{MessageBoxMessage, MessageBoxResult},
/// #     UiNode,
/// # };
/// # fn on_ui_message(my_message_box: Handle<UiNode>, message: &UiMessage) {
/// if message.destination() == my_message_box {
///     if let Some(MessageBoxMessage::Close(result)) = message.data() {
///         match result {
///             MessageBoxResult::No => {
///                 println!("No");
///             }
///             MessageBoxResult::Yes => {
///                 println!("Yes");
///             }
///             _ => (),
///         }
///     }
/// }
/// # }
/// ```
///
/// To open an existing message box, use [`MessageBoxMessage::Open`]. You can optionally specify a new title and a text for the
/// message box:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, message::MessageDirection, messagebox::MessageBoxMessage, UiNode,
/// #     UserInterface,
/// # };
/// # fn open_message_box(my_message_box: Handle<UiNode>, ui: &UserInterface) {
/// ui.send_message(MessageBoxMessage::open(
///     my_message_box,
///     MessageDirection::ToWidget,
///     Some("This is the new title".to_string()),
///     Some("This is the new text".to_string()),
/// ))
/// # }
/// ```
///
/// ## Styling
///
/// There's no way to change the style of the message box, nor add some widgets to it. If you need custom message box, then you
/// need to create your own widget. This message box is meant to be used as a standard dialog box for standard situations in UI.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct MessageBox {
    /// Base window of the message box.
    #[component(include)]
    pub window: Window,
    /// Current set of buttons of the message box.
    pub buttons: InheritableVariable<MessageBoxButtons>,
    /// A handle of `Ok`/`Yes` buttons.
    pub ok_yes: InheritableVariable<Handle<UiNode>>,
    /// A handle of `No` button.
    pub no: InheritableVariable<Handle<UiNode>>,
    /// A handle of `Cancel` button.
    pub cancel: InheritableVariable<Handle<UiNode>>,
    /// A handle of text widget.
    pub text: InheritableVariable<Handle<UiNode>>,
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

uuid_provider!(MessageBox = "b14c0012-4383-45cf-b9a1-231415d95373");

// Message box extends Window widget so it delegates most of calls
// to inner window.
impl Control for MessageBox {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == *self.ok_yes {
                let result = match *self.buttons {
                    MessageBoxButtons::Ok => MessageBoxResult::Ok,
                    MessageBoxButtons::YesNo => MessageBoxResult::Yes,
                    MessageBoxButtons::YesNoCancel => MessageBoxResult::Yes,
                };
                ui.send_message(MessageBoxMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                    result,
                ));
            } else if message.destination() == *self.cancel {
                ui.send_message(MessageBoxMessage::close(
                    self.handle(),
                    MessageDirection::ToWidget,
                    MessageBoxResult::Cancel,
                ));
            } else if message.destination() == *self.no {
                ui.send_message(MessageBoxMessage::close(
                    self.handle(),
                    MessageDirection::ToWidget,
                    MessageBoxResult::No,
                ));
            }
        } else if let Some(msg) = message.data::<MessageBoxMessage>() {
            match msg {
                MessageBoxMessage::Open { title, text } => {
                    if let Some(title) = title {
                        ui.send_message(WindowMessage::title(
                            self.handle(),
                            MessageDirection::ToWidget,
                            WindowTitle::text(title.clone()),
                        ));
                    }

                    if let Some(text) = text {
                        ui.send_message(TextMessage::text(
                            *self.text,
                            MessageDirection::ToWidget,
                            text.clone(),
                        ));
                    }

                    ui.send_message(WindowMessage::open_modal(
                        self.handle(),
                        MessageDirection::ToWidget,
                        true,
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
}

/// Creates [`MessageBox`] widgets and adds them to user interface.
pub struct MessageBoxBuilder<'b> {
    window_builder: WindowBuilder,
    buttons: MessageBoxButtons,
    text: &'b str,
}

impl<'b> MessageBoxBuilder<'b> {
    /// Creates new builder instace. `window_builder` could be used to customize the look of you message box.
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            buttons: MessageBoxButtons::Ok,
            text: "",
        }
    }

    /// Sets a desired text of the message box.
    pub fn with_text(mut self, text: &'b str) -> Self {
        self.text = text;
        self
    }

    /// Sets a desired set of buttons of the message box.
    pub fn with_buttons(mut self, buttons: MessageBoxButtons) -> Self {
        self.buttons = buttons;
        self
    }

    /// Finished message box building and adds it to the user interface.
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
                    })
                    .with_margin(Thickness::uniform(5.0)),
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
                    )
                    .with_margin(Thickness::uniform(5.0)),
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
                    )
                    .with_margin(Thickness::uniform(5.0)),
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
            buttons: self.buttons.into(),
            window: self.window_builder.with_content(content).build_window(ctx),
            ok_yes: ok_yes.into(),
            no: no.into(),
            cancel: cancel.into(),
            text: text.into(),
        };

        let handle = ctx.add_node(UiNode::new(message_box));

        if is_open {
            // We must restrict picking because message box is modal.
            ctx.push_picking_restriction(RestrictionEntry { handle, stop: true });
        }

        handle
    }
}
