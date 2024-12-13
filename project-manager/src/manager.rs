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
    settings::{Project, Settings},
    upgrade::UpgradeTool,
    utils::{self, is_production_ready, load_image, make_button},
};
use fyrox::gui::Orientation;
use fyrox::{
    core::{color::Color, log::Log, pool::Handle, some_or_return},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::DecoratorBuilder,
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        list_view::{ListViewBuilder, ListViewMessage},
        log::LogPanel,
        message::{MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
        navigation::NavigationLayerBuilder,
        screen::ScreenBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        style::{resource::StyleResourceExt, Style},
        text::{TextBuilder, TextMessage},
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use fyrox_build_tools::{BuildCommand, BuildProfile};
use std::process::Stdio;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

enum Mode {
    Normal,
    Build {
        queue: VecDeque<BuildCommand>,
        process: Option<std::process::Child>,
        current_dir: PathBuf,
    },
}

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
    import_project_dialog: Handle<UiNode>,
    mode: Mode,
    search_text: String,
    log: LogPanel,
    open_log: Handle<UiNode>,
    message_count: Handle<UiNode>,
    deletion_confirmation_dialog: Handle<UiNode>,
    upgrade: Handle<UiNode>,
    upgrade_tool: Option<UpgradeTool>,
}

fn make_project_item(
    name: &str,
    path: &Path,
    hot_reload: bool,
    visible: bool,
    engine_version: &str,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let icon = ImageBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(4.0))
            .with_width(40.0)
            .with_height(40.0)
            .on_column(0),
    )
    .with_opt_texture(load_image(include_bytes!("../resources/icon.png")))
    .build(ctx);

    let hot_reload = ImageBuilder::new(
        WidgetBuilder::new()
            .with_width(18.0)
            .with_height(18.0)
            .with_margin(Thickness::uniform(2.0))
            .with_visibility(hot_reload),
    )
    .with_opt_texture(load_image(include_bytes!("../resources/flame.png")))
    .build(ctx);

    let engine_version = TextBuilder::new(
        WidgetBuilder::new()
            .with_foreground(ctx.style.property(Style::BRUSH_BRIGHTEST))
            .with_margin(Thickness {
                left: 0.0,
                top: 6.0,
                right: 3.0,
                bottom: 0.0,
            }),
    )
    .with_font_size(13.0.into())
    .with_text(engine_version)
    .build(ctx);

    let info = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Right)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_child(engine_version)
            .with_child(hot_reload),
    )
    .with_orientation(Orientation::Horizontal)
    .build(ctx);

    let item = GridBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .with_child(info)
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
                        .with_foreground(ctx.style.property(Style::BRUSH_BRIGHTEST))
                        .with_margin(Thickness::uniform(2.0))
                        .with_vertical_alignment(VerticalAlignment::Center),
                )
                .with_font_size(13.0.into())
                .with_text(path.to_string_lossy())
                .build(ctx),
            ),
    )
    .add_column(Column::stretch())
    .add_row(Row::auto())
    .add_row(Row::auto())
    .build(ctx);

    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_visibility(visible)
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
    .with_selected_brush(ctx.style.property(Style::BRUSH_DIM_BLUE))
    .build(ctx)
}

fn make_project_items(
    settings: &Settings,
    search_text: &str,
    ctx: &mut BuildContext,
) -> Vec<Handle<UiNode>> {
    settings
        .projects
        .iter()
        .map(|project| {
            let visible = search_text.is_empty()
                || project
                    .name
                    .to_lowercase()
                    .contains(&search_text.to_lowercase());

            let engine_version = utils::read_crate_metadata(&project.manifest_path)
                .ok()
                .and_then(|metadata| utils::fyrox_version_string(&metadata))
                .unwrap_or_default();

            make_project_item(
                &project.name,
                &project.manifest_path,
                project.hot_reload,
                visible,
                &engine_version,
                ctx,
            )
        })
        .collect::<Vec<_>>()
}

impl ProjectManager {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let settings = Settings::load();

        let (sender, receiver) = std::sync::mpsc::channel();
        Log::add_listener(sender);

        let log = LogPanel::new(ctx, receiver, None, false);

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

        let create_tooltip = "Opens a new dialog window that creates a new project with specified \
        settings.";
        let import_tooltip = "Allows you to import an existing project in the project manager.";

        let create = make_button("+ Create", 100.0, 25.0, 0, 0, 0, Some(create_tooltip), ctx);
        let import = make_button("Import", 100.0, 25.0, 1, 0, 1, Some(import_tooltip), ctx);
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_column(2)
                .with_tab_index(Some(2))
                .with_margin(Thickness::uniform(1.0))
                .with_height(25.0),
        )
        .build(ctx);

        let message_count;
        let open_log = ButtonBuilder::new(WidgetBuilder::new().on_column(3).with_visibility(false))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_vertical_alignment(VerticalAlignment::Center)
                        .with_child(
                            ImageBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_width(18.0)
                                    .with_height(18.0)
                                    .on_column(0),
                            )
                            .with_opt_texture(load_image(include_bytes!(
                                "../resources/caution.png"
                            )))
                            .build(ctx),
                        )
                        .with_child({
                            message_count = TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_vertical_alignment(VerticalAlignment::Center)
                                    .with_foreground(Brush::Solid(Color::GOLD).into()),
                            )
                            .with_text("0")
                            .build(ctx);
                            message_count
                        }),
                )
                .add_column(Column::auto())
                .add_column(Column::auto())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build(ctx);

        let toolbar = GridBuilder::new(
            WidgetBuilder::new()
                .with_enabled(is_ready)
                .on_row(1)
                .with_child(create)
                .with_child(import)
                .with_child(search_bar)
                .with_child(open_log),
        )
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::auto())
        .build(ctx);

        let edit_tooltip = "Build the editor and run it.";
        let run_tooltip = "Build the game and run it.";
        let delete_tooltip = "Delete the entire project with all its assets. \
        WARNING: This is irreversible operation and it permanently deletes your project!";
        let upgrade_tooltip = "Allows you to change the engine version in a few clicks.";

        let edit = make_button("Edit", 130.0, 25.0, 3, 0, 0, Some(edit_tooltip), ctx);
        let run = make_button("Run", 130.0, 25.0, 4, 0, 0, Some(run_tooltip), ctx);
        let delete = make_button("Delete", 130.0, 25.0, 5, 0, 0, Some(delete_tooltip), ctx);
        let upgrade = make_button("Upgrade", 130.0, 25.0, 6, 0, 0, Some(upgrade_tooltip), ctx);
        let hot_reload = CheckBoxBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_tooltip(make_simple_tooltip(
                    ctx,
                    "Run the project with code hot reloading support. \
            Significantly reduces iteration times, but might result in subtle bugs due to \
            experimental and unsafe nature of code hot reloading.",
                )),
        )
        .with_content(
            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left(2.0)))
                .with_font_size(16.0f32.into())
                .with_text("Hot Reloading")
                .build(ctx),
        )
        .build(ctx);

        let project_controls = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_enabled(false)
                .on_column(1)
                .with_child(hot_reload)
                .with_child(edit)
                .with_child(run)
                .with_child(delete)
                .with_child(upgrade),
        )
        .build(ctx);

        let projects = ListViewBuilder::new(
            WidgetBuilder::new()
                .with_enabled(is_ready)
                .with_tab_index(Some(6))
                .with_margin(Thickness::uniform(1.0))
                .on_column(0),
        )
        .with_items(make_project_items(&settings, "", ctx))
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
            import_project_dialog: Default::default(),
            mode: Mode::Normal,
            search_text: Default::default(),
            log,
            open_log,
            message_count,
            deletion_confirmation_dialog: Default::default(),
            upgrade,
            upgrade_tool: None,
        }
    }

    fn refresh(&mut self, ui: &mut UserInterface) {
        let items = make_project_items(&self.settings, &self.search_text, &mut ui.build_ctx());
        ui.send_message(ListViewMessage::items(
            self.projects,
            MessageDirection::ToWidget,
            items,
        ))
    }

    fn handle_modes(&mut self, ui: &mut UserInterface) {
        let Mode::Build {
            ref mut process,
            ref mut queue,
            ref current_dir,
        } = self.mode
        else {
            return;
        };

        let build_window = some_or_return!(self.build_window.as_mut());

        if process.is_none() {
            if let Some(build_command) = queue.pop_front() {
                Log::info(format!("Trying to run build command: {build_command}"));

                let mut command = build_command.make_command();

                command.stderr(Stdio::piped()).current_dir(current_dir);

                match command.spawn() {
                    Ok(mut new_process) => {
                        build_window.listen(new_process.stderr.take().unwrap(), ui);

                        *process = Some(new_process);
                    }
                    Err(e) => Log::err(format!("Failed to enter build mode: {e:?}")),
                }
            } else {
                Log::warn("Empty build command queue!");
                self.mode = Mode::Normal;
                return;
            }
        }

        if let Some(process_ref) = process {
            match process_ref.try_wait() {
                Ok(status) => {
                    if let Some(status) = status {
                        // https://doc.rust-lang.org/cargo/commands/cargo-build.html#exit-status
                        let err_code = 101;
                        let code = status.code().unwrap_or(err_code);
                        if code == err_code {
                            Log::err("Failed to build the game!");
                            self.mode = Mode::Normal;
                        } else if queue.is_empty() {
                            build_window.reset(ui);
                            build_window.close(ui);
                        } else {
                            build_window.reset(ui);
                            // Continue on next command.
                            *process = None;
                        }
                    }
                }
                Err(err) => Log::err(format!("Failed to wait for game process: {err:?}")),
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface) {
        self.handle_modes(ui);
        if self.log.update(65536, ui) {
            ui.send_message(TextMessage::text(
                self.message_count,
                MessageDirection::ToWidget,
                self.log.message_count.to_string(),
            ));
            ui.send_message(WidgetMessage::visibility(
                self.open_log,
                MessageDirection::ToWidget,
                true,
            ));
        }

        if let Some(build_window) = self.build_window.as_mut() {
            build_window.update(ui);
        }
    }

    fn try_import(&mut self, path: &Path, ui: &mut UserInterface) {
        let manifest_path = utils::folder_to_manifest_path(path);

        let metadata = match utils::read_crate_metadata(&manifest_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                Log::err(format!(
                    "Failed to read manifest at {}: {}",
                    manifest_path.display(),
                    err
                ));
                return;
            }
        };

        if !utils::has_fyrox_in_deps(&metadata) {
            Log::err(format!("{manifest_path:?} is not a Fyrox project."));
            return;
        }

        if let Some(game_package) = metadata
            .workspace_packages()
            .iter()
            .find(|package| package.id.repr.contains("game#"))
        {
            self.settings.projects.push(Project {
                manifest_path,
                name: game_package.name.clone(),
                hot_reload: false,
            });
            self.refresh(ui);
        } else {
            Log::err(format!(
                "{manifest_path:?} does not contain a game package!"
            ));
        }
    }

    fn on_button_click(&mut self, button: Handle<UiNode>, ui: &mut UserInterface) {
        if button == self.create {
            self.project_wizard = Some(ProjectWizard::new(&mut ui.build_ctx()));
        } else if button == self.import {
            let ctx = &mut ui.build_ctx();
            self.import_project_dialog = FileSelectorBuilder::new(
                WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                    .open(false)
                    .with_remove_on_close(true),
            )
            .with_filter(Filter::new(|path| path.is_dir()))
            .with_mode(FileBrowserMode::Open)
            .build(ctx);
            ui.send_message(WindowMessage::open_modal(
                self.import_project_dialog,
                MessageDirection::ToWidget,
                true,
                true,
            ));
        } else if button == self.download {
            let _ = open::that("https://rustup.rs/");
        } else if button == self.open_log {
            self.log.open(ui);
        } else if button == self.upgrade {
            if let Some(index) = self.selection {
                if let Some(project) = self.settings.projects.get(index) {
                    let ctx = &mut ui.build_ctx();
                    self.upgrade_tool = Some(UpgradeTool::new(project, ctx));
                }
            }
        }

        if let Some(index) = self.selection {
            if let Some(project) = self.settings.projects.get(index) {
                if button == self.edit {
                    let profile = if project.hot_reload {
                        BuildProfile::debug_editor_hot_reloading()
                    } else {
                        BuildProfile::debug_editor()
                    };
                    self.run_build_profile("editor", &profile, ui);
                } else if button == self.run {
                    let profile = if project.hot_reload {
                        BuildProfile::debug_hot_reloading()
                    } else {
                        BuildProfile::debug()
                    };
                    self.run_build_profile("game", &profile, ui);
                } else if button == self.delete {
                    let ctx = &mut ui.build_ctx();
                    self.deletion_confirmation_dialog = MessageBoxBuilder::new(
                        WindowBuilder::new(WidgetBuilder::new())
                            .with_remove_on_close(true)
                            .with_title(WindowTitle::text("Delete Project"))
                            .open(false),
                    )
                    .with_text(&format!("Do you really want to delete {}?", project.name))
                    .with_buttons(MessageBoxButtons::YesNo)
                    .build(ctx);
                    ui.send_message(WindowMessage::open_modal(
                        self.deletion_confirmation_dialog,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));
                }
            }
        }
    }

    fn run_build_profile(
        &mut self,
        name: &str,
        build_profile: &BuildProfile,
        ui: &mut UserInterface,
    ) {
        let index = some_or_return!(self.selection);
        let project = some_or_return!(self.settings.projects.get(index));

        self.mode = Mode::Build {
            queue: build_profile.build_and_run_queue(),
            process: None,
            current_dir: project.manifest_path.parent().unwrap().into(),
        };
        self.build_window = Some(BuildWindow::new(name, &mut ui.build_ctx()));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(project_wizard) = self.project_wizard.as_mut() {
            if project_wizard.handle_ui_message(message, ui, &mut self.settings) {
                self.refresh(ui);
            }
        }

        self.log.handle_ui_message(message, ui);

        if let Some(build_window) = self.build_window.as_mut() {
            build_window.handle_ui_message(message, ui);
        }

        if let Some(upgrade_tool) = self.upgrade_tool.take() {
            if let Some(index) = self.selection {
                if let Some(project) = self.settings.projects.get(index) {
                    let mut need_refresh = false;
                    self.upgrade_tool =
                        upgrade_tool.handle_ui_message(message, ui, project, &mut need_refresh);
                    if need_refresh {
                        self.refresh(ui);
                    }
                }
            }
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

                if let Some(project) = self.selection.and_then(|i| self.settings.projects.get(i)) {
                    ui.send_message(CheckBoxMessage::checked(
                        self.hot_reload,
                        MessageDirection::ToWidget,
                        Some(project.hot_reload),
                    ));
                }
            }
        } else if let Some(SearchBarMessage::Text(filter)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                self.search_text = filter.clone();
                self.refresh(ui);
            }
        } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.hot_reload
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(selection) = self.selection {
                    if let Some(project) = self.settings.projects.get_mut(selection) {
                        if project.hot_reload != *value {
                            project.hot_reload = *value;
                            self.refresh(ui);
                        }
                    }
                }
            }
        } else if let Some(FileSelectorMessage::Commit(path)) = message.data() {
            if message.destination() == self.import_project_dialog {
                self.try_import(path, ui);
            }
        } else if let Some(MessageBoxMessage::Close(MessageBoxResult::Yes)) = message.data() {
            if message.destination() == self.deletion_confirmation_dialog {
                if let Some(index) = self.selection {
                    if let Some(project) = self.settings.projects.get(index) {
                        if let Some(dir) = project.manifest_path.parent() {
                            let _ = std::fs::remove_dir_all(dir);
                        }
                        self.settings.projects.remove(index);
                        self.refresh(ui);
                    }
                }
            }
        }
    }
}
