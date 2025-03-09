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

//! A widget for showing handles in the tile set editor.

use crate::{
    fyrox::gui::{
        button::ButtonMessage,
        define_constructor, define_widget_deref,
        grid::{Column, GridBuilder, Row},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        widget::Widget,
        Control, Orientation,
    },
    send_sync_message,
};

use super::*;

const BUTTON_SIZE: f32 = 12.0;

#[derive(Debug, PartialEq, Clone)]
pub enum TileHandleEditorMessage {
    Goto(TileDefinitionHandle),
    OpenPalette(TileDefinitionHandle),
    Value(Option<TileDefinitionHandle>),
}

impl TileHandleEditorMessage {
    define_constructor!(TileHandleEditorMessage:Goto => fn goto(TileDefinitionHandle), layout: false);
    define_constructor!(TileHandleEditorMessage:OpenPalette => fn open_palette(TileDefinitionHandle), layout: false);
    define_constructor!(TileHandleEditorMessage:Value => fn value(Option<TileDefinitionHandle>), layout: false);
}

/// The widget for editing a [`TileDefinitionHandle`].
/// It has a button for focusing the tile map control panel on the tile represented
/// by this handle, and another button for focusing the tile set editor on that tile.
///
/// The value is displayed in a text box in the form "(x,y):(x,y)" where the first
/// pair is the page coordinates and the second pair is the tile coordinates.
/// When editing the handle, one need merely type four integers. Whatever
/// characters separate the integers are ignored, so "1 2 3 4" would be accepted.
#[derive(Debug, Default, Clone, Visit, Reflect, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "86513074-461d-4583-a214-fb84f5aacac1")]
#[reflect(derived_type = "UiNode")]
pub struct TileHandleField {
    widget: Widget,
    value: Option<TileDefinitionHandle>,
    field: Handle<UiNode>,
    palette_button: Handle<UiNode>,
    goto_button: Handle<UiNode>,
}

define_widget_deref!(TileHandleField);

fn value_to_string(value: Option<TileDefinitionHandle>) -> String {
    if let Some(v) = value {
        v.to_string()
    } else {
        "?".into()
    }
}

fn make_label(name: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new())
        .with_text(name)
        .build(ctx)
}

impl Control for TileHandleField {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        if message.direction() == MessageDirection::ToWidget {
            if let Some(TileHandleEditorMessage::Value(value)) = message.data() {
                self.value = *value;
                ui.send_message(TextMessage::text(
                    self.field,
                    MessageDirection::ToWidget,
                    value_to_string(*value),
                ));
                ui.send_message(message.reverse());
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if let Some(value) = self.value {
                if message.destination() == self.palette_button {
                    ui.send_message(TileHandleEditorMessage::open_palette(
                        self.handle(),
                        MessageDirection::FromWidget,
                        value,
                    ));
                } else if message.destination() == self.goto_button {
                    ui.send_message(TileHandleEditorMessage::goto(
                        self.handle(),
                        MessageDirection::FromWidget,
                        value,
                    ));
                }
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.flags == 0 {
                if let Ok(value) = text.parse() {
                    if self.value != Some(value) {
                        self.value = Some(value);
                        ui.send_message(TileHandleEditorMessage::value(
                            self.handle(),
                            MessageDirection::FromWidget,
                            self.value,
                        ));
                    }
                }
                send_sync_message(
                    ui,
                    TextMessage::text(
                        self.field,
                        MessageDirection::ToWidget,
                        value_to_string(self.value),
                    ),
                );
            }
        }
    }
}

pub struct TileHandleFieldBuilder {
    widget_builder: WidgetBuilder,
    label: String,
    value: Option<TileDefinitionHandle>,
}

impl TileHandleFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            label: String::default(),
            value: None,
        }
    }
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.into();
        self
    }
    pub fn with_value(mut self, value: Option<TileDefinitionHandle>) -> Self {
        self.value = value;
        self
    }
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let field = TextBoxBuilder::new(WidgetBuilder::new().on_column(1))
            .with_text(value_to_string(self.value))
            .build(ctx);
        let goto_button = make_drawing_mode_button(
            ctx,
            BUTTON_SIZE,
            BUTTON_SIZE,
            PICK_IMAGE.clone(),
            "Jump to tile",
            None,
        );
        let palette_button = make_drawing_mode_button(
            ctx,
            BUTTON_SIZE,
            BUTTON_SIZE,
            PALETTE_IMAGE.clone(),
            "Open in palette window",
            None,
        );
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(2)
                .with_child(goto_button)
                .with_child(palette_button),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_label(&self.label, ctx))
                .with_child(field)
                .with_child(buttons),
        )
        .add_row(Row::auto())
        .add_column(Column::strict(FIELD_LABEL_WIDTH))
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.add_node(UiNode::new(TileHandleField {
            widget: self.widget_builder.with_child(content).build(ctx),
            value: self.value,
            field,
            goto_button,
            palette_button,
        }))
    }
}
