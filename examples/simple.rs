//! Example 01. Simple scene.
//!
//! Difficulty: Easy.
//!
//! This example shows how to create simple scene with animated model.

extern crate rg3d;

pub mod shared;

use crate::shared::create_camera;
use rg3d::{
    animation::Animation,
    core::{
        algebra::{Matrix4, UnitQuaternion, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::{resource_manager::ResourceManager, simple::prelude::*},
    event::{ElementState, VirtualKeyCode, WindowEvent},
    gui::{
        message::{MessageDirection, TextMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
    },
    scene::{
        base::BaseBuilder,
        light::{BaseLightBuilder, PointLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder,
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
};
use std::sync::{Arc, RwLock};

struct GameSceneLoader {
    scene: Scene,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
}

impl GameSceneLoader {
    async fn load_with(resource_manager: ResourceManager) -> Self {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(200, 200, 200);

        // Camera is our eyes in the world - you won't see anything without it.
        create_camera(
            resource_manager.clone(),
            Vector3::new(0.0, 6.0, -12.0),
            &mut scene.graph,
        )
        .await;

        // Add some light.
        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, 12.0, 0.0))
                    .build(),
            ),
        ))
        .with_radius(20.0)
        .build(&mut scene.graph);

        // Load model and animation resource in parallel. Is does *not* adds anything to
        // our scene - it just loads a resource then can be used later on to instantiate
        // models from it on scene. Why loading of resource is separated from instantiation?
        // Because it is too inefficient to load a resource every time you trying to
        // create instance of it - much more efficient is to load it once and then make copies
        // of it. In case of models it is very efficient because single vertex and index buffer
        // can be used for all models instances, so memory footprint on GPU will be lower.
        let (model_resource, walk_animation_resource) = rg3d::core::futures::join!(
            resource_manager.request_model("examples/data/mutant.FBX"),
            resource_manager.request_model("examples/data/walk.fbx")
        );

        // Instantiate model on scene - but only geometry, without any animations.
        // Instantiation is a process of embedding model resource data in desired scene.
        let model_handle = model_resource.unwrap().instantiate_geometry(&mut scene);

        // Now we have whole sub-graph instantiated, we can start modifying model instance.
        scene.graph[model_handle]
            .local_transform_mut()
            // Our model is too big, fix it by scale.
            .set_scale(Vector3::new(0.05, 0.05, 0.05));

        // Add simple animation for our model. Animations are loaded from model resources -
        // this is because animation is a set of skeleton bones with their own transforms.
        // Once animation resource is loaded it must be re-targeted to our model instance.
        // Why? Because animation in *resource* uses information about *resource* bones,
        // not model instance bones, retarget_animations maps animations of each bone on
        // model instance so animation will know about nodes it should operate on.
        let walk_animation = *walk_animation_resource
            .unwrap()
            .retarget_animations(model_handle, &mut scene)
            .get(0)
            .unwrap();

        // Add floor.
        MeshBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.0, -0.25, 0.0))
                    .build(),
            ),
        )
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(RwLock::new(
            SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
                25.0, 0.25, 25.0,
            ))),
        )))
        .with_diffuse_texture(resource_manager.request_texture("examples/data/concrete2.dds"))
        .build()])
        .build(&mut scene.graph);

        Self {
            scene,
            model_handle,
            walk_animation,
        }
    }
}

struct InputController {
    rotate_left: bool,
    rotate_right: bool,
}

struct State {
    scene: Handle<Scene>,
    model_handle: Handle<Node>,
    walk_animation: Handle<Animation>,
    input_controller: InputController,
    debug_text: Handle<UiNode>,
    model_angle: f32,
}

impl State {
    pub fn new(loader: GameSceneLoader, engine: &mut GameEngine) -> Self {
        // We will rotate model using keyboard input.
        let mut model_angle = 180.0f32.to_radians();

        // Create input controller - it will hold information about needed actions.
        let mut input_controller = InputController {
            rotate_left: false,
            rotate_right: false,
        };

        Self {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut engine.user_interface.build_ctx()),
            scene: engine.scenes.add(loader.scene),
            model_handle: loader.model_handle,
            walk_animation: loader.walk_animation,
            input_controller,
            model_angle,
        }
    }
}

fn main() {
    // Framework is a simple wrapper that initializes engine and hides game loop details, allowing
    // you to focus only on important things.
    Framework::new()
        .unwrap()
        .title("Example 01 - Simple")
        // Define game state initializer function.
        .init(|engine| {
            // Prepare resource manager - it must be notified where to search textures. When engine
            // loads model resource it automatically tries to load textures it uses. But since most
            // model formats store absolute paths, we can't use them as direct path to load texture
            // instead we telling engine to search textures in given folder.
            engine
                .resource_manager
                .state()
                .set_textures_path("examples/data");

            let scene = rg3d::core::futures::executor::block_on(GameSceneLoader::load_with(
                engine.resource_manager.clone(),
            ));

            State::new(scene, engine)
        })
        // Define a function that will be called when game's window is received an event.
        .window_event(|engine, state, event| {
            let state = state.unwrap();

            if let WindowEvent::KeyboardInput { input, .. } = event {
                if let Some(key_code) = input.virtual_keycode {
                    match key_code {
                        VirtualKeyCode::A => {
                            state.input_controller.rotate_left =
                                input.state == ElementState::Pressed
                        }
                        VirtualKeyCode::D => {
                            state.input_controller.rotate_right =
                                input.state == ElementState::Pressed
                        }
                        _ => (),
                    }
                }
            }
        })
        // Define a function that will be responsible for game logic. It will be called
        // at fixed rate of 60 Hz.
        .tick(|engine, state, dt| {
            let state = state.unwrap();
            let scene = &mut engine.scenes[state.scene];

            // Our animation must be applied to scene explicitly, otherwise
            // it will have no effect.
            scene
                .animations
                .get_mut(state.walk_animation)
                .get_pose()
                .apply(&mut scene.graph);

            // Rotate model according to input controller state.
            if state.input_controller.rotate_left {
                state.model_angle -= 5.0f32.to_radians();
            } else if state.input_controller.rotate_right {
                state.model_angle += 5.0f32.to_radians();
            }

            scene.graph[state.model_handle]
                .local_transform_mut()
                .set_rotation(UnitQuaternion::from_axis_angle(
                    &Vector3::y_axis(),
                    state.model_angle,
                ));

            engine.user_interface.send_message(TextMessage::text(
                state.debug_text,
                MessageDirection::ToWidget,
                format!(
                    "Example 01 - Simple Scene\nUse [A][D] keys to rotate model.\nFPS: {}",
                    engine.renderer.get_statistics().frames_per_second
                ),
            ));
        })
        // Finally, run the game.
        .run();
}
