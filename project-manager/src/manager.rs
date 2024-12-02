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
    build::BuildWindow,
    project::ProjectWizard,
    settings::Settings,
    utils::{is_production_ready, load_image, make_button},
};
use fyrox::{
    core::{color::Color, log::Log, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::DecoratorBuilder,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        navigation::NavigationLayerBuilder,
        screen::ScreenBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use std::{path::Path, process::Stdio};

pub struct ProjectManager {
    create: Handle<UiNode>,
    import: Handle<UiNode>,
    projects: Handle<UiNode>,
    edit: Handle<UiNode>,
    run: Handle<UiNode>,
    delete: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    project_controls: Handle<UiNode>,
    hot_reload: Handle<UiNode>,
    download: Handle<UiNode>,
    selection: Option<usize>,
    pub settings: Settings,
    project_wizard: Option<ProjectWizard>,
    build_window: Option<BuildWindow>,
}

fn make_project_item(name: &str, path: &Path, ctx: &mut BuildContext) -> Handle<UiNode> {
    let icon = ImageBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_width(40.0)
            .with_height(40.0)
            .on_column(0),
    )
    .with_opt_texture(load_image(include_bytes!("../resources/icon.png")))
    .build(ctx);

    let item = GridBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .on_row(0)
                        .with_margin(Thickness::uniform(2.0))
                        .with_vertical_alignment(VerticalAlignment::Center),
                )
                .with_font_size(18.0.into())
                .with_text(name)
                .build(ctx),
            )
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .on_row(1)
                        .with_margin(Thickness::uniform(2.0))
                        .with_vertical_alignment(VerticalAlignment::Center),
                )
                .with_font_size(13.0.into())
                .with_text(path.to_string_lossy())
                .build(ctx),
            ),
    )
    .add_column(Column::auto())
    .add_row(Row::auto())
    .add_row(Row::auto())
    .build(ctx);

    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_child(
                    GridBuilder::new(WidgetBuilder::new().with_child(icon).with_child(item))
                        .add_column(Column::auto())
                        .add_column(Column::stretch())
                        .add_row(Row::auto())
                        .build(ctx),
                ),
        )
        .with_corner_radius(4.0f32.into()),
    )
    .build(ctx)
}

fn make_project_items(settings: &Settings, ctx: &mut BuildContext) -> Vec<Handle<UiNode>> {
    settings
        .projects
        .iter()
        .map(|project| make_project_item(&project.name, &project.manifest_path, ctx))
        .collect::<Vec<_>>()
}

impl ProjectManager {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let settings = Settings::load();

        let is_ready = is_production_ready();

        let download = ButtonBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_width(100.0)
                .with_height(26.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Download...")
        .build(ctx);

        let warning = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(!is_ready)
                .with_child(
                    TextBuilder::new(
                        WidgetBuilder::new()
                            .on_column(0)
                            .with_margin(Thickness::uniform(2.0))
                            .with_foreground(Brush::Solid(Color::RED).into()),
                    )
                    .with_text(
                        "Rust is not installed, please click the button at the right \
                        and follow build instructions for your platform.",
                    )
                    .with_font_size(18.0.into())
                    .with_wrap(WrapMode::Word)
                    .build(ctx),
                )
                .with_child(download),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::auto())
        .build(ctx);

        let create = make_button("+ Create", 100.0, 25.0, 0, ctx);
        let import = make_button("Import", 100.0, 25.0, 1, ctx);
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(2))
                .with_margin(Thickness::uniform(1.0))
                .with_height(25.0)
                .with_width(200.0),
        )
        .build(ctx);

        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_enabled(is_ready)
                .on_row(1)
                .with_child(create)
                .with_child(import)
                .with_child(search_bar),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let edit = make_button("Edit", 100.0, 25.0, 3, ctx);
        let run = make_button("Run", 100.0, 25.0, 4, ctx);
        let delete = make_button("Delete", 100.0, 25.0, 5, ctx);
        let hot_reload = CheckBoxBuilder::new(WidgetBuilder::new().with_tooltip(
            make_simple_tooltip(ctx, "Run the project with code hot reloading support. \
            Significantly reduces iteration times, but might result in subtle bugs due to experimental \
            and unsafe nature of code hot reloading."),
        ))
            .build(ctx);

        let project_controls = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_enabled(false)
                .on_column(1)
                .with_child(edit)
                .with_child(run)
                .with_child(delete),
        )
        .build(ctx);

        let projects = ListViewBuilder::new(
            WidgetBuilder::new()
                .with_enabled(is_ready)
                .with_tab_index(Some(6))
                .with_margin(Thickness::uniform(1.0))
                .on_column(0),
        )
        .with_items(make_project_items(&settings, ctx))
        .build(ctx);

        let inner_content = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(projects)
                .with_child(project_controls),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let main_content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(warning)
                .with_child(toolbar)
                .with_child(inner_content),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .build(ctx);

        let navigation_layer =
            NavigationLayerBuilder::new(WidgetBuilder::new().with_child(main_content)).build(ctx);

        ScreenBuilder::new(WidgetBuilder::new().with_child(
            BorderBuilder::new(WidgetBuilder::new().with_child(navigation_layer)).build(ctx),
        ))
        .build(ctx);

        Self {
            create,
            import,
            projects,
            edit,
            run,
            delete,
            search_bar,
            project_controls,
            hot_reload,
            download,
            selection: None,
            settings,
            project_wizard: None,
            build_window: None,
        }
    }

    fn refresh(&mut self, ui: &mut UserInterface) {
        let items = make_project_items(&self.settings, &mut ui.build_ctx());
        ui.send_message(ListViewMessage::items(
            self.projects,
            MessageDirection::ToWidget,
            items,
        ))
    }

    pub fn update(&mut self, ui: &mut UserInterface) {
        if let Some(build_window) = self.build_window.as_mut() {
            build_window.update(ui);
        }
    }

    fn on_button_click(&mut self, button: Handle<UiNode>, ui: &mut UserInterface) {
        if button == self.create {
            self.project_wizard = Some(ProjectWizard::new(&mut ui.build_ctx()));
        } else if button == self.import {
            // TODO: Import project.
        } else if button == self.download {
            let _ = open::that("https://rustup.rs/");
        }

        if let Some(index) = self.selection {
            if let Some(project) = self.settings.projects.get(index) {
                if button == self.edit {
                    let mut new_process = std::process::Command::new("cargo");
                    new_process
                        .current_dir(project.manifest_path.parent().unwrap())
                        .stderr(Stdio::piped())
                        .args(["run", "--package", "editor"]);

                    match new_process.spawn() {
                        Ok(mut new_process) => {
                            let mut build_window = BuildWindow::new(&mut ui.build_ctx());

                            build_window.listen(new_process.stderr.take().unwrap(), ui);

                            self.build_window = Some(build_window);
                        }
                        Err(e) => Log::err(format!("Failed to start the editor: {e:?}")),
                    }
                } else if button == self.run {
                    let mut new_process = std::process::Command::new("cargo");
                    new_process
                        .current_dir(project.manifest_path.parent().unwrap())
                        .stderr(Stdio::piped())
                        .args(["run", "--package", "executor"]);

                    match new_process.spawn() {
                        Ok(mut new_process) => {
                            let mut build_window = BuildWindow::new(&mut ui.build_ctx());

                            build_window.listen(new_process.stderr.take().unwrap(), ui);

                            self.build_window = Some(build_window);
                        }
                        Err(e) => Log::err(format!("Failed to start the game: {e:?}")),
                    }
                } else if button == self.delete {
                    if let Some(dir) = project.manifest_path.parent() {
                        let _ = std::fs::remove_dir_all(dir);
                    }
                    self.settings.projects.remove(index);
                    self.refresh(ui);
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(project_wizard) = self.project_wizard.as_mut() {
            if project_wizard.handle_ui_message(message, ui, &mut self.settings) {
                self.refresh(ui);
            }
        }

        if let Some(build_window) = self.build_window.as_mut() {
            build_window.handle_ui_message(message, ui);
        }

        if let Some(ButtonMessage::Click) = message.data() {
            self.on_button_click(message.destination, ui);
        } else if let Some(ListViewMessage::SelectionChanged(selection)) = message.data() {
            if message.destination() == self.projects
                && message.direction() == MessageDirection::FromWidget
            {
                self.selection.clone_from(&selection.first().cloned());

                ui.send_message(WidgetMessage::enabled(
                    self.project_controls,
                    MessageDirection::ToWidget,
                    !selection.is_empty(),
                ));
            }
        } else if let Some(SearchBarMessage::Text(_filter)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                // TODO: Filter projects.
                self.refresh(ui);
            }
        } else if let Some(CheckBoxMessage::Check(Some(_value))) = message.data() {
            if message.destination() == self.hot_reload
                && message.direction() == MessageDirection::FromWidget
            {
                // TODO: Switch to respective mode.
            }
        }
    }
}
