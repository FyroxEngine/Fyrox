use crate::scene::EditorScene;
use fyrox::core::algebra::UnitQuaternion;
use fyrox::core::sstorage::ImmutableString;
use fyrox::material::{Material, PropertyValue, SharedMaterial};
use fyrox::scene::graph::Graph;
use fyrox::scene::light::directional::DirectionalLightBuilder;
use fyrox::scene::light::BaseLightBuilder;
use fyrox::scene::node::Node;
use fyrox::scene::pivot::PivotBuilder;
use fyrox::{
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        pool::Handle,
    },
    engine::Engine,
    resource::texture::{TextureResource, TextureResourceExtension},
    scene::{
        base::BaseBuilder,
        camera::{CameraBuilder, SkyBoxKind},
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        transform::TransformBuilder,
        Scene,
    },
};

pub struct SceneGizmo {
    pub scene: Handle<Scene>,
    pub render_target: TextureResource,
    pub camera_pivot: Handle<Node>,
    pub camera_hinge: Handle<Node>,
}

fn make_cone(transform: Matrix4<f32>, color: Color, graph: &mut Graph) -> Handle<Node> {
    let mut material = Material::standard();

    material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(color),
        )
        .unwrap();

    MeshBuilder::new(BaseBuilder::new().with_cast_shadows(false))
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
            SurfaceData::make_cone(16, 0.3, 1.0, &transform),
        ))
        .with_material(SharedMaterial::new(material))
        .build()])
        .build(graph)
}

impl SceneGizmo {
    pub fn new(engine: &mut Engine) -> Self {
        let mut scene = Scene::new();

        let render_target = TextureResource::new_render_target(85, 85);
        scene.rendering_options.render_target = Some(render_target.clone());
        scene.rendering_options.clear_color = Some(Color::TRANSPARENT);

        DirectionalLightBuilder::new(BaseLightBuilder::new(BaseBuilder::new()))
            .build(&mut scene.graph);

        MeshBuilder::new(
            BaseBuilder::new().with_cast_shadows(false).with_children(&[
                make_cone(
                    Matrix4::new_translation(&Vector3::new(0.0, -1.50, 0.0)),
                    Color::WHITE,
                    &mut scene.graph,
                ),
                make_cone(
                    Matrix4::new_translation(&Vector3::new(0.0, 1.50, 0.0))
                        * UnitQuaternion::from_axis_angle(
                            &Vector3::x_axis(),
                            180.0f32.to_radians(),
                        )
                        .to_homogeneous(),
                    Color::GREEN,
                    &mut scene.graph,
                ),
                make_cone(
                    Matrix4::new_translation(&Vector3::new(-1.50, 0.0, 0.0))
                        * UnitQuaternion::from_axis_angle(
                            &Vector3::z_axis(),
                            -90.0f32.to_radians(),
                        )
                        .to_homogeneous(),
                    Color::RED,
                    &mut scene.graph,
                ),
                make_cone(
                    Matrix4::new_translation(&Vector3::new(1.50, 0.0, 0.0))
                        * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), 90.0f32.to_radians())
                            .to_homogeneous(),
                    Color::WHITE,
                    &mut scene.graph,
                ),
                make_cone(
                    Matrix4::new_translation(&Vector3::new(0.0, 0.0, 1.50))
                        * UnitQuaternion::from_axis_angle(
                            &Vector3::x_axis(),
                            -90.0f32.to_radians(),
                        )
                        .to_homogeneous(),
                    Color::BLUE,
                    &mut scene.graph,
                ),
                // -Z
                make_cone(
                    Matrix4::new_translation(&Vector3::new(0.0, 0.0, -1.50))
                        * UnitQuaternion::from_axis_angle(&Vector3::x_axis(), 90.0f32.to_radians())
                            .to_homogeneous(),
                    Color::WHITE,
                    &mut scene.graph,
                ),
            ]),
        )
        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
            SurfaceData::make_cube(Matrix4::identity()),
        ))
        .build()])
        .build(&mut scene.graph);

        let camera_hinge;
        let camera_pivot = PivotBuilder::new(BaseBuilder::new().with_children(&[{
            camera_hinge = PivotBuilder::new(
                BaseBuilder::new().with_children(&[CameraBuilder::new(
                    BaseBuilder::new().with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(Vector3::new(0.0, 0.0, -3.0))
                            .build(),
                    ),
                )
                .with_specific_skybox(SkyBoxKind::None)
                .build(&mut scene.graph)]),
            )
            .build(&mut scene.graph);
            camera_hinge
        }]))
        .build(&mut scene.graph);

        scene.graph.update_hierarchical_data();

        Self {
            scene: engine.scenes.add(scene),
            render_target,
            camera_pivot,
            camera_hinge,
        }
    }

    pub fn sync_rotations(&self, editor_scene: &EditorScene, engine: &mut Engine) {
        let graph = &engine.scenes[editor_scene.scene].graph;
        let hinge_rotation = **graph[editor_scene.camera_controller.camera_hinge]
            .local_transform()
            .rotation();
        let pivot_rotation = **graph[editor_scene.camera_controller.pivot]
            .local_transform()
            .rotation();

        let gizmo_graph = &mut engine.scenes[self.scene].graph;

        gizmo_graph[self.camera_hinge]
            .local_transform_mut()
            .set_rotation(hinge_rotation);
        gizmo_graph[self.camera_pivot]
            .local_transform_mut()
            .set_rotation(pivot_rotation);
    }
}
