//! Example 12. Custom loader
//!
//! Difficulty: Moderate.
//!
//! This example shows how to create a custom loader. It is a very basic example and in future it should be improved by 
//! writing some more complex loader such as loading a model from ply or obj file.

/// For simplicity we just simply wrap the default loader and log the invocation t
pub mod shared;

use crate::shared::create_camera;
use fyrox::engine::framework::{GameState, Framework};
use fyrox::engine::resource_manager::ResourceManager;
use fyrox::engine::Engine;
use fyrox::engine::resource_manager::container::event::ResourceEventBroadcaster;
use fyrox::engine::resource_manager::loader::{ResourceLoader, BoxedLoaderFuture};
use fyrox::engine::resource_manager::loader::model::ModelLoader;
use fyrox::engine::resource_manager::loader::texture::TextureLoader;
use fyrox::material::{Material, PropertyValue};
use fyrox::material::shader::SamplerFallback;
use fyrox::resource::model::{ModelImportOptions, Model};
use fyrox::resource::texture::{TextureImportOptions, Texture};
use fyrox::scene::Scene;
use fyrox::scene::base::BaseBuilder;
use fyrox::scene::light::BaseLightBuilder;
use fyrox::scene::light::point::PointLightBuilder;
use fyrox::scene::mesh::MeshBuilder;
use fyrox::scene::mesh::surface::{SurfaceBuilder, SurfaceData};
use fyrox::scene::transform::TransformBuilder;
use fyrox_core::algebra::{Matrix4, Vector3};
use fyrox_core::color::Color;
use fyrox_core::parking_lot::Mutex;
use fyrox_core::pool::Handle;
use fyrox_core::sstorage::ImmutableString;
use std::sync::Arc;

struct CustomModelLoader(Arc<ModelLoader>);

impl ResourceLoader<Model, ModelImportOptions> for CustomModelLoader {
    fn load(
        &self,
        resource: Model,
        default_import_options: ModelImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Model>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        // Arc is required as BoxedLoaderFuture has a static lifetime and hence self cannot be 
        // moved into an async block. 
        let loader = self.0.clone();

        Box::pin(async move {
            println!(
                "CUSTOM LOADER: loading model {:?}",
                resource.state().path()
            );
            loader
                .load(
                    resource,
                    default_import_options,
                    event_broadcaster,
                    reload,
                )
                .await
        })
    }
}

/// For simplicity we just simply wrap the default loader and log the invocation to the console.
struct CustomTextureLoader(Arc<TextureLoader>);

impl ResourceLoader<Texture, TextureImportOptions> for CustomTextureLoader {
    fn load(
        &self,
        resource: Texture,
        default_import_options: TextureImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Texture>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        // Arc is required as BoxedLoaderFuture has a static lifetime and hence self cannot be 
        // moved into an async block. 
        let loader = self.0.clone();

        Box::pin(async move {
            println!(
                "CUSTOM LOADER: loading texture {:?}",
                resource.state().path()
            );
            loader
                .load(
                    resource,
                    default_import_options,
                    event_broadcaster,
                    reload,
                )
                .await
        })
    }
}

struct GameSceneLoader {
    scene: Scene
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

        // Add some model with animation.
        let (model_resource, walk_animation_resource) = fyrox::core::futures::join!(
            resource_manager.request_model("examples/data/mutant/mutant.FBX",),
            resource_manager.request_model("examples/data/mutant/walk.fbx",)
        );

        let model_handle = model_resource.unwrap().instantiate_geometry(&mut scene);
        scene.graph[model_handle]
            .local_transform_mut()
            .set_scale(Vector3::new(0.05, 0.05, 0.05));

        let walk_animation = *walk_animation_resource
            .unwrap()
            .retarget_animations(model_handle, &mut scene)
            .get(0)
            .unwrap();

        let mut material = Material::standard();
        material
            .set_property(
                &ImmutableString::new("diffuseTexture"),
                PropertyValue::Sampler {
                    value: Some(resource_manager.request_texture("examples/data/concrete2.dds")),
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
        .with_surfaces(vec![SurfaceBuilder::new(Arc::new(Mutex::new(
            SurfaceData::make_cube(Matrix4::new_nonuniform_scaling(&Vector3::new(
                25.0, 0.25, 25.0,
            ))),
        )))
        .with_material(Arc::new(Mutex::new(material)))
        .build()])
        .build(&mut scene.graph);

        Self {
            scene,
        }
    }
}

struct Game {
    scene: Handle<Scene>,
}

impl GameState for Game {
    fn init(engine: &mut Engine) -> Self
    where
        Self: Sized,
    {
        let scene = fyrox::core::futures::executor::block_on(GameSceneLoader::load_with(
            engine.resource_manager.clone(),
        ));

        Game{ scene: engine.scenes.add(scene.scene) }
    }
}

fn main() {
    Framework::<Game>::new()
        .unwrap()
        .title("Example 11 - Custom resource loader")
        .run();
}
