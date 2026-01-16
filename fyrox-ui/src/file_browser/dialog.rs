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
use crate::{
    button::{ButtonBuilder, ButtonMessage},
    control_trait_proxy_impls,
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_widget_deref_proxy,
    grid::{Column, GridBuilder, Row},
    message::{MessageData, UiMessage},
    stack_panel::StackPanelBuilder,
    text::TextMessage,
    text_box::{EmptyTextPlaceholder, TextBoxBuilder},
    widget::{WidgetBuilder, WidgetMessage},
    window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
};

#[derive(Clone, PartialEq, Debug)]
pub enum FolderNameDialogMessage {
    Name(String),
}
impl MessageData for FolderNameDialogMessage {}

#[derive(Clone, Visit, Reflect, Default, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "832f63b8-1372-49b8-8ce5-7564920343a8")]
#[reflect(derived_type = "UiNode")]
pub struct FolderNameDialog {
    pub window: Window,
    pub folder_name_tb: Handle<UiNode>,
    pub folder_name: String,
    pub ok: Handle<Button>,
    pub cancel: Handle<Button>,
}

define_widget_deref_proxy!(FolderNameDialog, window);

impl Control for FolderNameDialog {
    control_trait_proxy_impls!(window);

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data_from(self.ok) {
            ui.post(
                self.handle(),
                FolderNameDialogMessage::Name(self.folder_name.clone()),
            );
            ui.send(self.handle(), WindowMessage::Close);
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            self.folder_name.clear();
            ui.send(self.handle(), WindowMessage::Close);
        } else if let Some(TextMessage::Text(text)) = message.data_from(self.folder_name_tb) {
            self.folder_name.clone_from(text);
        }
    }
}

impl FolderNameDialog {
    pub fn build_and_open(ui: &mut UserInterface) -> Handle<FolderNameDialog> {
        let ctx = &mut ui.build_ctx();
        let ok = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_width(80.0)
                .with_tab_index(Some(1)),
        )
        .with_text("OK")
        .build(ctx);

        let cancel = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_width(80.0)
                .with_tab_index(Some(2)),
        )
        .with_text("Cancel")
        .build(ctx);

        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_margin(Thickness::uniform(1.0))
                .with_height(23.0)
                .on_row(2)
                .with_child(ok)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let folder_name_tb = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_height(22.0)
                .on_row(0)
                .with_tab_index(Some(0)),
        )
        .with_empty_text_placeholder(EmptyTextPlaceholder::Text("Enter a new folder name"))
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(220.0).with_height(80.0))
            .open(false)
            .with_remove_on_close(true)
            .with_title(WindowTitle::text("New Folder Name"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(folder_name_tb)
                        .with_child(buttons),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build_window(ctx);

        let dialog = Self {
            window,
            folder_name_tb,
            folder_name: Default::default(),
            ok,
            cancel,
        };

        let handle = ctx.add(dialog);

        ui.send(
            handle,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: false,
            },
        );

        ui.send(folder_name_tb, WidgetMessage::Focus);

        handle
    }
}
