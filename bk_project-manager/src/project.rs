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

use crate::{
    make_button,
    settings::{Project, Settings},
    utils,
};
use fyrox::{
    core::pool::Handle,
    gui::{
        button::ButtonMessage,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        path::{PathEditorBuilder, PathEditorMessage},
        stack_panel::StackPanelBuilder,
        style::{self, resource::StyleResourceExt},
        text::{TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        utils::make_dropdown_list_option,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use std::path::PathBuf;

enum Style {
    TwoD,
    ThreeD,
}

impl Style {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::TwoD,
            1 => Self::ThreeD,
            _ => unreachable!(),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Style::TwoD => "2d",
            Style::ThreeD => "3d",
        }
    }
}

enum Vcs {
    None,
    Git,
    Mercurial,
    Pijul,
    Fossil,
}

impl Vcs {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::None,
            1 => Self::Git,
            2 => Self::Mercurial,
            3 => Self::Pijul,
            4 => Self::Fossil,
            _ => unreachable!(),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Vcs::None => "none",
            Vcs::Git => "git",
            Vcs::Mercurial => "hg",
            Vcs::Pijul => "pijul",
            Vcs::Fossil => "fossil",
        }
    }
}

pub struct ProjectWizard {
    pub window: Handle<UiNode>,
    create: Handle<UiNode>,
    cancel: Handle<UiNode>,
    path_field: Handle<UiNode>,
    name_field: Handle<UiNode>,
    style_field: Handle<UiNode>,
    vcs_field: Handle<UiNode>,
    name: String,
    style: Style,
    vcs: Vcs,
    path: PathBuf,
    validation_text: Handle<UiNode>,
}

fn make_text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(Thickness {
                left: 5.0,
                top: 1.0,
                right: 1.0,
                bottom: 1.0,
            })
            .on_row(row),
    )
    .with_text(text)
    .build(ctx)
}

impl ProjectWizard {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let path_field = PathEditorBuilder::new(
            WidgetBuilder::new()
                .with_height(22.0)
                .with_margin(Thickness::uniform(1.0))
                .on_row(0)
                .on_column(1),
        )
        .with_path("./")
        .build(ctx);

        let name_field = TextBoxBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .on_row(1)
                .on_column(1),
        )
        .with_text_commit_mode(TextCommitMode::Immediate)
        .with_text("MyProject")
        .build(ctx);

        let style_field = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(2)
                .on_column(1),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "2D"),
            make_dropdown_list_option(ctx, "3D"),
        ])
        .with_selected(1)
        .build(ctx);

        let vcs_field = DropdownListBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_height(22.0)
                .on_row(3)
                .on_column(1),
        )
        .with_items(vec![
            make_dropdown_list_option(ctx, "None"),
            make_dropdown_list_option(ctx, "Git"),
            make_dropdown_list_option(ctx, "Mercurial"),
            make_dropdown_list_option(ctx, "Pijul"),
            make_dropdown_list_option(ctx, "Fossil"),
        ])
        .with_selected(1)
        .build(ctx);

        let create = make_button("Create", 100.0, 22.0, 0, 0, 0, None, ctx);
        let cancel = make_button("Cancel", 100.0, 22.0, 0, 0, 0, None, ctx);
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_child(create)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_child(make_text("Path", 0, ctx))
                .with_child(path_field)
                .with_child(make_text("Name", 1, ctx))
                .with_child(name_field)
                .with_child(make_text("Style", 2, ctx))
                .with_child(style_field)
                .with_child(make_text("Version Control", 3, ctx))
                .with_child(vcs_field),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        let validation_text = TextBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_foreground(ctx.style.property(style::Style::BRUSH_ERROR)),
        )
        .with_font_size(12.0.into())
        .with_wrap(WrapMode::Word)
        .build(ctx);

        let outer_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(grid)
                .with_child(validation_text)
                .with_child(buttons),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(180.0))
            .with_content(outer_grid)
            .open(false)
            .with_title(WindowTitle::text("Project Wizard"))
            .build(ctx);

        ctx.send_message(WindowMessage::open_modal(
            window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        Self {
            window,
            name: "MyProject".to_string(),
            style: Style::ThreeD,
            vcs: Vcs::Git,
            create,
            cancel,
            path_field,
            name_field,
            style_field,
            vcs_field,
            path: Default::default(),
            validation_text,
        }
    }

    fn close_and_remove(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    fn validate(&self, ui: &UserInterface) {
        let is_valid = match fyrox_template_core::check_name(&self.name) {
            Ok(_) => true,
            Err(err) => {
                ui.send_message(TextMessage::text(
                    self.validation_text,
                    MessageDirection::ToWidget,
                    err.to_string(),
                ));
                false
            }
        };

        ui.send_message(WidgetMessage::visibility(
            self.validation_text,
            MessageDirection::ToWidget,
            !is_valid,
        ));

        ui.send_message(WidgetMessage::enabled(
            self.create,
            MessageDirection::ToWidget,
            is_valid,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        settings: &mut Settings,
    ) -> bool {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create {
                let _ = fyrox_template_core::init_project(
                    &self.path,
                    &self.name,
                    self.style.as_str(),
                    self.vcs.as_str(),
                    true,
                );
                settings.projects.push(Project {
                    manifest_path: utils::folder_to_manifest_path(&self.path.join(&self.name)),
                    name: self.name.clone(),
                    hot_reload: false,
                });
                self.close_and_remove(ui);
                return true;
            } else if message.destination() == self.cancel {
                self.close_and_remove(ui);
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.name_field
            {
                self.name.clone_from(text);
                self.validate(ui);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.style_field {
                    self.style = Style::from_index(*index);
                } else if message.destination() == self.vcs_field {
                    self.vcs = Vcs::from_index(*index);
                }
            }
        } else if let Some(PathEditorMessage::Path(path)) = message.data() {
            if message.destination() == self.path_field
                && message.direction() == MessageDirection::FromWidget
            {
                self.path.clone_from(path);
            }
        }
        false
    }
}
