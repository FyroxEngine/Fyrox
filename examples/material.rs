//! Example - Custom materials and shaders.
//!
//! Difficulty: Medium.
//!
//! This example shows how to create simple scene with a mesh with custom shader.

pub mod shared;

use crate::shared::create_camera;
use fyrox::material::SharedMaterial;
use fyrox::scene::mesh::surface::SurfaceSharedData;
use fyrox::{
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        futures::executor::block_on,
        pool::Handle,
        sstorage::ImmutableString,
    },
    engine::{executor::Executor, resource_manager::ResourceManager},
    event_loop::ControlFlow,
    gui::{
        message::MessageDirection,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        UiNode,
    },
    material::{shader::SamplerFallback, Material, PropertyValue},
    plugin::{Plugin, PluginConstructor, PluginContext},
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

struct Game {
    debug_text: Handle<UiNode>,
    material: SharedMaterial,
    time: f32,
}

impl Plugin for Game {
    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        self.material
            .lock()
            .set_property(
                &ImmutableString::new("time"),
                PropertyValue::Float(self.time),
            )
            .unwrap();

        self.time += context.dt;

        context.user_interface.send_message(TextMessage::text(
            self.debug_text,
            MessageDirection::ToWidget,
            format!(
                "Example - Materials and Shaders\nFPS: {}",
                context.renderer.get_statistics().frames_per_second
            ),
        ));
    }
}

fn create_custom_material(resource_manager: ResourceManager) -> SharedMaterial {
    let shader =
        block_on(resource_manager.request_shader("examples/data/shaders/custom.shader")).unwrap();

    let mut material = Material::from_shader(shader, Some(resource_manager.clone()));

    material
        .set_property(
            &ImmutableString::new("diffuseTexture"),
            PropertyValue::Sampler {
                value: Some(resource_manager.request_texture("examples/data/concrete2.dds")),
                fallback: SamplerFallback::White,
            },
        )
        .unwrap();

    SharedMaterial::new(material)
}

struct GameConstructor;

impl PluginConstructor for GameConstructor {
    fn create_instance(
        &self,
        _override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        let mut scene = Scene::new();

        // Set ambient light.
        scene.ambient_lighting_color = Color::opaque(0, 0, 0);

        // Camera is our eyes in the world - you won't see anything without it.
        block_on(create_camera(
            context.resource_manager.clone(),
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

        let material = create_custom_material(context.resource_manager.clone());

        // Add cylinder with custom shader.
        MeshBuilder::new(BaseBuilder::new())
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                SurfaceData::make_cylinder(20, 0.75, 2.0, true, &Matrix4::identity()),
            ))
            .with_material(material.clone())
            .build()])
            .build(&mut scene.graph);

        context.scenes.add(scene);

        Box::new(Game {
            debug_text: TextBuilder::new(WidgetBuilder::new())
                .build(&mut context.user_interface.build_ctx()),
            material,
            time: 0.0,
        })
    }
}

fn main() {
    let mut executor = Executor::new();
    executor
        .get_window()
        .set_title("Example - Materials and Shaders");
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}
