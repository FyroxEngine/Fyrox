use crate::menu::physics::PhysicsMenu;
use crate::{
    create_terrain_layer_material,
    menu::{create_menu_item, create_root_menu_item},
    scene::commands::{graph::AddNodeCommand, sound::AddSoundSourceCommand},
    Message,
};
use rg3d::{
    core::{algebra::Matrix4, pool::Handle},
    gui::{
        message::{MenuItemMessage, UiMessage, UiMessageData},
        BuildContext, UiNode,
    },
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        decal::DecalBuilder,
        light::{
            directional::DirectionalLightBuilder, point::PointLightBuilder, spot::SpotLightBuilder,
            BaseLightBuilder,
        },
        mesh::{
            surface::{Surface, SurfaceData},
            Mesh, MeshBuilder,
        },
        node::Node,
        particle_system::{
            emitter::{base::BaseEmitterBuilder, sphere::SphereEmitterBuilder},
            ParticleSystemBuilder,
        },
        sprite::SpriteBuilder,
        terrain::{LayerDefinition, TerrainBuilder},
    },
    sound::source::{generic::GenericSourceBuilder, spatial::SpatialSourceBuilder},
};
use std::sync::{mpsc::Sender, Arc, RwLock};

pub struct CreateEntityMenu {
    pub menu: Handle<UiNode>,
    create_pivot: Handle<UiNode>,
    create_cube: Handle<UiNode>,
    create_cone: Handle<UiNode>,
    create_sphere: Handle<UiNode>,
    create_cylinder: Handle<UiNode>,
    create_quad: Handle<UiNode>,
    create_decal: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
    create_directional_light: Handle<UiNode>,
    create_terrain: Handle<UiNode>,
    create_camera: Handle<UiNode>,
    create_sprite: Handle<UiNode>,
    create_particle_system: Handle<UiNode>,
    create_sound_source: Handle<UiNode>,
    create_spatial_sound_source: Handle<UiNode>,
    physics_menu: PhysicsMenu,
}

impl CreateEntityMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let create_cube;
        let create_cone;
        let create_sphere;
        let create_cylinder;
        let create_quad;
        let create_point_light;
        let create_spot_light;
        let create_directional_light;
        let create_camera;
        let create_sprite;
        let create_decal;
        let create_particle_system;
        let create_terrain;
        let create_pivot;
        let create_sound_source;
        let create_spatial_sound_source;

        let physics_menu = PhysicsMenu::new(ctx);

        let menu = create_root_menu_item(
            "Create",
            vec![
                {
                    create_pivot = create_menu_item("Pivot", vec![], ctx);
                    create_pivot
                },
                create_menu_item(
                    "Mesh",
                    vec![
                        {
                            create_cube = create_menu_item("Cube", vec![], ctx);
                            create_cube
                        },
                        {
                            create_sphere = create_menu_item("Sphere", vec![], ctx);
                            create_sphere
                        },
                        {
                            create_cylinder = create_menu_item("Cylinder", vec![], ctx);
                            create_cylinder
                        },
                        {
                            create_cone = create_menu_item("Cone", vec![], ctx);
                            create_cone
                        },
                        {
                            create_quad = create_menu_item("Quad", vec![], ctx);
                            create_quad
                        },
                    ],
                    ctx,
                ),
                create_menu_item(
                    "Sound",
                    vec![
                        {
                            create_sound_source = create_menu_item("2D Source", vec![], ctx);
                            create_sound_source
                        },
                        {
                            create_spatial_sound_source =
                                create_menu_item("3D Source", vec![], ctx);
                            create_spatial_sound_source
                        },
                    ],
                    ctx,
                ),
                create_menu_item(
                    "Light",
                    vec![
                        {
                            create_directional_light =
                                create_menu_item("Directional Light", vec![], ctx);
                            create_directional_light
                        },
                        {
                            create_spot_light = create_menu_item("Spot Light", vec![], ctx);
                            create_spot_light
                        },
                        {
                            create_point_light = create_menu_item("Point Light", vec![], ctx);
                            create_point_light
                        },
                    ],
                    ctx,
                ),
                physics_menu.menu,
                {
                    create_camera = create_menu_item("Camera", vec![], ctx);
                    create_camera
                },
                {
                    create_sprite = create_menu_item("Sprite", vec![], ctx);
                    create_sprite
                },
                {
                    create_particle_system = create_menu_item("Particle System", vec![], ctx);
                    create_particle_system
                },
                {
                    create_terrain = create_menu_item("Terrain", vec![], ctx);
                    create_terrain
                },
                {
                    create_decal = create_menu_item("Decal", vec![], ctx);
                    create_decal
                },
            ],
            ctx,
        );

        Self {
            menu,
            create_cube,
            create_cone,
            create_sphere,
            create_cylinder,
            create_quad,
            create_point_light,
            create_spot_light,
            create_directional_light,
            create_camera,
            create_sprite,
            create_particle_system,
            create_pivot,
            create_terrain,
            create_sound_source,
            create_spatial_sound_source,
            create_decal,
            physics_menu,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, sender: &Sender<Message>) {
        self.physics_menu.handle_ui_message(message, sender);

        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.create_cube {
                let mut mesh = Mesh::default();
                mesh.set_name("Cube");
                mesh.add_surface(Surface::new(Arc::new(RwLock::new(SurfaceData::make_cube(
                    Matrix4::identity(),
                )))));
                let node = Node::Mesh(mesh);
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_spot_light {
                let node = SpotLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("SpotLight"),
                ))
                .with_distance(10.0)
                .with_hotspot_cone_angle(45.0f32.to_radians())
                .with_falloff_angle_delta(2.0f32.to_radians())
                .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_pivot {
                let node = BaseBuilder::new().with_name("Pivot").build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_point_light {
                let node = PointLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("PointLight"),
                ))
                .with_radius(10.0)
                .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_directional_light {
                let node = DirectionalLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("DirectionalLight"),
                ))
                .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_cone {
                let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Cone"))
                    .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                        SurfaceData::make_cone(16, 0.5, 1.0, &Matrix4::identity()),
                    )))])
                    .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(mesh)))
                    .unwrap();
            } else if message.destination() == self.create_cylinder {
                let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Cylinder"))
                    .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                        SurfaceData::make_cylinder(16, 0.5, 1.0, true, &Matrix4::identity()),
                    )))])
                    .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(mesh)))
                    .unwrap();
            } else if message.destination() == self.create_sphere {
                let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Sphere"))
                    .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                        SurfaceData::make_sphere(16, 16, 0.5, &Matrix4::identity()),
                    )))])
                    .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(mesh)))
                    .unwrap();
            } else if message.destination() == self.create_quad {
                let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Quad"))
                    .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                        SurfaceData::make_quad(&Matrix4::identity()),
                    )))])
                    .build_node();
                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(mesh)))
                    .unwrap();
            } else if message.destination() == self.create_camera {
                let node = CameraBuilder::new(BaseBuilder::new().with_name("Camera"))
                    .enabled(false)
                    .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_sprite {
                let node = SpriteBuilder::new(BaseBuilder::new().with_name("Sprite")).build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_sound_source {
                let source = GenericSourceBuilder::new()
                    .with_name("2D Source")
                    .build_source()
                    .unwrap();

                sender
                    .send(Message::do_scene_command(AddSoundSourceCommand::new(
                        source,
                    )))
                    .unwrap();
            } else if message.destination() == self.create_spatial_sound_source {
                let source = SpatialSourceBuilder::new(
                    GenericSourceBuilder::new()
                        .with_name("3D Source")
                        .build()
                        .unwrap(),
                )
                .build_source();

                sender
                    .send(Message::do_scene_command(AddSoundSourceCommand::new(
                        source,
                    )))
                    .unwrap();
            } else if message.destination() == self.create_particle_system {
                let node =
                    ParticleSystemBuilder::new(BaseBuilder::new().with_name("ParticleSystem"))
                        .with_emitters(vec![SphereEmitterBuilder::new(
                            BaseEmitterBuilder::new()
                                .with_max_particles(100)
                                .resurrect_particles(true),
                        )
                        .with_radius(1.0)
                        .build()])
                        .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_terrain {
                let node = TerrainBuilder::new(BaseBuilder::new().with_name("Terrain"))
                    .with_layers(vec![LayerDefinition {
                        material: create_terrain_layer_material(),
                        mask_property_name: "maskTexture".to_owned(),
                    }])
                    .with_height_map_resolution(4.0)
                    .build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            } else if message.destination() == self.create_decal {
                let node = DecalBuilder::new(BaseBuilder::new().with_name("Decal")).build_node();

                sender
                    .send(Message::do_scene_command(AddNodeCommand::new(node)))
                    .unwrap();
            }
        }
    }
}
