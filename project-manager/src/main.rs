//! Project manager is used to create, import, rename, delete, run and edit projects built with Fyrox.

use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::Vector2,
        instant::Instant,
        log::{Log, MessageKind},
        pool::Handle,
        task::TaskPool,
    },
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        constructor::WidgetConstructorContainer,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        screen::ScreenBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        BuildContext, Orientation, UiNode,
    },
    utils::translate_event,
    window::WindowAttributes,
};
use std::sync::Arc;

fn main() {
    let mut window_attributes = WindowAttributes::default();
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

    let _project_manager = ProjectManager::new(&mut engine.user_interfaces.first_mut().build_ctx());

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

struct ProjectManager {}

fn make_button(text: &str, ctx: &mut BuildContext) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_height(25.0)
            .with_min_size(Vector2::new(120.0, 25.0)),
    )
    .with_text(text)
    .build(ctx)
}

impl ProjectManager {
    fn new(ctx: &mut BuildContext) -> Self {
        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(make_button("+ Create", ctx))
                .with_child(make_button("Import", ctx)),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let sidebar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_child(make_button("Edit", ctx))
                .with_child(make_button("Run", ctx))
                .with_child(make_button("Delete", ctx)),
        )
        .build(ctx);

        let projects = ListViewBuilder::new(WidgetBuilder::new().on_column(0)).build(ctx);

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

        ScreenBuilder::new(WidgetBuilder::new().with_child(
            BorderBuilder::new(WidgetBuilder::new().with_child(main_content)).build(ctx),
        ))
        .build(ctx);

        Self {}
    }
}
