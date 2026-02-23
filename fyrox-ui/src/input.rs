// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! input box is a window that is used to show standard confirmation/information dialogues, for example, closing a document with
//! unsaved changes. It has a title, some text, and a fixed set of buttons (Yes, No, Cancel in different combinations). See
//! [`InputBox`] docs for more info and usage examples.

use crate::{
    button::{Button, ButtonBuilder, ButtonMessage},
    control_trait_proxy_impls,
    core::{
        algebra::Vector2, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        variable::InheritableVariable, visitor::prelude::*,
    },
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    message::{KeyCode, MessageData, UiMessage},
    stack_panel::StackPanelBuilder,
    text::{Text, TextBuilder, TextMessage},
    text_box::{TextBox, TextBoxBuilder, TextCommitMode},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment,
};
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::ops::{Deref, DerefMut};

/// A set of messages that can be used to communicate with input boxes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputBoxMessage {
    /// A message that can be used to open input box, and optionally change its title and/or text.
    Open {
        /// If [`Some`], the input box title will be set to the new value.
        title: Option<String>,
        /// If [`Some`], the input box text will be set to the new value.
        text: Option<String>,
        /// If [`Some`], the input box value will be set to the new value.
        value: Option<String>,
    },
    /// A message that can be used to close a input box with some result. It can also be read to get the changes
    /// from the UI. See [`InputBox`] docs for examples.
    Close(InputBoxResult),
}
impl MessageData for InputBoxMessage {}

impl InputBoxMessage {
    pub fn open_as_is() -> Self {
        Self::Open {
            title: None,
            text: None,
            value: None,
        }
    }
}

/// A set of possible reasons why a input box was closed.
#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Hash, Debug)]
pub enum InputBoxResult {
    /// `Ok` button was pressed.
    Ok(String),
    /// `Cancel` button was pressed.
    Cancel,
}

/// Input box is a window that is used to show standard input dialogues, for example, a rename dialog.
/// It has a title, some description text, input field and `Ok` + `Cancel` buttons.
///
/// ## Styling
///
/// There's no way to change the style of the input box, nor add some widgets to it. If you need a
/// custom input box, then you need to create your own widget. This input box is meant to be used as
/// a standard dialog box for standard situations in the UI.
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "6b7b6b82-939b-4f98-9bb9-9bd19ce68b21")]
#[reflect(derived_type = "UiNode")]
pub struct InputBox {
    /// Base window of the input box.
    #[component(include)]
    pub window: Window,
    /// A handle of `Ok`/`Yes` buttons.
    pub ok: InheritableVariable<Handle<Button>>,
    /// A handle of `Cancel` button.
    pub cancel: InheritableVariable<Handle<Button>>,
    /// A handle of text widget.
    pub text: InheritableVariable<Handle<Text>>,
    pub value_box: InheritableVariable<Handle<TextBox>>,
    pub value: String,
}

impl ConstructorProvider<UiNode, UserInterface> for InputBox {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("Input Box", |ui| {
                InputBoxBuilder::new(WindowBuilder::new(
                    WidgetBuilder::new().with_name("Input Box"),
                ))
                .build(&mut ui.build_ctx())
                .to_base()
                .into()
            })
            .with_group("Input")
    }
}

impl Deref for InputBox {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for InputBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

impl InputBox {
    fn close_ok(&self, ui: &UserInterface) {
        ui.send(
            self.handle(),
            InputBoxMessage::Close(InputBoxResult::Ok(self.value.clone())),
        );
    }
}

impl Control for InputBox {
    control_trait_proxy_impls!(window);

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data_from(*self.ok) {
            self.close_ok(ui);
        } else if let Some(ButtonMessage::Click) = message.data_from(*self.cancel) {
            ui.send(
                self.handle(),
                InputBoxMessage::Close(InputBoxResult::Cancel),
            );
        } else if let Some(msg) = message.data_for::<InputBoxMessage>(self.handle) {
            match msg {
                InputBoxMessage::Open { title, text, value } => {
                    if let Some(title) = title {
                        ui.send(
                            self.handle(),
                            WindowMessage::Title(WindowTitle::text(title.clone())),
                        );
                    }

                    if let Some(text) = text {
                        ui.send(*self.text, TextMessage::Text(text.clone()));
                    }

                    if let Some(value) = value {
                        ui.send(*self.value_box, TextMessage::Text(value.clone()));
                    }

                    ui.send(
                        self.handle(),
                        WindowMessage::Open {
                            alignment: WindowAlignment::Center,
                            modal: true,
                            focus_content: false,
                        },
                    );

                    ui.send(*self.value_box, WidgetMessage::Focus);

                    ui.send_message(message.reverse());
                }
                InputBoxMessage::Close(_) => {
                    // Translate input box message into window message.
                    ui.send(self.handle(), WindowMessage::Close);

                    ui.send_message(message.reverse());
                }
            }
        } else if let Some(TextMessage::Text(text)) = message.data_from(*self.value_box) {
            self.value = text.clone();
        } else if let Some(WidgetMessage::KeyDown(code)) = message.data() {
            if matches!(*code, KeyCode::Enter | KeyCode::NumpadEnter) {
                self.close_ok(ui);
            }
        }
    }
}

/// Creates [`InputBox`] widgets and adds them to the user interface.
pub struct InputBoxBuilder<'b> {
    window_builder: WindowBuilder,
    text: &'b str,
    value: String,
}

impl<'b> InputBoxBuilder<'b> {
    /// Creates new builder instance. `window_builder` could be used to customize the look of your input box.
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            text: "",
            value: Default::default(),
        }
    }

    /// Sets a desired text of the input box.
    pub fn with_text(mut self, text: &'b str) -> Self {
        self.text = text;
        self
    }

    /// Sets the desired value of the input box.
    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }

    /// Finished input box building and adds it to the user interface.
    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<InputBox> {
        let ok;
        let cancel;
        let text;
        let value_box;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    text =
                        TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(4.0)))
                            .with_text(self.text)
                            .with_wrap(WrapMode::Word)
                            .build(ctx);
                    text
                })
                .with_child({
                    value_box = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::top(4.0))
                            .with_tab_index(Some(0))
                            .on_row(1)
                            .with_min_size(Vector2::new(f32::INFINITY, 24.0)),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text_commit_mode(TextCommitMode::Immediate)
                    .with_text(&self.value)
                    .build(ctx);
                    value_box
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::top(4.0))
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .on_row(2)
                            .with_child({
                                ok = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_tab_index(Some(1))
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(80.0)
                                        .with_horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .with_text("OK")
                                .build(ctx);
                                ok
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_tab_index(Some(2))
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_width(80.0)
                                        .with_horizontal_alignment(HorizontalAlignment::Center),
                                )
                                .with_text("Cancel")
                                .build(ctx);
                                cancel
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                )
                .with_margin(Thickness::uniform(4.0)),
        )
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_row(Row::strict(30.0))
        .add_column(Column::stretch())
        .build(ctx);

        if self.window_builder.widget_builder.min_size.is_none() {
            self.window_builder.widget_builder.min_size = Some(Vector2::new(200.0, 100.0));
        }

        self.window_builder.widget_builder.handle_os_events = true;

        let input_box = InputBox {
            window: self.window_builder.with_content(content).build_window(ctx),
            ok: ok.into(),
            cancel: cancel.into(),
            text: text.into(),
            value_box: value_box.into(),
            value: self.value,
        };

        ctx.add(input_box)
    }
}
