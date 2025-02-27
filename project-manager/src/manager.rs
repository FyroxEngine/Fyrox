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
    project::ProjectWizard,
    settings::{Project, Settings, SettingsWindow, MANIFEST_PATH_VAR},
    upgrade::UpgradeTool,
    utils::{self, is_production_ready},
};
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
        message::{KeyCode, MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
        navigation::NavigationLayerBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        style::{resource::StyleResourceExt, Style},
        text::{TextBuilder, TextMessage},
        utils::{
            load_image, make_image_button_with_tooltip, make_simple_tooltip,
            make_text_and_image_button_with_tooltip,
        },
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
};
use fyrox_build_tools::{build::BuildWindow, BuildProfile, CommandDescriptor};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Stdio,
};

pub enum Mode {
    Normal,
    CommandExecution {
        queue: VecDeque<CommandDescriptor>,
        process: Option<std::process::Child>,
        current_dir: PathBuf,
    },
}

impl Mode {
    pub fn is_build(&self) -> bool {
        matches!(self, Mode::CommandExecution { .. })
    }
}

pub struct UpdateLoopState(u32);

impl Default for UpdateLoopState {
    fn default() -> Self {
        // Run at least a second from the start to ensure that all OS-specific stuff was done.
        Self(60)
    }
}

impl UpdateLoopState {
    pub fn request_update_in_next_frame(&mut self) {
        if !self.is_warming_up() {
            self.0 = 2;
        }
    }

    pub fn request_update_in_current_frame(&mut self) {
        if !self.is_warming_up() {
            self.0 = 1;
        }
    }

    pub fn is_warming_up(&self) -> bool {
        self.0 > 2
    }

    pub fn decrease_counter(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    pub fn is_suspended(&self) -> bool {
        self.0 == 0
    }
}

pub struct ProjectManager {
    pub root_grid: Handle<UiNode>,
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
    pub mode: Mode,
    search_text: String,
    log: LogPanel,
    open_log: Handle<UiNode>,
    message_count: Handle<UiNode>,
    deletion_confirmation_dialog: Handle<UiNode>,
    upgrade: Handle<UiNode>,
    locate: Handle<UiNode>,
    open_settings: Handle<UiNode>,
    open_help: Handle<UiNode>,
    open_ide: Handle<UiNode>,
    upgrade_tool: Option<UpgradeTool>,
    settings_window: Option<SettingsWindow>,
    no_projects_warning: Handle<UiNode>,
    exclude_project: Handle<UiNode>,
    clean_project: Handle<UiNode>,
    pub focused: bool,
    pub update_loop_state: UpdateLoopState,
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

    let project_size = if let Some(project_dir) = Path::new(path).parent() {
        let size = utils::calculate_directory_size(project_dir);
        utils::format_size(size)
    } else {
        String::from("N/A")
    };

    let size_text = TextBuilder::new(
        WidgetBuilder::new()
            .with_foreground(ctx.style.property(Style::BRUSH_BRIGHTEST))
            .with_margin(Thickness {
                left: 0.0,
                top: 6.0,
                right: 8.0,
                bottom: 0.0,
            }),
    )
    .with_font_size(13.0.into())
    .with_text(format!("Size: {}", project_size))
    .build(ctx);

    let info = StackPanelBuilder::new(
        WidgetBuilder::new()
            .with_horizontal_alignment(HorizontalAlignment::Right)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_child(size_text)
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
        settings.\nHotkey: Ctrl+C";
        let import_tooltip = "Allows you to import an existing project in the project manager.\
        \nHotkey: Ctrl+I";

        let font_size = 16.0;
        let create = make_text_and_image_button_with_tooltip(
            ctx,
            "Create",
            20.0,
            20.0,
            load_image(include_bytes!("../resources/plus.png")),
            create_tooltip,
            0,
            0,
            Some(0),
            Color::LIME_GREEN,
            font_size,
        );
        let import = make_text_and_image_button_with_tooltip(
            ctx,
            "Import",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/open-folder.png")),
            import_tooltip,
            0,
            1,
            Some(1),
            Color::GOLD,
            font_size,
        );
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_column(2)
                .with_tab_index(Some(2))
                .with_margin(Thickness::uniform(1.0))
                .with_height(25.0),
        )
        .build(ctx);
        let open_settings = make_image_button_with_tooltip(
            ctx,
            18.0,
            18.0,
            load_image(include_bytes!("../resources/gear.png")),
            "Settings\nHotkey: Ctrl+S",
            Some(7),
        );
        ctx[open_settings].set_column(3);

        let open_help = make_image_button_with_tooltip(
            ctx,
            18.0,
            18.0,
            load_image(include_bytes!("../resources/question.png")),
            "Help\nHotkey: F1",
            Some(8),
        );
        ctx[open_help].set_column(4);

        let message_count;
        let open_log = ButtonBuilder::new(WidgetBuilder::new().on_column(4).with_visibility(false))
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
                .with_child(open_settings)
                .with_child(open_help)
                .with_child(open_log),
        )
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_row(Row::auto())
        .build(ctx);

        let edit_tooltip = "Build the editor and run it.\nHotkey: Shift+Enter";
        let run_tooltip = "Build the game and run it.\nHotkey: Enter";
        let delete_tooltip = "Delete the entire project with all its assets. \
        WARNING: This is irreversible operation and it permanently deletes your project!\
        \nHotkey: Delete";
        let upgrade_tooltip = "Allows you to change the engine version in a few clicks.\
        \nHotkey: Ctrl+U";
        let hot_reload_tooltip = "Run the project with code hot reloading support. \
            Significantly reduces iteration times, but might result in subtle bugs due to \
            experimental and unsafe nature of code hot reloading.\
            \nHotkey: Ctrl+H";
        let locate_tooltip = "Opens project folder in the default OS file manager.\
        \nHotkey: Ctrl+L";
        let open_ide_tooltip = "Opens project folder in the currently selected IDE \
        (can be changed in settings).\nHotkey: Ctrl+O";
        let exclude_project_tooltip = "Removes the project from the project manager, \
        but does NOT delete it.\nHotkey: Ctrl+E";
        let clean_project_tooltip = "Removes project's build artifacts.\nHotkey: Ctrl+N";

        let edit = make_text_and_image_button_with_tooltip(
            ctx,
            "Edit",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/pencil.png")),
            edit_tooltip,
            0,
            0,
            Some(5),
            Color::GOLD,
            font_size,
        );
        let run = make_text_and_image_button_with_tooltip(
            ctx,
            "Run",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/play.png")),
            run_tooltip,
            0,
            0,
            Some(6),
            Color::opaque(73, 156, 84),
            font_size,
        );
        let delete = make_text_and_image_button_with_tooltip(
            ctx,
            "Delete",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/delete.png")),
            delete_tooltip,
            0,
            0,
            Some(7),
            Color::ORANGE_RED,
            font_size,
        );
        let upgrade = make_text_and_image_button_with_tooltip(
            ctx,
            "Upgrade",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/up.png")),
            upgrade_tooltip,
            0,
            0,
            Some(8),
            Color::MEDIUM_PURPLE,
            font_size,
        );
        let locate = make_text_and_image_button_with_tooltip(
            ctx,
            "Locate",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/location.png")),
            locate_tooltip,
            0,
            0,
            Some(9),
            Color::DODGER_BLUE,
            font_size,
        );
        let open_ide = make_text_and_image_button_with_tooltip(
            ctx,
            "Open IDE",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/ide.png")),
            open_ide_tooltip,
            0,
            0,
            Some(9),
            Color::LIGHT_GRAY,
            font_size,
        );
        let exclude_project = make_text_and_image_button_with_tooltip(
            ctx,
            "Exclude",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/cross.png")),
            exclude_project_tooltip,
            0,
            0,
            Some(10),
            Color::ORANGE,
            font_size,
        );
        let clean_project = make_text_and_image_button_with_tooltip(
            ctx,
            "Clean",
            22.0,
            22.0,
            load_image(include_bytes!("../resources/clean.png")),
            clean_project_tooltip,
            0,
            0,
            Some(11),
            Color::LIGHT_STEEL_BLUE,
            font_size,
        );
        let hot_reload = CheckBoxBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(4))
                .with_margin(Thickness::uniform(1.0))
                .with_tooltip(make_simple_tooltip(ctx, hot_reload_tooltip)),
        )
        .with_content(
            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left(2.0)))
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
                .with_child(open_ide)
                .with_child(upgrade)
                .with_child(clean_project)
                .with_child(locate)
                .with_child(delete)
                .with_child(exclude_project),
        )
        .build(ctx);

        let projects = ListViewBuilder::new(
            WidgetBuilder::new()
                .with_enabled(is_ready)
                .with_tab_index(Some(3))
                .with_margin(Thickness::uniform(1.0)),
        )
        .with_items(make_project_items(&settings, "", ctx))
        .build(ctx);

        let no_projects_warning =
            TextBuilder::new(WidgetBuilder::new().with_visibility(settings.projects.is_empty()))
                .with_text(
                    "At this moment you don't have any existing projects.\n\
                        Click \"+Create\" button to create a new project or \"Import\" an \
                        existing one.",
                )
                .with_font_size(16.0f32.into())
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx);

        let border = BorderBuilder::new(
            WidgetBuilder::new()
                .on_column(0)
                .with_child(projects)
                .with_child(no_projects_warning),
        )
        .build(ctx);

        let inner_content = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(border)
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

        let root_grid = GridBuilder::new(WidgetBuilder::new().with_child(
            BorderBuilder::new(WidgetBuilder::new().with_child(navigation_layer)).build(ctx),
        ))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        ctx.send_message(WidgetMessage::focus(
            navigation_layer,
            MessageDirection::ToWidget,
        ));

        Self {
            root_grid,
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
            locate,
            open_settings,
            open_help,
            open_ide,
            upgrade_tool: None,
            settings_window: None,
            no_projects_warning,
            exclude_project,
            clean_project,
            focused: true,
            update_loop_state: Default::default(),
        }
    }

    pub fn is_active(&self, ui: &UserInterface) -> bool {
        !self.update_loop_state.is_suspended() && self.focused
            || ui.captured_node().is_some()
            || self.mode.is_build()
    }

    fn refresh(&mut self, ui: &mut UserInterface) {
        let items = make_project_items(&self.settings, &self.search_text, &mut ui.build_ctx());
        ui.send_message(WidgetMessage::visibility(
            self.no_projects_warning,
            MessageDirection::ToWidget,
            items.is_empty(),
        ));
        ui.send_message(ListViewMessage::items(
            self.projects,
            MessageDirection::ToWidget,
            items,
        ));
    }

    fn handle_modes(&mut self, ui: &mut UserInterface) {
        let Mode::CommandExecution {
            ref mut process,
            ref mut queue,
            ref current_dir,
        } = self.mode
        else {
            return;
        };

        if process.is_none() {
            if let Some(build_command) = queue.pop_front() {
                Log::info(format!("Trying to run build command: {build_command}"));

                let mut command = build_command.make_command();

                command.stderr(Stdio::piped()).current_dir(current_dir);

                match command.spawn() {
                    Ok(mut new_process) => {
                        if let Some(build_window) = self.build_window.as_mut() {
                            build_window.listen(new_process.stderr.take().unwrap(), ui);
                        }

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
                            if let Some(build_window) = self.build_window.take() {
                                build_window.destroy(ui);
                            }
                            self.mode = Mode::Normal;
                        } else {
                            if let Some(build_window) = self.build_window.as_mut() {
                                build_window.reset(ui);
                            }
                            // Continue on next command.
                            *process = None;
                        }
                    }
                }
                Err(err) => Log::err(format!("Failed to wait for game process: {err:?}")),
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface, dt: f32) {
        self.handle_modes(ui);

        if let Some(active_tooltip) = ui.active_tooltip() {
            if !active_tooltip.shown {
                // Keep the manager running until the current tooltip is not shown.
                self.update_loop_state.request_update_in_next_frame();
            }
        }

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
            build_window.update(ui, dt);
        }

        self.settings.try_save();
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

    fn on_run_clicked(&mut self, ui: &mut UserInterface) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));

        let profile = if project.hot_reload {
            BuildProfile::debug_hot_reloading()
        } else {
            BuildProfile::debug()
        };
        self.run_build_profile("game", &profile, ui);
    }

    fn on_edit_clicked(&mut self, ui: &mut UserInterface) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));

        let profile = if project.hot_reload {
            BuildProfile::debug_editor_hot_reloading()
        } else {
            BuildProfile::debug_editor()
        };
        self.run_build_profile("editor", &profile, ui);
    }

    fn on_delete_clicked(&mut self, ui: &mut UserInterface) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));

        let ctx = &mut ui.build_ctx();
        self.deletion_confirmation_dialog = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new())
                .with_remove_on_close(true)
                .with_title(WindowTitle::text("Delete Project"))
                .open(false),
        )
        .with_text(&format!(
            "Do you really want to delete {} project?\n\
        WARNING: This is irreversible operation and it permanently deletes the project!",
            project.name
        ))
        .with_buttons(MessageBoxButtons::YesNo)
        .build(ctx);
        ui.send_message(WindowMessage::open_modal(
            self.deletion_confirmation_dialog,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    fn on_open_help_clicked(&mut self) {
        Log::verify(open::that_detached(
            "https://fyrox-book.github.io/beginning/project_manager.html",
        ));
    }

    fn on_upgrade_clicked(&mut self, ui: &mut UserInterface) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));
        let ctx = &mut ui.build_ctx();
        self.upgrade_tool = Some(UpgradeTool::new(project, ctx));
    }

    fn on_import_clicked(&mut self, ui: &mut UserInterface) {
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
        ui.send_message(FileSelectorMessage::focus_current_path(
            self.import_project_dialog,
            MessageDirection::ToWidget,
        ));
    }

    fn on_create_clicked(&mut self, ui: &mut UserInterface) {
        self.project_wizard = Some(ProjectWizard::new(&mut ui.build_ctx()));
    }

    fn on_exclude_project_clicked(&mut self, ui: &mut UserInterface) {
        let project_index = some_or_return!(self.selection);
        if project_index < self.settings.projects.len() {
            self.settings.projects.remove(project_index);
        }
        self.refresh(ui);
    }

    fn on_clean_clicked(&mut self, ui: &mut UserInterface) {
        self.run_selected_project_command(
            "clean",
            vec![CommandDescriptor {
                command: "cargo".to_string(),
                args: vec!["clean".to_string()],
                environment_variables: vec![],
            }]
            .into(),
            ui,
        )
    }

    fn on_button_click(&mut self, button: Handle<UiNode>, ui: &mut UserInterface) {
        if button == self.create {
            self.on_create_clicked(ui);
        } else if button == self.import {
            self.on_import_clicked(ui);
        } else if button == self.download {
            let _ = open::that("https://rustup.rs/");
        } else if button == self.open_log {
            self.log.open(ui);
        } else if button == self.upgrade {
            self.on_upgrade_clicked(ui);
        } else if button == self.edit {
            self.on_edit_clicked(ui);
        } else if button == self.run {
            self.on_run_clicked(ui);
        } else if button == self.delete {
            self.on_delete_clicked(ui);
        } else if button == self.locate {
            self.on_locate_click();
        } else if button == self.open_ide {
            self.on_open_ide_click(ui);
        } else if button == self.open_settings {
            self.on_open_settings_click(ui);
        } else if button == self.exclude_project {
            self.on_exclude_project_clicked(ui);
        } else if button == self.clean_project {
            self.on_clean_clicked(ui);
        } else if button == self.open_help {
            self.on_open_help_clicked();
        }
    }

    fn on_locate_click(&mut self) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));
        let folder = some_or_return!(project.manifest_path.parent());
        Log::verify(open::that_detached(folder));
    }

    fn on_open_ide_click(&mut self, ui: &mut UserInterface) {
        let project = some_or_return!(self.selection.and_then(|i| self.settings.projects.get(i)));
        let mut open_ide_command = self.settings.open_ide_command.clone();
        if let Some(manifest_path_arg) = open_ide_command
            .args
            .iter_mut()
            .find(|cmd| cmd.as_str() == MANIFEST_PATH_VAR)
        {
            *manifest_path_arg = project.manifest_path.to_string_lossy().to_string();
        } else {
            Log::warn(format!("{} variable is not specified!", MANIFEST_PATH_VAR));
        }
        let mut command = open_ide_command.make_command();
        if let Err(err) = command.spawn() {
            Log::err(format!(
                "Unable to open the IDE using {} command. Reason: {:?}",
                open_ide_command, err
            ));

            self.on_open_settings_click(ui);
        }
    }

    fn on_open_settings_click(&mut self, ui: &mut UserInterface) {
        let ctx = &mut ui.build_ctx();
        self.settings_window = Some(SettingsWindow::new(&self.settings, ctx));
    }

    fn run_selected_project_command(
        &mut self,
        action_name: &str,
        queue: VecDeque<CommandDescriptor>,
        ui: &mut UserInterface,
    ) {
        let index = some_or_return!(self.selection);
        let project = some_or_return!(self.settings.projects.get(index));
        self.mode = Mode::CommandExecution {
            queue,
            process: None,
            current_dir: project.manifest_path.parent().unwrap().into(),
        };
        self.build_window = Some(BuildWindow::new(action_name, &mut ui.build_ctx()));
    }

    fn run_build_profile(
        &mut self,
        name: &str,
        build_profile: &BuildProfile,
        ui: &mut UserInterface,
    ) {
        let mut build_profile = build_profile.clone();
        // Force run `cargo update` before running the project to prevent various issues with
        // dependency versions incompatibility.
        build_profile.build_commands.insert(
            0,
            CommandDescriptor {
                command: "cargo".to_string(),
                args: vec!["update".to_string()],
                environment_variables: vec![],
            },
        );
        self.run_selected_project_command(name, build_profile.build_and_run_queue(), ui);
    }

    fn on_hot_reload_changed(&mut self, value: bool, ui: &mut UserInterface) {
        let project = some_or_return!(self
            .selection
            .and_then(|i| self.settings.projects.get_mut(i)));
        if project.hot_reload != value {
            project.hot_reload = value;
            self.refresh(ui);
        }
    }

    fn on_hot_key(&mut self, key_code: KeyCode, ui: &mut UserInterface) {
        let modifiers = ui.keyboard_modifiers();
        match key_code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if modifiers.shift {
                    self.on_edit_clicked(ui);
                } else {
                    self.on_run_clicked(ui);
                }
            }
            KeyCode::Delete => self.on_delete_clicked(ui),
            KeyCode::KeyU if modifiers.control => self.on_upgrade_clicked(ui),
            KeyCode::KeyI if modifiers.control => self.on_import_clicked(ui),
            KeyCode::KeyC if modifiers.control => self.on_create_clicked(ui),
            KeyCode::KeyH if modifiers.control => self.on_hot_reload_changed(true, ui),
            KeyCode::KeyL if modifiers.control => self.on_locate_click(),
            KeyCode::KeyO if modifiers.control => self.on_open_ide_click(ui),
            KeyCode::KeyS if modifiers.control => self.on_open_settings_click(ui),
            KeyCode::KeyE if modifiers.control => self.on_exclude_project_clicked(ui),
            KeyCode::KeyN if modifiers.control => self.on_clean_clicked(ui),
            KeyCode::F1 => self.on_open_help_clicked(),
            _ => (),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(project_wizard) = self.project_wizard.as_mut() {
            if project_wizard.handle_ui_message(message, ui, &mut self.settings) {
                self.refresh(ui);
            }
        }

        self.log.handle_ui_message(message, ui);

        if let Some(build_window) = self.build_window.take() {
            self.build_window = build_window.handle_ui_message(message, ui, || {});
        }

        if let Some(settings_window) = self.settings_window.take() {
            self.settings_window =
                settings_window.handle_ui_message(&mut self.settings, message, ui);
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
                self.on_hot_reload_changed(*value, ui);
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
        } else if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            if !message.handled() {
                self.on_hot_key(*key, ui)
            }
        }
    }
}
