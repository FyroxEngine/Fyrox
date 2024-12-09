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

use crate::{settings::Project, utils, utils::make_button};
use fyrox::{
    core::{log::Log, pool::Handle},
    gui::{
        button::ButtonMessage,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        utils::make_dropdown_list_option,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};

#[allow(dead_code)]
pub struct UpgradeTool {
    window: Handle<UiNode>,
    version: Handle<UiNode>,
    version_selector: Handle<UiNode>,
    upgrade: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl UpgradeTool {
    pub fn new(project: &Project, ctx: &mut BuildContext) -> Self {
        let version = utils::fyrox_version_or_default(&project.manifest_path);

        let version = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text(format!("Current Engine Version: {version}"))
        .build(ctx);

        let version_selector = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(0))
                .on_row(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "Specific"),
            make_dropdown_list_option(ctx, "Latest"),
            make_dropdown_list_option(ctx, "Nightly"),
        ])
        .with_selected(0)
        .build(ctx);

        let upgrade;
        let cancel;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .on_row(3)
                .with_child({
                    upgrade = make_button("Upgrade", 130.0, 25.0, 1, 0, 0, None, ctx);
                    upgrade
                })
                .with_child({
                    cancel = make_button("Cancel", 130.0, 25.0, 2, 0, 0, None, ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(version)
                .with_child(version_selector)
                .with_child(buttons),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(200.0))
            .open(false)
            .with_title(WindowTitle::text("Upgrade Project"))
            .with_content(content)
            .with_remove_on_close(true)
            .build(ctx);

        ctx.sender()
            .send(WindowMessage::open_modal(
                window,
                MessageDirection::ToWidget,
                true,
                true,
            ))
            .unwrap();

        Self {
            window,
            version,
            version_selector,
            upgrade,
            cancel,
        }
    }

    pub fn handle_ui_message(
        self,
        message: &UiMessage,
        ui: &mut UserInterface,
        project: &Project,
    ) -> Option<Self> {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.upgrade {
                Log::verify(fyrox_template_core::upgrade_project(
                    &project.manifest_path,
                    "",
                    false,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                return None;
            }
        }

        Some(self)
    }
}
