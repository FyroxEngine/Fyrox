use crate::scene::controller::SceneController;
use crate::scene::Selection;
use crate::ui_scene::UiScene;
use crate::{
    create_terrain_layer_material,
    menu::ui::UiMenu,
    menu::{
        animation::AnimationMenu, create_menu_item, create_root_menu_item, dim2::Dim2Menu,
        physics::PhysicsMenu, physics2d::Physics2dMenu,
    },
    message::MessageSender,
    scene::commands::graph::AddNodeCommand,
    Mode,
};
use fyrox::{
    core::{
        algebra::{Matrix4, Vector3},
        math::TriangleDefinition,
        pool::Handle,
    },
    gui::{
        menu::MenuItemMessage, message::MessageDirection, message::UiMessage,
        widget::WidgetMessage, BuildContext, UiNode, UserInterface,
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
            surface::{Surface, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        navmesh::NavigationalMeshBuilder,
        node::Node,
        particle_system::{
            emitter::{base::BaseEmitterBuilder, sphere::SphereEmitterBuilder},
            ParticleSystemBuilder,
        },
        pivot::PivotBuilder,
        sound::{listener::ListenerBuilder, SoundBuilder},
        sprite::SpriteBuilder,
        terrain::{Layer, TerrainBuilder},
    },
    utils::navmesh::Navmesh,
};

pub struct CreateEntityRootMenu {
    pub menu: Handle<UiNode>,
    pub sub_menus: CreateEntityMenu,
}

impl CreateEntityRootMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let (sub_menus, root_items) = CreateEntityMenu::new(ctx);

        let menu = create_root_menu_item("Create", root_items, ctx);

        Self { menu, sub_menus }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        controller: &mut dyn SceneController,
        selection: &Selection,
    ) {
        if let Some(node) = self
            .sub_menus
            .handle_ui_message(message, sender, controller, selection)
        {
            sender.do_scene_command(AddNodeCommand::new(node, Handle::NONE, true));
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            self.menu,
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}

pub struct CreateEntityMenu {
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
    create_navmesh: Handle<UiNode>,
    create_terrain: Handle<UiNode>,
    create_camera: Handle<UiNode>,
    create_sprite: Handle<UiNode>,
    create_particle_system: Handle<UiNode>,
    create_listener: Handle<UiNode>,
    create_sound_source: Handle<UiNode>,
    physics_menu: PhysicsMenu,
    physics2d_menu: Physics2dMenu,
    dim2_menu: Dim2Menu,
    animation_menu: AnimationMenu,
    ui_menu: UiMenu,
}

impl CreateEntityMenu {
    pub fn new(ctx: &mut BuildContext) -> (Self, Vec<Handle<UiNode>>) {
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
        let create_navmesh;
        let create_particle_system;
        let create_terrain;
        let create_pivot;
        let create_sound_source;
        let create_listener;
        let physics_menu = PhysicsMenu::new(ctx);
        let physics2d_menu = Physics2dMenu::new(ctx);
        let dim2_menu = Dim2Menu::new(ctx);
        let animation_menu = AnimationMenu::new(ctx);

        let ui_menu = UiMenu::new(UiMenu::default_entries(), "UI", ctx);

        let items = vec![
            ui_menu.menu,
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
                        create_sound_source = create_menu_item("Source", vec![], ctx);
                        create_sound_source
                    },
                    {
                        create_listener = create_menu_item("Listener", vec![], ctx);
                        create_listener
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
            physics2d_menu.menu,
            dim2_menu.menu,
            animation_menu.menu,
            {
                create_camera = create_menu_item("Camera", vec![], ctx);
                create_camera
            },
            {
                create_sprite = create_menu_item("Sprite (3D)", vec![], ctx);
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
            {
                create_navmesh = create_menu_item("Navmesh", vec![], ctx);
                create_navmesh
            },
        ];

        (
            Self {
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
                create_listener,
                create_navmesh,
                create_decal,
                physics_menu,
                physics2d_menu,
                dim2_menu,
                animation_menu,
                ui_menu,
            },
            items,
        )
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        controller: &mut dyn SceneController,
        selection: &Selection,
    ) -> Option<Node> {
        if let Some(ui_scene) = controller.downcast_mut::<UiScene>() {
            self.ui_menu
                .handle_ui_message(sender, message, ui_scene, selection);
        }

        self.physics_menu
            .handle_ui_message(message)
            .or_else(|| self.physics2d_menu.handle_ui_message(message))
            .or_else(|| self.dim2_menu.handle_ui_message(message))
            .or_else(|| self.animation_menu.handle_ui_message(message))
            .or_else(|| {
                if let Some(MenuItemMessage::Click) = message.data::<MenuItemMessage>() {
                    if message.destination() == self.create_cube {
                        Some(
                            MeshBuilder::new(BaseBuilder::new().with_name("Cube"))
                                .with_surfaces(vec![Surface::new(SurfaceSharedData::new(
                                    SurfaceData::make_cube(Matrix4::identity()),
                                ))])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_spot_light {
                        Some(
                            SpotLightBuilder::new(BaseLightBuilder::new(
                                BaseBuilder::new().with_name("SpotLight"),
                            ))
                            .with_distance(10.0)
                            .with_hotspot_cone_angle(45.0f32.to_radians())
                            .with_falloff_angle_delta(2.0f32.to_radians())
                            .build_node(),
                        )
                    } else if message.destination() == self.create_pivot {
                        Some(PivotBuilder::new(BaseBuilder::new().with_name("Pivot")).build_node())
                    } else if message.destination() == self.create_point_light {
                        Some(
                            PointLightBuilder::new(BaseLightBuilder::new(
                                BaseBuilder::new().with_name("PointLight"),
                            ))
                            .with_radius(10.0)
                            .build_node(),
                        )
                    } else if message.destination() == self.create_directional_light {
                        Some(
                            DirectionalLightBuilder::new(BaseLightBuilder::new(
                                BaseBuilder::new().with_name("DirectionalLight"),
                            ))
                            .build_node(),
                        )
                    } else if message.destination() == self.create_cone {
                        Some(
                            MeshBuilder::new(BaseBuilder::new().with_name("Cone"))
                                .with_surfaces(vec![Surface::new(SurfaceSharedData::new(
                                    SurfaceData::make_cone(16, 0.5, 1.0, &Matrix4::identity()),
                                ))])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_cylinder {
                        Some(
                            MeshBuilder::new(BaseBuilder::new().with_name("Cylinder"))
                                .with_surfaces(vec![Surface::new(SurfaceSharedData::new(
                                    SurfaceData::make_cylinder(
                                        16,
                                        0.5,
                                        1.0,
                                        true,
                                        &Matrix4::identity(),
                                    ),
                                ))])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_sphere {
                        Some(
                            MeshBuilder::new(BaseBuilder::new().with_name("Sphere"))
                                .with_surfaces(vec![Surface::new(SurfaceSharedData::new(
                                    SurfaceData::make_sphere(16, 16, 0.5, &Matrix4::identity()),
                                ))])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_quad {
                        Some(
                            MeshBuilder::new(BaseBuilder::new().with_name("Quad"))
                                .with_surfaces(vec![Surface::new(SurfaceSharedData::new(
                                    SurfaceData::make_quad(&Matrix4::identity()),
                                ))])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_camera {
                        Some(
                            CameraBuilder::new(BaseBuilder::new().with_name("Camera")).build_node(),
                        )
                    } else if message.destination() == self.create_navmesh {
                        let navmesh = Navmesh::new(
                            vec![TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])],
                            vec![
                                Vector3::new(-1.0, 0.0, 1.0),
                                Vector3::new(1.0, 0.0, 1.0),
                                Vector3::new(1.0, 0.0, -1.0),
                                Vector3::new(-1.0, 0.0, -1.0),
                            ],
                        );
                        Some(
                            NavigationalMeshBuilder::new(BaseBuilder::new().with_name("Navmesh"))
                                .with_navmesh(navmesh)
                                .build_node(),
                        )
                    } else if message.destination() == self.create_sprite {
                        Some(
                            SpriteBuilder::new(BaseBuilder::new().with_name("Sprite")).build_node(),
                        )
                    } else if message.destination() == self.create_sound_source {
                        Some(SoundBuilder::new(BaseBuilder::new().with_name("Sound")).build_node())
                    } else if message.destination() == self.create_particle_system {
                        Some(
                            ParticleSystemBuilder::new(
                                BaseBuilder::new().with_name("ParticleSystem"),
                            )
                            .with_emitters(vec![SphereEmitterBuilder::new(
                                BaseEmitterBuilder::new()
                                    .with_max_particles(100)
                                    .resurrect_particles(true),
                            )
                            .with_radius(1.0)
                            .build()])
                            .build_node(),
                        )
                    } else if message.destination() == self.create_terrain {
                        Some(
                            TerrainBuilder::new(BaseBuilder::new().with_name("Terrain"))
                                .with_layers(vec![Layer {
                                    material: create_terrain_layer_material(),
                                    ..Default::default()
                                }])
                                .build_node(),
                        )
                    } else if message.destination() == self.create_decal {
                        Some(DecalBuilder::new(BaseBuilder::new().with_name("Decal")).build_node())
                    } else if message.destination() == self.create_listener {
                        Some(
                            ListenerBuilder::new(BaseBuilder::new().with_name("Listener"))
                                .build_node(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    }
}
