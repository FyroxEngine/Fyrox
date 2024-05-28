//! Project manager is used to create, import, rename, delete, run and edit projects built with Fyrox.

use fyrox::gui::navigation::NavigationLayerBuilder;
use fyrox::{
    asset::manager::ResourceManager,
    core::{
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
        button::{ButtonBuilder, ButtonMessage},
        constructor::WidgetConstructorContainer,
        font::Font,
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        screen::ScreenBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, VerticalAlignment,
    },
    utils::translate_event,
    window::WindowAttributes,
};
use std::sync::Arc;

fn main() {
    let mut window_attributes = WindowAttributes::default();
    window_attributes.inner_size = Some(PhysicalSize::new(1000, 562).into());
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
    let project_manager = ProjectManager::new(&mut primary_ui.build_ctx());

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
                        lag -= fixed_time_step;
                    }

                    while let Some(message) = engine.user_interfaces.first_mut().poll_message() {
                        project_manager.handle_ui_message(&message);
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
}

fn make_button(
    text: &str,
    width: f32,
    height: f32,
    tab_index: usize,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_width(width)
            .with_height(height)
            .with_tab_index(Some(tab_index))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_content(
        TextBuilder::new(WidgetBuilder::new())
            .with_text(text)
            .with_font_size(16.0)
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_horizontal_text_alignment(HorizontalAlignment::Center)
            .build(ctx),
    )
    .build(ctx)
}

impl ProjectManager {
    fn new(ctx: &mut BuildContext) -> Self {
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
                .with_child(create)
                .with_child(import)
                .with_child(search_bar),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let edit = make_button("Edit", 100.0, 25.0, 3, ctx);
        let run = make_button("Run", 100.0, 25.0, 4, ctx);
        let delete = make_button("Delete", 100.0, 25.0, 5, ctx);

        let sidebar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child(edit)
                .with_child(run)
                .with_child(delete),
        )
        .build(ctx);

        let projects = ListViewBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(6))
                .with_margin(Thickness::uniform(1.0))
                .on_column(0),
        )
        .build(ctx);

        let inner_content = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(projects)
                .with_child(sidebar),
        )
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .add_row(Row::auto())
        .build(ctx);

        let main_content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar)
                .with_child(inner_content),
        )
        .add_column(Column::auto())
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
        }
    }

    fn handle_ui_message(&self, message: &UiMessage) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.create {
                // TODO: Create project.
            } else if message.destination() == self.import {
                // TODO: Import project.
            } else if message.destination() == self.edit {
                // TODO: Edit project.
            } else if message.destination() == self.run {
                // TODO: Delete project.
            } else if message.destination() == self.delete {
            }
        } else if let Some(ListViewMessage::SelectionChanged(Some(_index))) = message.data() {
            if message.destination() == self.projects
                && message.direction() == MessageDirection::FromWidget
            {
                // TODO: Change selection.
            }
        } else if let Some(SearchBarMessage::Text(_filter)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                // TODO: Filter projects.
            }
        }
    }
}
