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

use crate::button::Button;
use crate::file_browser::FileSelector;
use crate::text_box::TextBox;
use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        pool::Handle, reflect::prelude::*, uuid_provider, visitor::prelude::*, ComponentProvider,
    },
    define_widget_deref,
    file_browser::{FileSelectorBuilder, FileSelectorMessage, FileSelectorMode},
    grid::{Column, GridBuilder, Row},
    message::{MessageData, MessageDirection, UiMessage},
    text::TextMessage,
    text_box::TextBoxBuilder,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    window::{WindowAlignment, WindowBuilder, WindowMessage},
    BuildContext, Control, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_graph::constructor::{ConstructorProvider, GraphNodeConstructor};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorFieldMessage {
    Path(PathBuf),
}

impl MessageData for FileSelectorFieldMessage {}

define_widget_deref!(FileSelectorField);

uuid_provider!(FileSelectorField = "2dbda730-8a60-4f62-aee8-2ff0ccd15bf2");

#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct FileSelectorField {
    widget: Widget,
    path: PathBuf,
    path_field: Handle<TextBox>,
    select: Handle<Button>,
    file_selector: Handle<FileSelector>,
}

impl ConstructorProvider<UiNode, UserInterface> for FileSelectorField {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("File Selector Field", |ui| {
                FileSelectorFieldBuilder::new(WidgetBuilder::new().with_name("File Selector Field"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("File System")
    }
}

impl Control for FileSelectorField {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.path_field
                && message.direction() == MessageDirection::FromWidget
                && Path::new(text.as_str()) != self.path
            {
                ui.send(self.handle, FileSelectorFieldMessage::Path(text.into()));
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.select {
                let file_selector = FileSelectorBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .open(false)
                        .can_minimize(false),
                )
                .with_path(self.path.clone())
                .with_root(std::env::current_dir().unwrap_or_default())
                .with_mode(FileSelectorMode::Open)
                .build(&mut ui.build_ctx());

                self.file_selector = file_selector;

                ui.send(
                    file_selector,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: true,
                        focus_content: true,
                    },
                );
            }
        } else if let Some(FileSelectorFieldMessage::Path(new_path)) = message.data_for(self.handle)
        {
            if &self.path != new_path {
                self.path.clone_from(new_path);
                ui.send(
                    self.path_field,
                    TextMessage::Text(self.path.to_string_lossy().to_string()),
                );

                ui.send_message(message.reverse());
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        if let Some(FileSelectorMessage::Commit(new_path)) = message.data() {
            if message.destination() == self.file_selector {
                ui.send(
                    self.handle,
                    FileSelectorFieldMessage::Path(new_path.clone()),
                );
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.file_selector {
                ui.send(self.file_selector, WidgetMessage::Remove);
            }
        }
    }
}

pub struct FileSelectorFieldBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
}

impl FileSelectorFieldBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let select;
        let path_field;
        let field = FileSelectorField {
            widget: self
                .widget_builder
                .with_preview_messages(true)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                path_field = TextBoxBuilder::new(WidgetBuilder::new().on_column(0))
                                    .with_text(self.path.to_string_lossy())
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                path_field
                            })
                            .with_child({
                                select = ButtonBuilder::new(
                                    WidgetBuilder::new().on_column(1).with_width(25.0),
                                )
                                .with_text("...")
                                .build(ctx);
                                select
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                )
                .build(ctx),
            path: self.path,
            path_field,
            select,
            file_selector: Default::default(),
        };

        ctx.add_node(UiNode::new(field))
    }
}
