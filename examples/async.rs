//! Example 02. Asynchronous scene loading.
//!
//! Difficulty: Medium.
//!
//! This example shows how to load scene in separate thread and how create standard
//! loading screen which will show progress.

pub mod shared;

use fyrox::{
    animation::Animation,
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3},
        color::Color,
        futures,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    engine::{executor::Executor, resource_manager::ResourceManager},
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::MessageDirection,
        progress_bar::{ProgressBarBuilder, ProgressBarMessage},
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment,
    },
    plugin::{Plugin, PluginConstructor, PluginContext},
    scene::{
        node::{Node, TypeUuidProvider},
        Scene,
    },
};
use std::sync::{Arc, Mutex};

use crate::shared::create_camera;

struct Interface {
    root: Handle<UiNode>,
    debug_text: Handle<UiNode>,
    progress_bar: Handle<UiNode>,
    progress_text: Handle<UiNode>,
}

fn create_ui(ctx: &mut BuildContext, screen_size: Vector2<f32>) -> Interface {
    let debug_text;
    let progress_bar;
    let progress_text;
    let root = GridBuilder::new(
        WidgetBuilder::new()
            .with_width(screen_size.x)
            .with_height(screen_size.y)
            .with_child({
                debug_text =
                    TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0)).build(ctx);
                debug_text
            })
            .with_child({
                progress_bar =
                    ProgressBarBuilder::new(WidgetBuilder::new().on_row(1).on_column(1)).build(ctx);
                progress_bar
            })
            .with_child({
                progress_text = TextBuilder::new(
                    WidgetBuilder::new()
                        .on_column(1)
                        .on_row(0)
                        .with_margin(Thickness::bottom(20.0))
                        .with_vertical_alignment(VerticalAlignment::Bottom),
                )
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .build(ctx);
                progress_text
            }),
    )
    .add_row(Row::stretch())
    .add_row(Row::strict(30.0))
    .add_row(Row::stretch())
    .add_column(Column::stretch())
    .add_column(Column::strict(200.0))
    .add_column(Column::stretch())
    .build(ctx);

    Interface {
        root,
        debug_text,
        progress_bar,
        progress_text,
    }
}

struct SceneLoader {
    scene: Scene,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
}

impl SceneLoader {
    async fn load_with(resource_manager: ResourceManager, context: Arc<Mutex<AsyncLoaderContext>>) {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(200, 200, 200);

        // It is important to lock context for short period of time so other thread can
        // read data from it as soon as possible - not when everything was loaded.
        context
            .lock()
            .unwrap()
            .report_progress(0.0, "Creating camera...");

        // Camera is our eyes in the world - you won't see anything without it.
        create_camera(
            resource_manager.clone(),
            Vector3::new(0.0, 6.0, -12.0),
            &mut scene.graph,
        )
        .await;

        context
            .lock()
            .unwrap()
            .report_progress(0.33, "Loading model...");

        // Load model resource. Is does *not* adds anything to our scene - it just loads a
        // resource then can be used later on to instantiate models from it on scene. Why
        // loading of resource is separated from instantiation? Because there it is too
        // inefficient to load a resource every time you trying to create instance of it -
        // much more efficient is to load it one and then make copies of it. In case of
        // models it is very efficient because single vertex and index buffer can be used
        // for all models instances, so memory footprint on GPU will be lower.
        let model_resource = resource_manager
            .request_model("examples/data/mutant/mutant.FBX")
            .await
            .unwrap();

        // Instantiate model on scene - but only geometry, without any animations.
        // Instantiation is a process of embedding model resource data in desired scene.
        let model_handle = model_resource.instantiate_geometry(&mut scene);

        // Now we have whole sub-graph instantiated, we can start modifying model instance.
        scene.graph[model_handle]
            .local_transform_mut()
            // Our model is too big, fix it by scale.
            .set_scale(Vector3::new(0.05, 0.05, 0.05));

        context
            .lock()
            .unwrap()
            .report_progress(0.66, "Loading animation...");

        // Add simple animation for our model. Animations are loaded from model resources -
        // this is because animation is a set of skeleton bones with their own transforms.
        let walk_animation_resource = resource_manager
            .request_model("examples/data/mutant/walk.fbx")
            .await
            .unwrap();

        // Once animation resource is loaded it must be re-targeted to our model instance.
        // Why? Because animation in *resource* uses information about *resource* bones,
        // not model instance bones, retarget_animations maps animations of each bone on
        // model instance so animation will know about nodes it should operate on.
        let walk_animation = *walk_animation_resource
            .retarget_animations(model_handle, &mut scene)
            .get(0)
            .unwrap();

        context.lock().unwrap().report_progress(1.0, "Done");

        context.lock().unwrap().data = Some(Self {
            scene,
            model_handle,
            walk_animation,
        });
    }
}

struct AsyncLoaderContext {
    data: Option<SceneLoader>,
    message: String,
    progress: f32,
}

impl AsyncLoaderContext {
    fn report_progress(&mut self, progress: f32, message: &str) {
        self.progress = progress;
        self.message = message.to_owned();
        println!("Loading progress: {}% - {}", progress * 100.0, message);
    }

    fn load_with(resource_manager: ResourceManager) -> Arc<Mutex<Self>> {
        // Create load context - it will be shared with caller and loader threads.
        let context = Arc::new(Mutex::new(Self {
            data: None,
            message: "Starting..".to_string(),
            progress: 0.0,
        }));
        let result = context.clone();

        // Spawn separate thread which will create scene by loading various assets.
        std::thread::spawn(move || {
            // Scene will be loaded in separate thread.
            futures::executor::block_on(SceneLoader::load_with(resource_manager, context))
        });

        // Immediately return shared context.
        result
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct GameScene {
    scene: Handle<Scene>,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
}

struct Game {
    interface: Interface,
    input_controller: InputController,
    model_angle: f32,
    game_scene: Option<GameScene>,
    scene_loader: Arc<Mutex<AsyncLoaderContext>>,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Check each frame if our scene is created - here we just trying to lock context
        // without blocking, it is important for main thread to be functional while other
        // thread still loading data.
        if let Ok(mut loader_context) = self.scene_loader.try_lock() {
            if let Some(loader) = loader_context.data.take() {
                self.game_scene = Some(GameScene {
                    scene: context.scenes.add(loader.scene),
                    model_handle: loader.model_handle,
                    walk_animation: loader.walk_animation,
                });

                // Once scene is loaded, we should hide progress bar and text.
                context
                    .user_interface
                    .send_message(WidgetMessage::visibility(
                        self.interface.progress_bar,
                        MessageDirection::ToWidget,
                        false,
                    ));
                context
                    .user_interface
                    .send_message(WidgetMessage::visibility(
                        self.interface.progress_text,
                        MessageDirection::ToWidget,
                        false,
                    ));
            }

            // Report progress in UI.
            context
                .user_interface
                .send_message(ProgressBarMessage::progress(
                    self.interface.progress_bar,
                    MessageDirection::ToWidget,
                    loader_context.progress,
                ));
            context.user_interface.send_message(TextMessage::text(
                self.interface.progress_text,
                MessageDirection::ToWidget,
                format!(
                    "Loading scene: {}%\n{}",
                    loader_context.progress * 100.0,
                    loader_context.message
                ),
            ));
        }

        // Update scene only if it is loaded.
        if let Some(game_scene) = self.game_scene.as_mut() {
            // Use stored scene handle to borrow a mutable reference of scene in
            // engine.
            let scene = &mut context.scenes[game_scene.scene];

            // Our animation must be applied to scene explicitly, otherwise
            // it will have no effect.
            scene
                .animations
                .get_mut(game_scene.walk_animation)
                .get_pose()
                .apply(&mut scene.graph);

            // Rotate model according to input controller state.
            if self.input_controller.rotate_left {
                self.model_angle -= 5.0f32.to_radians();
            } else if self.input_controller.rotate_right {
                self.model_angle += 5.0f32.to_radians();
            }

            scene.graph[game_scene.model_handle]
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    self.model_angle,
                ));
        }

        // While scene is loading, we will update progress bar.
        let fps = context.renderer.get_statistics().frames_per_second;
        let debug_text = format!(
            "Example 02 - Asynchronous Scene Loading\nUse [A][D] keys to rotate model.\nFPS: {}",
            fps
        );
        context.user_interface.send_message(TextMessage::text(
            self.interface.debug_text,
            MessageDirection::ToWidget,
            debug_text,
        ));
    }

    fn id(&self) -> Uuid {
        GameConstructor::type_uuid()
    }

    fn on_os_event(
        &mut self,
        event: &Event<()>,
        context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::Resized(size) => {
                    // Root UI node should be resized, otherwise progress bar will stay
                    // in wrong position after resize.
                    let size = size.to_logical(context.window.scale_factor());
                    context.user_interface.send_message(WidgetMessage::width(
                        self.interface.root,
                        MessageDirection::ToWidget,
                        size.width,
                    ));
                    context.user_interface.send_message(WidgetMessage::height(
                        self.interface.root,
                        MessageDirection::ToWidget,
                        size.height,
                    ));
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                    if let Some(key_code) = input.virtual_keycode {
                        match key_code {
                            VirtualKeyCode::A => {
                                self.input_controller.rotate_left =
                                    input.state == ElementState::Pressed
                            }
                            VirtualKeyCode::D => {
                                self.input_controller.rotate_right =
                                    input.state == ElementState::Pressed
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }
    }
}

struct GameConstructor;

impl TypeUuidProvider for GameConstructor {
    fn type_uuid() -> Uuid {
        uuid!("f615ac42-b259-4a23-bb44-407d753ac178")
    }
}

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        // Create simple user interface that will show some useful info.
        let screen_size = context
            .window
            .inner_size()
            .to_logical(context.window.scale_factor());
        let interface = create_ui(
            &mut context.user_interface.build_ctx(),
            Vector2::new(screen_size.width, screen_size.height),
        );

        Box::new(Game {
            interface,
            input_controller: InputController {
                rotate_left: false,
                rotate_right: false,
            },
            model_angle: 180.0f32.to_radians(),
            game_scene: None,
            scene_loader: AsyncLoaderContext::load_with(context.resource_manager.clone()),
        })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor
        .get_window()
        .set_title("Example - Asynchronous Scene Loading");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
