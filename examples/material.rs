//! Example - Custom materials and shaders.
//!
//! Difficulty: Medium.
//!
//! This example shows how to create simple scene with a mesh with custom shader.

pub mod shared;

use crate::shared::create_camera;
use rg3d::core::sstorage::ImmutableString;
use rg3d::{
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        futures::executor::block_on,
        parking_lot::Mutex,
        pool::Handle,
    },
    engine::{framework::prelude::*, resource_manager::ResourceManager, Engine},
    event_loop::ControlFlow,
    gui::{
        message::{MessageDirection, TextMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        UiNode,
    },
    material::{shader::SamplerFallback, Material, PropertyValue},
    scene::{
        base::BaseBuilder,
        light::{point::PointLightBuilder, BaseLightBuilder},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData},
            MeshBuilder,
        },
        transform::TransformBuilder,
        Scene,
    },
};
use std::sync::Arc;

struct Game {
    debug_text: Handle<UiNode>,
    material: Arc<Mutex<Material>>,
    time: f32,
}

fn create_custom_material(resource_manager: ResourceManager) -> Arc<Mutex<Material>> {
    let shader =
        block_on(resource_manager.request_shader("examples/data/shaders/custom.shader")).unwrap();

    let mut material = Material::from_shader(shader, Some(resource_manager.clone()));

    material
        .set_property(
            &ImmutableString::new("diffuseTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request_texture("examples/data/concrete2.dds", None)),
                fallback: SamplerFallback::White,
            },
        )
        .unwrap();

    Arc::new(Mutex::new(material))
}

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(0, 0, 0);

        // Camera is our eyes in the world - you won't see anything without it.
        block_on(create_camera(
            engine.resource_manager.clone(),
            Vector3::new(0.0, 1.0, -3.0),
            &mut scene.graph,
        ));

        // Add some light.
        PointLightBuilder::new(BaseLightBuilder::new(
            BaseBuilder::new().with_local_transform(
                TransformBuilder::new()
                    .with_local_position(Vector3::new(0.5, 1.0, -1.5))
                    .build(),
            ),
        ))
        .with_radius(5.0)
        .build(&mut scene.graph);

        let material = create_custom_material(engine.resource_manager.clone());

        // Add cylinder with custom shader.
        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
                SurfaceData::make_cylinder(20, 0.75, 2.0, true, &Matrix4::identity()),
            )))
            .with_material(material.clone())
            .build()])
            .build(&mut scene.graph);

        engine.scenes.add(scene);

        Self {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut engine.user_interface.build_ctx()),
            material,
            time: 0.0,
        }
    }

    fn on_tick(&mut self, engine: &mut Engine, dt: f32, _: &mut ControlFlow) {
        self.material
            .lock()
            .set_property(
                &ImmutableString::new("time"),
                PropertyValue::Float(self.time),
            )
            .unwrap();

        self.time += dt;

        engine.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            format!(
                "Example - Materials and Shaders\nFPS: {}",
                engine.renderer.get_statistics().frames_per_second
            ),
        ));
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example - Materials and Shaders")
        .run();
}
