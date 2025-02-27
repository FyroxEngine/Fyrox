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
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        text_box::TextBoxBuilder,
        utils::make_dropdown_list_option,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};

enum Version {
    Specific(String),
    Local,
    Nightly,
}

impl Version {
    fn index(&self) -> usize {
        match self {
            Version::Specific(_) => 0,
            Version::Local => 1,
            Version::Nightly => 2,
        }
    }

    fn as_string_version(&self) -> String {
        match self {
            Version::Specific(ver) => ver.clone(),
            Version::Local => "latest".to_string(),
            Version::Nightly => "nightly".to_string(),
        }
    }
}

pub struct UpgradeTool {
    window: Handle<UiNode>,
    version_type_selector: Handle<UiNode>,
    upgrade: Handle<UiNode>,
    cancel: Handle<UiNode>,
    selected_version: Version,
    version_input_field: Handle<UiNode>,
}

impl UpgradeTool {
    pub fn new(project: &Project, ctx: &mut BuildContext) -> Self {
        let dependency = utils::fyrox_dependency_from_path(&project.manifest_path).unwrap();

        let version = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_text(format!(
            "Current Engine Version: {}\nSource: {}",
            dependency.req,
            dependency
                .source
                .as_ref()
                .map(|s| s.to_string())
                .or_else(|| dependency.path.as_ref().map(|s| s.to_string()))
                .unwrap_or_default()
        ))
        .build(ctx);

        let is_local = dependency.path.is_some();

        let is_git = dependency
            .source
            .as_ref()
            .is_some_and(|s| s.contains("https://github.com/FyroxEngine/Fyrox"));
        let selected_version = if is_local {
            Version::Local
        } else if is_git {
            Version::Nightly
        } else {
            Version::Specific(dependency.name.clone())
        };

        let version_type_selector = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(0))
                .on_row(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_close_on_selection(true)
        .with_items(vec![
            make_dropdown_list_option(ctx, "Specific"),
            make_dropdown_list_option(ctx, "Local"),
            make_dropdown_list_option(ctx, "Nightly"),
        ])
        .with_selected(selected_version.index())
        .build(ctx);

        let version_input_field = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_visibility(!is_local)
                .with_height(22.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_vertical_text_alignment(VerticalAlignment::Center)
        .with_text(dependency.req.to_string())
        .build(ctx);

        let controls = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(version)
                .with_child(version_type_selector)
                .with_child(version_input_field),
        )
        .build(ctx);

        let upgrade;
        let cancel;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .on_row(2)
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
                .with_child(controls)
                .with_child(buttons),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(130.0))
            .open(false)
            .with_title(WindowTitle::text("Upgrade Project"))
            .with_content(content)
            .with_remove_on_close(true)
            .build(ctx);

        ctx.send_message(WindowMessage::open_modal(
            window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        Self {
            window,
            version_type_selector,
            upgrade,
            cancel,
            selected_version,
            version_input_field,
        }
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        project: &Project,
        need_refresh: &mut bool,
    ) -> Option<Self> {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.upgrade {
                if let Some(parent) = project.manifest_path.parent() {
                    Log::verify(fyrox_template_core::upgrade_project(
                        parent,
                        &self.selected_version.as_string_version(),
                        matches!(self.selected_version, Version::Local),
                    ));
                }
                *need_refresh = true;
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
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
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.version_type_selector {
                match *index {
                    0 => {
                        self.selected_version = Version::Specific(
                            fyrox_template_core::CURRENT_ENGINE_VERSION.to_string(),
                        );
                    }
                    1 => {
                        self.selected_version = Version::Local;
                    }
                    2 => {
                        self.selected_version = Version::Nightly;
                    }
                    _ => (),
                }

                ui.send_message(WidgetMessage::visibility(
                    self.version_input_field,
                    MessageDirection::ToWidget,
                    matches!(self.selected_version, Version::Specific(_)),
                ))
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.version_input_field
                && message.direction() == MessageDirection::FromWidget
            {
                if let Version::Specific(ref mut version) = self.selected_version {
                    *version = text.clone();
                }
            }
        }

        Some(self)
    }
}
