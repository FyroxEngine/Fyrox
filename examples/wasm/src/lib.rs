#![cfg(target_arch = "wasm32")]

//! Example - WebAssembly
//!
//! Warning - Work in progress!

use fyrox::{
    asset::manager::ResourceManager,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        parking_lot::Mutex,
        pool::Handle,
        sstorage::ImmutableString,
        wasm_bindgen::{self, prelude::*},
    },
    dpi::{LogicalSize, Size},
    engine::{
        Engine, EngineInitParams, GraphicsContext, GraphicsContextParams, SerializationContext,
    },
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
    },
    gui::{BuildContext, UiNode},
    keyboard::KeyCode,
    material::{shader::SamplerFallback, Material, PropertyValue, SharedMaterial},
    resource::{
        model::{Model, ModelResourceExtension},
        texture::{Texture, TextureWrapMode},
    },
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBoxBuilder},
        graph::Graph,
        light::{point::PointLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        node::Node,
        sound::{SoundBuffer, SoundBuilder, Status},
        transform::TransformBuilder,
        Scene,
    },
    utils::translate_event,
    window::WindowAttributes,
};
use std::{panic, sync::Arc};

fn create_ui(ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()).build(ctx)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}

fn hook_impl(info: &panic::PanicInfo) {
    let mut msg = info.to_string();

    // Add the error stack to our message.
    //
    // This ensures that even if the `console` implementation doesn't
    // include stacks for `console.error`, the stack is still available
    // for the user. Additionally, Firefox's console tries to clean up
    // stack traces, and ruins Rust symbols in the process
    // (https://bugzilla.mozilla.org/show_bug.cgi?id=1519569) but since
    // it only touches the logged message's associated stack, and not
    // the message's contents, by including the stack in the message
    // contents we make sure it is available to the user.
    msg.push_str("\n\nStack:\n\n");
    let e = Error::new();
    let stack = e.stack();
    msg.push_str(&stack);

    // Safari's devtools, on the other hand, _do_ mess with logged
    // messages' contents, so we attempt to break their heuristics for
    // doing that by appending some whitespace.
    // https://github.com/rustwasm/console_error_panic_hook/issues/7
    msg.push_str("\n\n");

    // Finally, log the panic with `console.error`!
    error(msg);
}

/// A panic hook for use with
/// [`std::panic::set_hook`](https://doc.rust-lang.org/nightly/std/panic/fn.set_hook.html)
/// that logs panics into
/// [`console.error`](https://developer.mozilla.org/en-US/docs/Web/API/Console/error).
///
/// On non-wasm targets, prints the panic to `stderr`.
pub fn hook(info: &panic::PanicInfo) {
    hook_impl(info);
}

/// Set the `console.error` panic hook the first time this is called. Subsequent
/// invocations do nothing.
#[inline]
pub fn set_once() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();
    SET_HOOK.call_once(|| {
        panic::set_hook(Box::new(hook));
    });
}

struct GameScene {
    scene: Scene,
    model: Handle<Node>,
}

struct SceneContext {
    data: Option<GameScene>,
}

/// Creates a camera at given position with a skybox.
pub async fn create_camera(
    resource_manager: ResourceManager,
    position: Vector3<f32>,
    graph: &mut Graph,
) -> Handle<Node> {
    // Load skybox textures in parallel.
    let (front, back, left, right, top, bottom) = fyrox::core::futures::join!(
        resource_manager.request::<Texture, _>("data/textures/DarkStormyFront.jpg"),
        resource_manager.request::<Texture, _>("data/textures/DarkStormyBack.jpg"),
        resource_manager.request::<Texture, _>("data/textures/DarkStormyLeft.jpg"),
        resource_manager.request::<Texture, _>("data/textures/DarkStormyRight.jpg"),
        resource_manager.request::<Texture, _>("data/textures/DarkStormyUp.jpg"),
        resource_manager.request::<Texture, _>("data/textures/DarkStormyDown.jpg")
    );

    // Unwrap everything.
    let skybox = SkyBoxBuilder {
        front: Some(front.unwrap()),
        back: Some(back.unwrap()),
        left: Some(left.unwrap()),
        right: Some(right.unwrap()),
        top: Some(top.unwrap()),
        bottom: Some(bottom.unwrap()),
    }
    .build()
    .unwrap();

    // Set S and T coordinate wrap mode, ClampToEdge will remove any possible seams on edges
    // of the skybox.
    if let Some(skybox_texture) = skybox.cubemap() {
        let mut data = skybox_texture.data_ref();
        data.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
        data.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
    }

    // Camera is our eyes in the world - you won't see anything without it.
    CameraBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(position)
                .build(),
        ),
    )
    .with_skybox(skybox)
    .build(graph)
}

async fn create_scene(resource_manager: ResourceManager, context: Arc<Mutex<SceneContext>>) {
    let mut scene = Scene::new();

    // Add music.
    SoundBuilder::new(BaseBuilder::new())
        .with_buffer(
            resource_manager
                .request::<SoundBuffer, _>("data/music.ogg")
                .await
                .ok(),
        )
        .with_status(Status::Playing)
        .build(&mut scene.graph);

    scene.ambient_lighting_color = Color::opaque(200, 200, 200);

    create_camera(
        resource_manager.clone(),
        Vector3::new(0.0, 6.0, -12.0),
        &mut scene.graph,
    )
    .await;

    PointLightBuilder::new(BaseLightBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, 12.0, 0.0))
                .build(),
        ),
    ))
    .with_radius(20.0)
    .build(&mut scene.graph);

    let (model_resource, walk_animation_resource) = fyrox::core::futures::join!(
        resource_manager.request::<Model, _>("data/mutant/mutant.FBX"),
        resource_manager.request::<Model, _>("data/mutant/walk.fbx")
    );

    // Instantiate model on scene - but only geometry, without any animations.
    // Instantiation is a process of embedding model resource data in desired scene.
    let model = model_resource.unwrap().instantiate(&mut scene);

    // Now we have whole sub-graph instantiated, we can start modifying model instance.
    scene.graph[model]
        .local_transform_mut()
        // Our model is too big, fix it by scale.
        .set_scale(Vector3::new(0.05, 0.05, 0.05));

    // Add simple animation for our model. Animations are loaded from model resources -
    // this is because animation is a set of skeleton bones with their own transforms.
    // Once animation resource is loaded it must be re-targeted to our model instance.
    // Why? Because animation in *resource* uses information about *resource* bones,
    // not model instance bones, retarget_animations maps animations of each bone on
    // model instance so animation will know about nodes it should operate on.
    walk_animation_resource
        .unwrap()
        .retarget_animations(model, &mut scene.graph);

    let mut material = Material::standard();
    material
        .set_property(
            &ImmutableString::new("diffuseTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request::<Texture, _>("data/textures/concrete.jpg")),
                fallback: SamplerFallback::White,
            },
        )
        .unwrap();

    // Add floor.
    MeshBuilder::new(
        BaseBuilder::new().with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, -0.25, 0.0))
                .build(),
        ),
    )
    .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
        SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
            25.0, 0.25, 25.0,
        ))),
    ))
    .with_material(SharedMaterial::new(material))
    .build()])
    .build(&mut scene.graph);

    context.lock().data = Some(GameScene { scene, model })
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

// Rename `main` to `main_js` as workaround for tests
// https://github.com/rustwasm/wasm-bindgen/issues/2206
#[wasm_bindgen]
pub fn main_js() {
    set_once();

    let event_loop = EventLoop::new();

    let graphics_context_params = GraphicsContextParams {
        window_attributes: WindowAttributes {
            title: "Example - WASM".to_string(),
            resizable: true,
            inner_size: Some(Size::Logical(LogicalSize::new(800.0, 600.0))),
            ..Default::default()
        },
        vsync: true,
    };

    let mut engine = Engine::new(EngineInitParams {
        graphics_context_params,
        resource_manager: ResourceManager::new(),
        serialization_context: Arc::new(SerializationContext::new()),
    })
    .unwrap();

    let load_context = Arc::new(Mutex::new(SceneContext { data: None }));

    fyrox::core::wasm_bindgen_futures::spawn_local(create_scene(
        engine.resource_manager.clone(),
        load_context.clone(),
    ));

    let mut scene_handle = Handle::NONE;
    let mut model_handle = Handle::NONE;

    // Create simple user interface that will show some useful info.
    let debug_text = create_ui(&mut engine.user_interface.build_ctx());

    let mut previous = fyrox::core::instant::Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut lag = 0.0;

    // We will rotate model using keyboard input.
    let mut model_angle = 180.0f32.to_radians();

    // Create input controller - it will hold information about needed actions.
    let mut input_controller = InputController {
        rotate_left: false,
        rotate_right: false,
    };

    // Finally run our event loop which will respond to OS and window events and update
    // engine state accordingly. Engine lets you to decide which event should be handled,
    // this is minimal working example if how it should be.
    event_loop.run(move |event, window_target, control_flow| {
        match event {
            Event::MainEventsCleared => {
                // This main game loop - it has fixed time step which means that game
                // code will run at fixed speed even if renderer can't give you desired
                // 60 fps.
                let elapsed = previous.elapsed();
                previous = fyrox::core::instant::Instant::now();
                lag += elapsed.as_secs_f32();
                while lag >= fixed_timestep {
                    if let Some(scene) = load_context.lock().data.take() {
                        scene_handle = engine.scenes.add(scene.scene);
                        model_handle = scene.model;
                    }

                    if scene_handle.is_some() && model_handle.is_some() {
                        let scene = &mut engine.scenes[scene_handle];

                        // Rotate model according to input controller state.
                        if input_controller.rotate_left {
                            model_angle -= 5.0f32.to_radians();
                        } else if input_controller.rotate_right {
                            model_angle += 5.0f32.to_radians();
                        }

                        scene.graph[model_handle]
                            .local_transform_mut()
                            .set_rotation(UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                model_angle,
                            ));
                    }

                    if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                        let fps = ctx.renderer.get_statistics().frames_per_second;
                        let text = format!(
                            "Example - WASM\nUse [A][D] keys to rotate model.\nFPS: {}\nAngle: {}",
                            fps, model_angle
                        );
                        engine.user_interface.send_message(TextMessage::text(
                            debug_text,
                            MessageDirection::ToWidget,
                            text,
                        ));
                    }

                    engine.update(fixed_timestep, control_flow, &mut lag, Default::default());

                    lag -= fixed_timestep;
                }

                // It is very important to "pump" messages from UI. Even if don't need to
                // respond to such message, you should call this method, otherwise UI
                // might behave very weird.
                while let Some(_ui_event) = engine.user_interface.poll_message() {
                    // ************************
                    // Put your data model synchronization code here. It should
                    // take message and update data in your game according to
                    // changes in UI.
                    // ************************
                }

                // Rendering must be explicitly requested and handled after RedrawRequested event is received.
                if let GraphicsContext::Initialized(ref ctx) = engine.graphics_context {
                    ctx.window.request_redraw();
                }
            }
            Event::Resumed => {
                engine.initialize_graphics_context(window_target).unwrap();
                engine
                    .graphics_context
                    .as_initialized_mut()
                    .renderer
                    .set_backbuffer_clear_color(Color::opaque(150, 150, 255));
            }
            Event::Suspended => {
                engine.destroy_graphics_context().unwrap();
            }
            Event::RedrawRequested(_) => {
                // Run renderer at max speed - it is not tied to game code.
                engine.render().unwrap();
            }
            Event::WindowEvent { event, .. } => {
                match &event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        // It is very important to handle Resized event from window, because
                        // renderer knows nothing about window size - it must be notified
                        // directly when window size has changed.
                        engine.set_frame_size((*size).into()).unwrap();
                    }
                    WindowEvent::KeyboardInput { event: input, .. } => {
                        // Handle key input events via `WindowEvent`, not via `DeviceEvent` (#32)
                        match input.physical_key {
                            KeyCode::KeyA => {
                                input_controller.rotate_left = input.state == ElementState::Pressed
                            }
                            KeyCode::KeyD => {
                                input_controller.rotate_right = input.state == ElementState::Pressed
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }

                // It is very important to "feed" user interface (UI) with events coming
                // from main window, otherwise UI won't respond to mouse, keyboard, or any
                // other event.
                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}
