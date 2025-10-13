//! Vulkan Backend Test Example
//!
//! This example demonstrates how to use the new Vulkan backend in Fyrox.
//! It creates a simple application that initializes the Vulkan graphics backend.

use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        log::Log,
        pool::Handle,
    },
    engine::{Engine, EngineInitParams, GraphicsBackend},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    material::{Material, MaterialResource},
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    resource::texture::Texture,
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBox},
        light::{directional::DirectionalLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder,
        },
        node::Node,
        transform::TransformBuilder,
        Scene,
    },
};
use std::{path::Path, sync::Arc};

struct GamePlugin {
    scene: Handle<Scene>,
}

impl GamePlugin {
    pub fn new() -> Self {
        Self {
            scene: Handle::NONE,
        }
    }
}

impl Plugin for GamePlugin {
    fn on_deinit(&mut self, _context: PluginContext) {
        // Clean up resources here
    }

    fn update(&mut self, _context: &mut PluginContext) {
        // Game logic update here
    }

    fn on_os_event(&mut self, _event: &Event<()>, _context: PluginContext) {
        // Handle OS events here
    }
}

impl PluginConstructor for GamePlugin {
    fn register(&self, _context: PluginRegistrationContext) {
        // Register plugin resources here
    }

    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let mut plugin = GamePlugin::new();

        // Create a simple scene with a cube
        let scene = create_test_scene(context);
        plugin.scene = context.scenes.add(scene);

        Box::new(plugin)
    }
}

fn create_test_scene(context: PluginContext) -> Scene {
    let mut scene = Scene::new();

    // Add a camera
    let camera = CameraBuilder::new(BaseBuilder::new().with_name("Camera"))
        .with_projection(fyrox::scene::camera::Projection::perspective(
            70.0f32.to_radians(),
            1.0,
            0.1,
            1000.0,
        ))
        .build(&mut scene.graph);

    scene.graph[camera]
        .local_transform_mut()
        .set_position(Vector3::new(0.0, 1.0, -3.0));

    // Add a directional light (sun)
    DirectionalLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new().with_name("Sun")))
        .build(&mut scene.graph);

    // Create a simple cube mesh
    let cube_mesh = MeshBuilder::new(
        BaseBuilder::new().with_name("Cube").with_local_transform(
            TransformBuilder::new()
                .with_local_position(Vector3::new(0.0, 0.0, 0.0))
                .build(),
        ),
    )
    .with_surfaces(vec![create_cube_surface(context)])
    .build(&mut scene.graph);

    scene
}

fn create_cube_surface(context: PluginContext) -> SurfaceData {
    // Simple cube vertices and indices
    let vertices = vec![
        // Front face
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
        // Back face
        [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, -0.5, -0.5],
        // Top face
        [-0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, -0.5],
        // Bottom face
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        // Right face
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [0.5, -0.5, 0.5],
        // Left face
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
        [-0.5, 0.5, -0.5],
    ];

    let indices = vec![
        0, 1, 2, 0, 2, 3, // Front
        4, 5, 6, 4, 6, 7, // Back
        8, 9, 10, 8, 10, 11, // Top
        12, 13, 14, 12, 14, 15, // Bottom
        16, 17, 18, 16, 18, 19, // Right
        20, 21, 22, 20, 22, 23, // Left
    ];

    let positions: Vec<Vector3<f32>> = vertices
        .iter()
        .map(|v| Vector3::new(v[0], v[1], v[2]))
        .collect();

    SurfaceData::make_cube()
}

fn main() {
    // Initialize logging
    Log::add_listener(Box::new(fyrox::core::log::print::PrintListener::new()));

    println!("🚀 Starting Vulkan Backend Test...");

    // Create event loop
    let event_loop = EventLoop::new();

    // Initialize the engine with Vulkan backend
    let mut engine = Engine::new(EngineInitParams {
        window_title: "Fyrox Vulkan Backend Test".to_string(),
        graphics_backend: GraphicsBackend::Vulkan, // Use the new Vulkan backend!
        resource_manager: Default::default(),
        serialization_context: Default::default(),
        window_attributes: Default::default(),
        widget_constructors: Default::default(),
        plugins: vec![Box::new(GamePlugin::new())],
        headless: false,
    })
    .unwrap();

    println!("✅ Engine initialized with Vulkan backend!");

    // Main loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                // Update and render
                engine.update();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Handle window resize
                if size.width > 0 && size.height > 0 {
                    // Engine handles resize automatically
                }
            }
            _ => {}
        }

        *control_flow = ControlFlow::Poll;
    });
}
