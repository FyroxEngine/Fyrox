use crate::scene::commands::graph::AddNodeCommand;
use crate::scene::commands::sound::AddSoundSourceCommand;
use crate::scene::commands::{PasteCommand, SceneCommand};
use crate::settings::Settings;
use crate::{
    gui::{Ui, UiMessage, UiNode},
    make_save_file_selector, make_scene_file_filter,
    scene::{EditorScene, Selection},
    send_sync_message,
    settings::SettingsWindow,
    GameEngine, Message,
};
use rg3d::scene::terrain::{LayerDefinition, TerrainBuilder};
use rg3d::sound::source::generic::GenericSourceBuilder;
use rg3d::sound::source::spatial::SpatialSourceBuilder;
use rg3d::{
    core::{
        algebra::{Matrix4, Vector2},
        pool::Handle,
        scope_profile,
    },
    gui::{
        file_browser::FileSelectorBuilder,
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
        message::{
            FileSelectorMessage, MenuItemMessage, MessageBoxMessage, MessageDirection,
            UiMessageData, WidgetMessage, WindowMessage,
        },
        messagebox::{MessageBoxBuilder, MessageBoxButtons},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness,
    },
    scene::mesh::surface::{Surface, SurfaceData},
    scene::{
        base::BaseBuilder,
        camera::CameraBuilder,
        light::{BaseLightBuilder, DirectionalLightBuilder, PointLightBuilder, SpotLightBuilder},
        mesh::{Mesh, MeshBuilder},
        node::Node,
        particle_system::{BaseEmitterBuilder, ParticleSystemBuilder, SphereEmitterBuilder},
        sprite::SpriteBuilder,
    },
};
use std::sync::{mpsc::Sender, Arc, RwLock};

pub struct Menu {
    pub menu: Handle<UiNode>,
    new_scene: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    load: Handle<UiNode>,
    close_scene: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    copy: Handle<UiNode>,
    paste: Handle<UiNode>,
    create_pivot: Handle<UiNode>,
    create_cube: Handle<UiNode>,
    create_cone: Handle<UiNode>,
    create_sphere: Handle<UiNode>,
    create_cylinder: Handle<UiNode>,
    create_quad: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
    create_directional_light: Handle<UiNode>,
    create_terrain: Handle<UiNode>,
    exit: Handle<UiNode>,
    message_sender: Sender<Message>,
    save_file_selector: Handle<UiNode>,
    load_file_selector: Handle<UiNode>,
    create_camera: Handle<UiNode>,
    create_sprite: Handle<UiNode>,
    create_particle_system: Handle<UiNode>,
    create_sound_source: Handle<UiNode>,
    create_spatial_sound_source: Handle<UiNode>,
    sidebar: Handle<UiNode>,
    world_outliner: Handle<UiNode>,
    asset_browser: Handle<UiNode>,
    open_settings: Handle<UiNode>,
    configure: Handle<UiNode>,
    light_panel: Handle<UiNode>,
    pub settings: SettingsWindow,
    configure_message: Handle<UiNode>,
    log_panel: Handle<UiNode>,
    create: Handle<UiNode>,
    edit: Handle<UiNode>,
}

pub struct MenuContext<'a, 'b> {
    pub engine: &'a mut GameEngine,
    pub editor_scene: Option<&'b mut EditorScene>,
    pub sidebar_window: Handle<UiNode>,
    pub world_outliner_window: Handle<UiNode>,
    pub asset_window: Handle<UiNode>,
    pub configurator_window: Handle<UiNode>,
    pub light_panel: Handle<UiNode>,
    pub log_panel: Handle<UiNode>,
    pub settings: &'b mut Settings,
}

fn switch_window_state(window: Handle<UiNode>, ui: &mut Ui, center: bool) {
    let current_state = ui.node(window).visibility();
    ui.send_message(if current_state {
        WindowMessage::close(window, MessageDirection::ToWidget)
    } else {
        WindowMessage::open(window, MessageDirection::ToWidget, center)
    })
}

impl Menu {
    pub fn new(
        engine: &mut GameEngine,
        message_sender: Sender<Message>,
        settings: &Settings,
    ) -> Self {
        let min_size = Vector2::new(120.0, 22.0);
        let new_scene;
        let save;
        let save_as;
        let close_scene;
        let load;
        let redo;
        let undo;
        let copy;
        let paste;
        let create_cube;
        let create_cone;
        let create_sphere;
        let create_cylinder;
        let create_quad;
        let create_point_light;
        let create_spot_light;
        let create_directional_light;
        let exit;
        let create_camera;
        let create_sprite;
        let create_particle_system;
        let create_terrain;
        let sidebar;
        let asset_browser;
        let world_outliner;
        let open_settings;
        let configure;
        let light_panel;
        let log_panel;
        let create_pivot;
        let create_sound_source;
        let create_spatial_sound_source;
        let ctx = &mut engine.user_interface.build_ctx();
        let configure_message = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(250.0).with_height(150.0))
                .open(false)
                .with_title(WindowTitle::Text("Warning".to_owned())),
        )
        .with_text("Cannot reconfigure editor while scene is open! Close scene first and retry.")
        .with_buttons(MessageBoxButtons::Ok)
        .build(ctx);

        let create = MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
            .with_content(MenuItemContent::text_with_shortcut("Create", ""))
            .with_items(vec![
                {
                    create_pivot =
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Pivot"))
                            .build(ctx);
                    create_pivot
                },
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                    .with_content(MenuItemContent::text("Mesh"))
                    .with_items(vec![
                        {
                            create_cube =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Cube"))
                                    .build(ctx);
                            create_cube
                        },
                        {
                            create_sphere =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Sphere"))
                                    .build(ctx);
                            create_sphere
                        },
                        {
                            create_cylinder =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Cylinder"))
                                    .build(ctx);
                            create_cylinder
                        },
                        {
                            create_cone =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Cone"))
                                    .build(ctx);
                            create_cone
                        },
                        {
                            create_quad =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Quad"))
                                    .build(ctx);
                            create_quad
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                    .with_content(MenuItemContent::text("Sound"))
                    .with_items(vec![
                        {
                            create_sound_source =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("2D Source"))
                                    .build(ctx);
                            create_sound_source
                        },
                        {
                            create_spatial_sound_source =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("3D Source"))
                                    .build(ctx);
                            create_spatial_sound_source
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                    .with_content(MenuItemContent::text("Light"))
                    .with_items(vec![
                        {
                            create_directional_light =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Directional Light"))
                                    .build(ctx);
                            create_directional_light
                        },
                        {
                            create_spot_light =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Spot Light"))
                                    .build(ctx);
                            create_spot_light
                        },
                        {
                            create_point_light =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Point Light"))
                                    .build(ctx);
                            create_point_light
                        },
                    ])
                    .build(ctx),
                {
                    create_camera =
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Camera"))
                            .build(ctx);
                    create_camera
                },
                {
                    create_sprite =
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Sprite"))
                            .build(ctx);
                    create_sprite
                },
                {
                    create_particle_system =
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Particle System"))
                            .build(ctx);
                    create_particle_system
                },
                {
                    create_terrain =
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Terrain"))
                            .build(ctx);
                    create_terrain
                },
            ])
            .build(ctx);

        let edit = MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
            .with_content(MenuItemContent::text_with_shortcut("Edit", ""))
            .with_items(vec![
                {
                    undo = MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                        .with_content(MenuItemContent::text_with_shortcut("Undo", "Ctrl+Z"))
                        .build(ctx);
                    undo
                },
                {
                    redo = MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                        .with_content(MenuItemContent::text_with_shortcut("Redo", "Ctrl+Y"))
                        .build(ctx);
                    redo
                },
                {
                    copy = MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                        .with_content(MenuItemContent::text_with_shortcut("Copy", "Ctrl+C"))
                        .build(ctx);
                    copy
                },
                {
                    paste = MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                        .with_content(MenuItemContent::text_with_shortcut("Paste", "Ctrl+V"))
                        .build(ctx);
                    paste
                },
            ])
            .build(ctx);

        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
                    .with_content(MenuItemContent::text("File"))
                    .with_items(vec![
                        {
                            new_scene =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "New Scene",
                                        "Ctrl+N",
                                    ))
                                    .build(ctx);
                            new_scene
                        },
                        {
                            save =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Save Scene",
                                        "Ctrl+S",
                                    ))
                                    .build(ctx);
                            save
                        },
                        {
                            save_as =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Save Scene As...",
                                        "Ctrl+Shift+S",
                                    ))
                                    .build(ctx);
                            save_as
                        },
                        {
                            load =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Load Scene...",
                                        "Ctrl+L",
                                    ))
                                    .build(ctx);
                            load
                        },
                        {
                            close_scene =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Close Scene",
                                        "Ctrl+Q",
                                    ))
                                    .build(ctx);
                            close_scene
                        },
                        {
                            open_settings =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Settings..."))
                                    .build(ctx);
                            open_settings
                        },
                        {
                            configure =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Configure..."))
                                    .build(ctx);
                            configure
                        },
                        {
                            exit =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Exit", "Alt+F4",
                                    ))
                                    .build(ctx);
                            exit
                        },
                    ])
                    .build(ctx),
                edit,
                create,
                MenuItemBuilder::new(WidgetBuilder::new().with_margin(Thickness::right(10.0)))
                    .with_content(MenuItemContent::text_with_shortcut("View", ""))
                    .with_items(vec![
                        {
                            sidebar =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Sidebar"))
                                    .build(ctx);
                            sidebar
                        },
                        {
                            asset_browser =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Asset Browser"))
                                    .build(ctx);
                            asset_browser
                        },
                        {
                            world_outliner =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("World Outliner"))
                                    .build(ctx);
                            world_outliner
                        },
                        {
                            light_panel =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Light Panel"))
                                    .build(ctx);
                            light_panel
                        },
                        {
                            log_panel =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text("Log Panel"))
                                    .build(ctx);
                            log_panel
                        },
                    ])
                    .build(ctx),
            ])
            .build(ctx);

        let save_file_selector = make_save_file_selector(ctx);

        let load_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select a Scene To Load".into())),
        )
        .with_filter(make_scene_file_filter())
        .build(ctx);

        Self {
            menu,
            new_scene,
            save,
            save_as,
            close_scene,
            load,
            undo,
            redo,
            create_cube,
            create_cone,
            create_sphere,
            create_cylinder,
            create_quad,
            create_point_light,
            create_spot_light,
            create_directional_light,
            exit,
            settings: SettingsWindow::new(engine, message_sender.clone(), settings),
            message_sender,
            save_file_selector,
            load_file_selector,
            create_camera,
            create_sprite,
            create_particle_system,
            sidebar,
            world_outliner,
            asset_browser,
            open_settings,
            configure,
            configure_message,
            light_panel,
            copy,
            paste,
            log_panel,
            create_pivot,
            create_terrain,
            create_sound_source,
            create_spatial_sound_source,
            create,
            edit,
        }
    }

    pub fn open_load_file_selector(&self, ui: &mut Ui) {
        ui.send_message(WindowMessage::open_modal(
            self.load_file_selector,
            MessageDirection::ToWidget,
            true,
        ));
        ui.send_message(FileSelectorMessage::root(
            self.load_file_selector,
            MessageDirection::ToWidget,
            Some(std::env::current_dir().unwrap()),
        ));
    }

    pub fn sync_to_model(&mut self, editor_scene: Option<&EditorScene>, ui: &mut Ui) {
        scope_profile!();

        for &widget in [
            self.close_scene,
            self.save,
            self.save_as,
            self.create,
            self.edit,
        ]
        .iter()
        {
            send_sync_message(
                ui,
                WidgetMessage::enabled(widget, MessageDirection::ToWidget, editor_scene.is_some()),
            );
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ctx: MenuContext) {
        scope_profile!();

        if let Some(scene) = ctx.editor_scene.as_ref() {
            self.settings
                .handle_message(message, scene, ctx.engine, ctx.settings);
        }

        match message.data() {
            UiMessageData::FileSelector(FileSelectorMessage::Commit(path)) => {
                if message.destination() == self.save_file_selector {
                    self.message_sender
                        .send(Message::SaveScene(path.to_owned()))
                        .unwrap();
                } else if message.destination() == self.load_file_selector {
                    self.message_sender
                        .send(Message::LoadScene(path.to_owned()))
                        .unwrap();
                }
            }
            UiMessageData::MenuItem(MenuItemMessage::Click) => {
                if message.destination() == self.create_cube {
                    let mut mesh = Mesh::default();
                    mesh.set_name("Cube");
                    mesh.add_surface(Surface::new(Arc::new(RwLock::new(SurfaceData::make_cube(
                        Matrix4::identity(),
                    )))));
                    let node = Node::Mesh(mesh);
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_spot_light {
                    let node = SpotLightBuilder::new(BaseLightBuilder::new(
                        BaseBuilder::new().with_name("SpotLight"),
                    ))
                    .with_distance(10.0)
                    .with_hotspot_cone_angle(45.0f32.to_radians())
                    .with_falloff_angle_delta(2.0f32.to_radians())
                    .build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_pivot {
                    let node = BaseBuilder::new().with_name("Pivot").build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_point_light {
                    let node = PointLightBuilder::new(BaseLightBuilder::new(
                        BaseBuilder::new().with_name("PointLight"),
                    ))
                    .with_radius(10.0)
                    .build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_directional_light {
                    let node = DirectionalLightBuilder::new(BaseLightBuilder::new(
                        BaseBuilder::new().with_name("DirectionalLight"),
                    ))
                    .build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_cone {
                    let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Cone"))
                        .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                            SurfaceData::make_cone(16, 0.5, 1.0, &Matrix4::identity()),
                        )))])
                        .build_node();
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(mesh),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_cylinder {
                    let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Cylinder"))
                        .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                            SurfaceData::make_cylinder(16, 0.5, 1.0, true, &Matrix4::identity()),
                        )))])
                        .build_node();
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(mesh),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_sphere {
                    let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Sphere"))
                        .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                            SurfaceData::make_sphere(16, 16, 0.5, &Matrix4::identity()),
                        )))])
                        .build_node();
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(mesh),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_quad {
                    let mesh = MeshBuilder::new(BaseBuilder::new().with_name("Quad"))
                        .with_surfaces(vec![Surface::new(Arc::new(RwLock::new(
                            SurfaceData::make_quad(&Matrix4::identity()),
                        )))])
                        .build_node();
                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(mesh),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_camera {
                    let node = CameraBuilder::new(BaseBuilder::new().with_name("Camera"))
                        .enabled(false)
                        .build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_sprite {
                    let node =
                        SpriteBuilder::new(BaseBuilder::new().with_name("Sprite")).build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_sound_source {
                    let source = GenericSourceBuilder::new()
                        .with_name("2D Source")
                        .build_source()
                        .unwrap();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddSoundSource(
                            AddSoundSourceCommand::new(source),
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

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddSoundSource(
                            AddSoundSourceCommand::new(source),
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

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.create_terrain {
                    let node = TerrainBuilder::new(BaseBuilder::new().with_name("Terrain"))
                        .with_layers(vec![LayerDefinition {
                            diffuse_texture: None,
                            normal_texture: None,
                            specular_texture: None,
                            roughness_texture: None,
                            height_texture: None,
                            tile_factor: Vector2::new(10.0, 10.0),
                        }])
                        .build_node();

                    self.message_sender
                        .send(Message::DoSceneCommand(SceneCommand::AddNode(
                            AddNodeCommand::new(node),
                        )))
                        .unwrap();
                } else if message.destination() == self.save {
                    if let Some(scene_path) =
                        ctx.editor_scene.as_ref().map(|s| s.path.as_ref()).flatten()
                    {
                        self.message_sender
                            .send(Message::SaveScene(scene_path.clone()))
                            .unwrap();
                    } else {
                        // If scene wasn't saved yet - open Save As window.
                        ctx.engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(
                                self.save_file_selector,
                                MessageDirection::ToWidget,
                                true,
                            ));
                        ctx.engine
                            .user_interface
                            .send_message(FileSelectorMessage::path(
                                self.save_file_selector,
                                MessageDirection::ToWidget,
                                std::env::current_dir().unwrap(),
                            ));
                    }
                } else if message.destination() == self.save_as {
                    ctx.engine
                        .user_interface
                        .send_message(WindowMessage::open_modal(
                            self.save_file_selector,
                            MessageDirection::ToWidget,
                            true,
                        ));
                    ctx.engine
                        .user_interface
                        .send_message(FileSelectorMessage::path(
                            self.save_file_selector,
                            MessageDirection::ToWidget,
                            std::env::current_dir().unwrap(),
                        ));
                } else if message.destination() == self.load {
                    self.open_load_file_selector(&mut ctx.engine.user_interface);
                } else if message.destination() == self.close_scene {
                    self.message_sender.send(Message::CloseScene).unwrap();
                } else if message.destination() == self.copy {
                    if let Some(editor_scene) = ctx.editor_scene {
                        if let Selection::Graph(selection) = &editor_scene.selection {
                            editor_scene.clipboard.fill_from_selection(
                                selection,
                                editor_scene.scene,
                                &editor_scene.physics,
                                ctx.engine,
                            );
                        }
                    }
                } else if message.destination() == self.paste {
                    if let Some(editor_scene) = ctx.editor_scene {
                        if !editor_scene.clipboard.is_empty() {
                            self.message_sender
                                .send(Message::DoSceneCommand(SceneCommand::Paste(
                                    PasteCommand::new(),
                                )))
                                .unwrap();
                        }
                    }
                } else if message.destination() == self.undo {
                    self.message_sender.send(Message::UndoSceneCommand).unwrap();
                } else if message.destination() == self.redo {
                    self.message_sender.send(Message::RedoSceneCommand).unwrap();
                } else if message.destination() == self.exit {
                    self.message_sender
                        .send(Message::Exit { force: false })
                        .unwrap();
                } else if message.destination() == self.new_scene {
                    self.message_sender.send(Message::NewScene).unwrap();
                } else if message.destination() == self.asset_browser {
                    switch_window_state(ctx.asset_window, &mut ctx.engine.user_interface, false);
                } else if message.destination() == self.light_panel {
                    switch_window_state(ctx.light_panel, &mut ctx.engine.user_interface, true);
                } else if message.destination() == self.world_outliner {
                    switch_window_state(
                        ctx.world_outliner_window,
                        &mut ctx.engine.user_interface,
                        false,
                    );
                } else if message.destination() == self.sidebar {
                    switch_window_state(ctx.sidebar_window, &mut ctx.engine.user_interface, false);
                } else if message.destination() == self.log_panel {
                    switch_window_state(ctx.log_panel, &mut ctx.engine.user_interface, false);
                } else if message.destination() == self.open_settings {
                    self.settings
                        .open(&ctx.engine.user_interface, ctx.settings, None);
                } else if message.destination() == self.configure {
                    if ctx.editor_scene.is_none() {
                        ctx.engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(
                                ctx.configurator_window,
                                MessageDirection::ToWidget,
                                true,
                            ));
                    } else {
                        ctx.engine
                            .user_interface
                            .send_message(MessageBoxMessage::open(
                                self.configure_message,
                                MessageDirection::ToWidget,
                                None,
                                None,
                            ));
                    }
                }
            }
            _ => (),
        }
    }
}
