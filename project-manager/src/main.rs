//! Project manager is used to create, import, rename, delete, run and edit projects built with Fyrox.

mod build;
mod project;
mod settings;
mod utils;

use crate::{
    build::BuildWindow,
    project::ProjectWizard,
    settings::Settings,
    utils::{is_production_ready, load_image, make_button},
};
use fyrox::{
    asset::manager::ResourceManager,
    core::{
        color::Color,
        instant::Instant,
        log::{Log, MessageKind},
        pool::Handle,
        task::TaskPool,
    },
    dpi::PhysicalSize,
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        constructor::WidgetConstructorContainer,
        decorator::DecoratorBuilder,
        font::Font,
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
    utils::translate_event,
    window::WindowAttributes,
};
use std::{path::Path, process::Stdio, sync::Arc};

fn main() {
    let mut window_attributes = WindowAttributes::default();
    window_attributes.inner_size = Some(PhysicalSize::new(520, 562).into());
    window_attributes.resizable = true;
    window_attributes.title = "Fyrox Project Manager".to_string();

    let serialization_context = Arc::new(SerializationContext::new());
    let task_pool = Arc::new(TaskPool::new());
    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params: GraphicsContextParams {
            window_attributes,
            vsync: true,
            msaa_sample_count: None,
        },
        resource_manager: ResourceManager::new(task_pool.clone()),
        serialization_context,
        task_pool,
        widget_constructors: Arc::new(WidgetConstructorContainer::new()),
    })
    .unwrap();

    let primary_ui = engine.user_interfaces.first_mut();
    primary_ui.default_font = engine
        .resource_manager
        .request::<Font>("resources/arial.ttf");
    let mut project_manager = ProjectManager::new(&mut primary_ui.build_ctx());

    let event_loop = EventLoop::new().unwrap();

    let mut previous = Instant::now();
    let fixed_time_step = 1.0 / 60.0;
    let mut lag = 0.0;

    event_loop
        .run(move |event, window_target| {
            window_target.set_control_flow(ControlFlow::Wait);

            match event {
                Event::Resumed => {
                    engine
                        .initialize_graphics_context(window_target)
                        .expect("Unable to initialize graphics context!");
                }
                Event::Suspended => {
                    engine
                        .destroy_graphics_context()
                        .expect("Unable to destroy graphics context!");
                }
                Event::AboutToWait => {
                    let elapsed = previous.elapsed();
                    previous = Instant::now();
                    lag += elapsed.as_secs_f32();

                    while lag >= fixed_time_step {
                        engine.update(fixed_time_step, window_target, &mut lag, Default::default());

                        project_manager.update(engine.user_interfaces.first_mut());

                        lag -= fixed_time_step;
                    }

                    let ui = engine.user_interfaces.first_mut();
                    while let Some(message) = ui.poll_message() {
                        project_manager.handle_ui_message(&message, ui);
                    }

                    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                        ctx.window.request_redraw();
                    }
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => window_target.exit(),
                        WindowEvent::Resized(size) => {
                            if let Err(e) = engine.set_frame_size(size.into()) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!("Unable to set frame size: {:?}", e),
                                );
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            engine.render().unwrap();
                        }
                        _ => (),
                    }

                    if let Some(os_event) = translate_event(&event) {
                        for ui in engine.user_interfaces.iter_mut() {
                            ui.process_os_event(&os_event);
                        }
                    }
                }
                Event::LoopExiting => {
                    project_manager.settings.save();
                }
                _ => (),
            }
        })
        .unwrap();
}

struct ProjectManager {
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
    settings: Settings,
    project_wizard: Option<ProjectWizard>,
    build_window: Option<BuildWindow>,
}

fn make_project_item(name: &str, path: &Path, ctx: &mut BuildContext) -> Handle<UiNode> {
    DecoratorBuilder::new(
        BorderBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(4.0))
                                        .with_width(40.0)
                                        .with_height(40.0)
                                        .on_column(0),
                                )
                                .with_opt_texture(load_image(include_bytes!(
                                    "../resources/icon.png"
                                )))
                                .build(ctx),
                            )
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_child(
                                            TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(0)
                                                    .with_margin(Thickness::uniform(2.0))
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .with_font_size(18.0)
                                            .with_text(name)
                                            .build(ctx),
                                        )
                                        .with_child(
                                            TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .on_row(1)
                                                    .with_margin(Thickness::uniform(2.0))
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    ),
                                            )
                                            .with_font_size(13.0)
                                            .with_text(path.to_string_lossy())
                                            .build(ctx),
                                        ),
                                )
                                .add_column(Column::auto())
                                .add_row(Row::auto())
                                .add_row(Row::auto())
                                .build(ctx),
                            ),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_row(Row::auto())
                    .build(ctx),
                ),
        )
        .with_corner_radius(4.0),
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
    fn new(ctx: &mut BuildContext) -> Self {
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
                            .with_foreground(Brush::Solid(Color::RED)),
                    )
                    .with_text(
                        "Rust is not installed, please click the button at the right \
                        and follow build instructions for your platform.",
                    )
                    .with_font_size(18.0)
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
        .add_row(Row::auto())
        .build(ctx);

        let main_content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(warning)
                .with_child(toolbar)
                .with_child(inner_content),
        )
        .add_column(Column::auto())
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

    fn update(&mut self, ui: &mut UserInterface) {
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
                        Err(e) => Log::err(format!("Failed to start the editor: {:?}", e)),
                    }
                } else if button == self.run {
                    let mut new_process = std::process::Command::new("cargo");
                    new_process
                        .current_dir(dbg!(project.manifest_path.parent().unwrap()))
                        .stderr(Stdio::piped())
                        .args(["run", "--package", "executor"]);

                    match new_process.spawn() {
                        Ok(mut new_process) => {
                            let mut build_window = BuildWindow::new(&mut ui.build_ctx());

                            build_window.listen(new_process.stderr.take().unwrap(), ui);

                            self.build_window = Some(build_window);
                        }
                        Err(e) => Log::err(format!("Failed to start the game: {:?}", e)),
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

    fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
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
