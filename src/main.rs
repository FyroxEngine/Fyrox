#![allow(irrefutable_let_patterns)]

extern crate rg3d;

pub mod asset;
pub mod camera;
pub mod command;
pub mod gui;
pub mod interaction;
pub mod scene;
pub mod sidebar;
pub mod world_outliner;

use crate::{
    asset::AssetBrowser,
    camera::CameraController,
    command::CommandStack,
    gui::{BuildContext, EditorUiMessage, EditorUiNode, UiMessage, UiNode},
    interaction::{
        InteractionMode, InteractionModeKind, MoveInteractionMode, RotateInteractionMode,
        ScaleInteractionMode, SelectInteractionMode,
    },
    scene::{
        AddNodeCommand, ChangeSelectionCommand, DeleteNodeCommand, EditorScene, SceneCommand,
        SceneContext, Selection,
    },
    sidebar::SideBar,
    world_outliner::WorldOutliner,
};
use rg3d::{
    core::{
        color::Color,
        math::{vec2::Vec2, Rect},
        pool::Handle,
        visitor::{Visit, Visitor},
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        canvas::CanvasBuilder,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        file_browser::FileSelectorBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
        message::{
            ButtonMessage, FileSelectorMessage, ImageMessage, KeyCode, MenuItemMessage,
            MouseButton, UiMessageData, WidgetMessage, WindowMessage,
        },
        messagebox::{MessageBoxBuilder, MessageBoxButtons},
        stack_panel::StackPanelBuilder,
        ttf::Font,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness,
    },
    renderer::surface::{Surface, SurfaceSharedData},
    resource::texture::TextureKind,
    scene::{
        base::BaseBuilder,
        light::{LightBuilder, LightKind, PointLight, SpotLight},
        mesh::Mesh,
        node::Node,
        Scene,
    },
    utils::{into_any_arc, translate_event},
};
use std::{
    cell::RefCell,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    time::Instant,
};

type GameEngine = rg3d::engine::Engine<EditorUiMessage, EditorUiNode>;

pub struct ScenePreview {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    last_mouse_pos: Option<Vec2>,
    click_mouse_pos: Option<Vec2>,
    selection_frame: Handle<UiNode>,
    // Side bar stuff
    select_mode: Handle<UiNode>,
    move_mode: Handle<UiNode>,
    rotate_mode: Handle<UiNode>,
    scale_mode: Handle<UiNode>,
    sender: Sender<Message>,
}

impl ScenePreview {
    pub fn new(
        engine: &mut GameEngine,
        editor_scene: &EditorScene,
        sender: Sender<Message>,
    ) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

        let frame_texture = engine.scenes[editor_scene.scene]
            .render_target
            .clone()
            .unwrap();

        let frame;
        let select_mode;
        let move_mode;
        let rotate_mode;
        let scale_mode;
        let selection_frame;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            frame = ImageBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                                .with_flip(true)
                                .with_texture(frame_texture)
                                .build(ctx);
                            frame
                        })
                        .with_child(
                            CanvasBuilder::new(WidgetBuilder::new().on_column(1).with_child({
                                selection_frame = BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .with_visibility(false)
                                        .with_background(Brush::Solid(Color::from_rgba(
                                            255, 255, 255, 40,
                                        )))
                                        .with_foreground(Brush::Solid(Color::opaque(0, 255, 0))),
                                )
                                .with_stroke_thickness(Thickness::uniform(1.0))
                                .build(ctx);
                                selection_frame
                            }))
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(0)
                                    .on_column(0)
                                    .with_child({
                                        select_mode = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(into_any_arc(
                                                engine
                                                    .resource_manager
                                                    .lock()
                                                    .unwrap()
                                                    .request_texture(
                                                        "resources/select.png",
                                                        TextureKind::RGBA8,
                                                    ),
                                            ))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        select_mode
                                    })
                                    .with_child({
                                        move_mode = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(into_any_arc(
                                                engine
                                                    .resource_manager
                                                    .lock()
                                                    .unwrap()
                                                    .request_texture(
                                                        "resources/move_arrow.png",
                                                        TextureKind::RGBA8,
                                                    ),
                                            ))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        move_mode
                                    })
                                    .with_child({
                                        rotate_mode = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(into_any_arc(
                                                engine
                                                    .resource_manager
                                                    .lock()
                                                    .unwrap()
                                                    .request_texture(
                                                        "resources/rotate_arrow.png",
                                                        TextureKind::RGBA8,
                                                    ),
                                            ))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        rotate_mode
                                    })
                                    .with_child({
                                        scale_mode = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(into_any_arc(
                                                engine
                                                    .resource_manager
                                                    .lock()
                                                    .unwrap()
                                                    .request_texture(
                                                        "resources/scale_arrow.png",
                                                        TextureKind::RGBA8,
                                                    ),
                                            ))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        scale_mode
                                    }),
                            )
                            .build(ctx),
                        ),
                )
                .add_row(Row::stretch())
                .add_column(Column::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Scene Preview"))
            .build(ctx);

        Self {
            sender,
            window,
            frame,
            last_mouse_pos: None,
            move_mode,
            rotate_mode,
            scale_mode,
            selection_frame,
            select_mode,
            click_mouse_pos: None,
        }
    }
}

impl ScenePreview {
    fn handle_message(&mut self, message: &UiMessage) {
        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.scale_mode {
                        self.sender
                            .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                            .unwrap();
                    } else if message.destination == self.rotate_mode {
                        self.sender
                            .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                            .unwrap();
                    } else if message.destination == self.move_mode {
                        self.sender
                            .send(Message::SetInteractionMode(InteractionModeKind::Move))
                            .unwrap();
                    } else if message.destination == self.select_mode {
                        self.sender
                            .send(Message::SetInteractionMode(InteractionModeKind::Select))
                            .unwrap();
                    }
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    SetSelection(Selection),
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    SetInteractionMode(InteractionModeKind),
    Log(String),
    Exit,
}

struct Editor {
    sidebar: SideBar,
    camera_controller: CameraController,
    scene: EditorScene,
    command_stack: CommandStack<SceneCommand>,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    interaction_modes: Vec<Box<dyn InteractionMode>>,
    current_interaction_mode: Option<InteractionModeKind>,
    world_outliner: WorldOutliner,
    root_grid: Handle<UiNode>,
    preview: ScenePreview,
    asset_browser: AssetBrowser,
    menu: Menu,
    exit: bool,
}

struct Menu {
    menu: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    load: Handle<UiNode>,
    close_scene: Handle<UiNode>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    create_cube: Handle<UiNode>,
    create_cone: Handle<UiNode>,
    create_sphere: Handle<UiNode>,
    create_cylinder: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
    exit: Handle<UiNode>,
    message_sender: Sender<Message>,
    save_file_selector: Handle<UiNode>,
    load_file_selector: Handle<UiNode>,
}

impl Menu {
    pub fn new(ctx: &mut BuildContext, message_sender: Sender<Message>) -> Self {
        let min_size = Vec2::new(120.0, 20.0);
        let min_size_menu = Vec2::new(40.0, 20.0);
        let save;
        let save_as;
        let close_scene;
        let load;
        let redo;
        let undo;
        let create_cube;
        let create_cone;
        let create_sphere;
        let create_cylinder;
        let create_point_light;
        let create_spot_light;
        let exit;
        let menu = MenuBuilder::new(WidgetBuilder::new().on_row(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text("File"))
                    .with_items(vec![
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
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text_with_shortcut("Edit", ""))
                    .with_items(vec![
                        {
                            undo =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Undo", "Ctrl+Z",
                                    ))
                                    .build(ctx);
                            undo
                        },
                        {
                            redo =
                                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                                    .with_content(MenuItemContent::text_with_shortcut(
                                        "Redo", "Ctrl+Y",
                                    ))
                                    .build(ctx);
                            redo
                        },
                    ])
                    .build(ctx),
                MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text_with_shortcut("Create", ""))
                    .with_items(vec![
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Mesh"))
                            .with_items(vec![
                                {
                                    create_cube = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Cube"))
                                    .build(ctx);
                                    create_cube
                                },
                                {
                                    create_sphere = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Sphere"))
                                    .build(ctx);
                                    create_sphere
                                },
                                {
                                    create_cylinder = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Cylinder"))
                                    .build(ctx);
                                    create_cylinder
                                },
                                {
                                    create_cone = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Cone"))
                                    .build(ctx);
                                    create_cone
                                },
                            ])
                            .build(ctx),
                        MenuItemBuilder::new(WidgetBuilder::new().with_min_size(min_size))
                            .with_content(MenuItemContent::text("Light"))
                            .with_items(vec![
                                {
                                    create_spot_light = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Spot Light"))
                                    .build(ctx);
                                    create_spot_light
                                },
                                {
                                    create_point_light = MenuItemBuilder::new(
                                        WidgetBuilder::new().with_min_size(min_size),
                                    )
                                    .with_content(MenuItemContent::text("Point Light"))
                                    .build(ctx);
                                    create_point_light
                                },
                            ])
                            .build(ctx),
                    ])
                    .build(ctx),
            ])
            .build(ctx);

        let filter = Rc::new(RefCell::new(|p: &Path| {
            if let Some(ext) = p.extension() {
                ext.to_string_lossy().as_ref() == "rgs"
            } else {
                p.is_dir()
            }
        }));

        let save_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .with_title(WindowTitle::Text("Save Scene As".into()))
                .open(false),
        )
        .with_path("./")
        .with_filter(filter.clone())
        .build(ctx);

        let load_file_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .with_title(WindowTitle::Text("Select a Scene to Load".into()))
                .open(false),
        )
        .with_path("./")
        .with_filter(filter)
        .build(ctx);

        Self {
            menu,
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
            create_point_light,
            create_spot_light,
            exit,
            message_sender,
            save_file_selector,
            load_file_selector,
        }
    }

    fn handle_message(
        &mut self,
        engine: &mut GameEngine,
        editor_scene: &EditorScene,
        message: &UiMessage,
    ) {
        match &message.data {
            UiMessageData::FileSelector(msg) => match msg {
                FileSelectorMessage::Commit(path) => {
                    if message.destination == self.save_file_selector {
                        self.message_sender
                            .send(Message::SaveScene(path.to_owned()))
                            .unwrap();
                    } else {
                        self.message_sender
                            .send(Message::LoadScene(path.to_owned()))
                            .unwrap();
                    }
                }
                _ => (),
            },
            UiMessageData::MenuItem(msg) => {
                if let MenuItemMessage::Click = msg {
                    if message.destination == self.create_cube {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cube");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(
                            SurfaceSharedData::make_cube(Default::default()),
                        ))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.create_spot_light {
                        let kind = LightKind::Spot(SpotLight::new(
                            10.0,
                            45.0f32.to_radians(),
                            2.0f32.to_radians(),
                        ));
                        let mut light = LightBuilder::new(kind, BaseBuilder::new()).build();
                        light.set_name("SpotLight");
                        let node = Node::Light(light);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.create_point_light {
                        let kind = LightKind::Point(PointLight::new(10.0));
                        let mut light = LightBuilder::new(kind, BaseBuilder::new()).build();
                        light.set_name("PointLight");
                        let node = Node::Light(light);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.create_cone {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cone");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(
                            SurfaceSharedData::make_cone(16, 1.0, 1.0, Default::default()),
                        ))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.create_cylinder {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cylinder");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(
                            SurfaceSharedData::make_cylinder(
                                16,
                                1.0,
                                1.0,
                                true,
                                Default::default(),
                            ),
                        ))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.create_sphere {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Sphere");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(
                            SurfaceSharedData::make_sphere(16, 16, 1.0),
                        ))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::DoSceneCommand(SceneCommand::AddNode(
                                AddNodeCommand::new(node),
                            )))
                            .unwrap();
                    } else if message.destination == self.save {
                        if let Some(scene_path) = editor_scene.path.as_ref() {
                            self.message_sender
                                .send(Message::SaveScene(scene_path.clone()))
                                .unwrap();
                        } else {
                            // If scene wasn't saved yet - open Save As window.
                            engine
                                .user_interface
                                .send_message(WindowMessage::open_modal(self.save_file_selector));
                        }
                    } else if message.destination == self.save_as {
                        engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(self.save_file_selector));
                    } else if message.destination == self.load {
                        engine
                            .user_interface
                            .send_message(WindowMessage::open_modal(self.load_file_selector));
                    } else if message.destination == self.undo {
                        self.message_sender.send(Message::UndoSceneCommand).unwrap();
                    } else if message.destination == self.redo {
                        self.message_sender.send(Message::RedoSceneCommand).unwrap();
                    } else if message.destination == self.exit {
                        self.message_sender.send(Message::Exit).unwrap();
                    }
                }
            }
            _ => (),
        }
    }
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        *rg3d::gui::DEFAULT_FONT.lock().unwrap() =
            Font::from_file("resources/arial.ttf", 14.0, Font::default_char_set()).unwrap();

        let mut scene = Scene::new();
        scene.render_target = Some(Default::default());

        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        let editor_scene = EditorScene {
            path: None,
            root,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
        };

        let preview = ScenePreview::new(engine, &editor_scene, message_sender.clone());
        let asset_browser = AssetBrowser::new(engine);

        let ctx = &mut engine.user_interface.build_ctx();

        let node_editor = SideBar::new(ctx, message_sender.clone());
        let world_outliner = WorldOutliner::new(ctx, message_sender.clone());
        let menu = Menu::new(ctx, message_sender.clone());

        let root_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(engine.renderer.get_frame_size().0 as f32)
                .with_height(engine.renderer.get_frame_size().1 as f32)
                .with_child(menu.menu)
                .with_child(
                    DockingManagerBuilder::new(WidgetBuilder::new().on_row(1).with_child({
                        TileBuilder::new(WidgetBuilder::new())
                            .with_content(TileContent::VerticalTiles {
                                splitter: 0.75,
                                tiles: [
                                    TileBuilder::new(WidgetBuilder::new())
                                        .with_content(TileContent::HorizontalTiles {
                                            splitter: 0.25,
                                            tiles: [
                                                TileBuilder::new(WidgetBuilder::new())
                                                    .with_content(TileContent::Window(
                                                        world_outliner.window,
                                                    ))
                                                    .build(ctx),
                                                TileBuilder::new(WidgetBuilder::new())
                                                    .with_content(TileContent::HorizontalTiles {
                                                        splitter: 0.66,
                                                        tiles: [
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    preview.window,
                                                                ))
                                                                .build(ctx),
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    node_editor.window,
                                                                ))
                                                                .build(ctx),
                                                        ],
                                                    })
                                                    .build(ctx),
                                            ],
                                        })
                                        .build(ctx),
                                    TileBuilder::new(WidgetBuilder::new())
                                        .with_content(TileContent::Window(asset_browser.window))
                                        .build(ctx),
                                ],
                            })
                            .build(ctx)
                    }))
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let interaction_modes: Vec<Box<dyn InteractionMode>> = vec![
            Box::new(SelectInteractionMode::new(
                preview.frame,
                preview.selection_frame,
                message_sender.clone(),
            )),
            Box::new(MoveInteractionMode::new(
                &editor_scene,
                engine,
                message_sender.clone(),
            )),
            Box::new(ScaleInteractionMode::new(
                &editor_scene,
                engine,
                message_sender.clone(),
            )),
            Box::new(RotateInteractionMode::new(
                &editor_scene,
                engine,
                message_sender.clone(),
            )),
        ];

        let mut editor = Self {
            sidebar: node_editor,
            preview,
            camera_controller: CameraController::new(&editor_scene, engine),
            scene: editor_scene,
            command_stack: CommandStack::new(),
            message_sender,
            message_receiver,
            interaction_modes,
            current_interaction_mode: None,
            world_outliner,
            root_grid,
            menu,
            exit: false,
            asset_browser,
        };

        editor.set_interaction_mode(Some(InteractionModeKind::Move), engine);

        editor
    }

    fn set_scene(&mut self, engine: &mut GameEngine, mut scene: Scene, path: PathBuf) {
        engine.scenes.remove(self.scene.scene);

        scene.render_target = Some(Default::default());
        engine.user_interface.send_message(ImageMessage::texture(
            self.preview.frame,
            scene.render_target.clone().unwrap(),
        ));

        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        self.scene = EditorScene {
            path: Some(path),
            root,
            scene: engine.scenes.add(scene),
            selection: Default::default(),
        };

        self.interaction_modes = vec![
            Box::new(SelectInteractionMode::new(
                self.preview.frame,
                self.preview.selection_frame,
                self.message_sender.clone(),
            )),
            Box::new(MoveInteractionMode::new(
                &self.scene,
                engine,
                self.message_sender.clone(),
            )),
            Box::new(ScaleInteractionMode::new(
                &self.scene,
                engine,
                self.message_sender.clone(),
            )),
            Box::new(RotateInteractionMode::new(
                &self.scene,
                engine,
                self.message_sender.clone(),
            )),
        ];

        self.world_outliner.clear(&mut engine.user_interface);
        self.camera_controller = CameraController::new(&self.scene, engine);
        self.command_stack = CommandStack::new();

        self.set_interaction_mode(Some(InteractionModeKind::Move), engine);
        self.sync_to_model(engine);
    }

    fn set_interaction_mode(&mut self, mode: Option<InteractionModeKind>, engine: &mut GameEngine) {
        if self.current_interaction_mode != mode {
            // Deactivate current first.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode as usize].deactivate(&self.scene, engine);
            }

            self.current_interaction_mode = mode;
        }
    }

    fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        self.sidebar.handle_message(message, &self.scene, engine);
        self.menu.handle_message(engine, &self.scene, message);
        self.asset_browser.handle_ui_message(message, engine);

        let ui = &mut engine.user_interface;
        self.world_outliner
            .handle_ui_message(message, ui, &self.scene.selection);
        self.preview.handle_message(message);

        if message.destination == self.preview.frame {
            match &message.data {
                UiMessageData::Widget(msg) => match msg {
                    &WidgetMessage::MouseDown { button, pos, .. } => {
                        ui.capture_mouse(self.preview.frame);
                        if button == MouseButton::Left {
                            if let Some(current_im) = self.current_interaction_mode {
                                let screen_bounds = ui.node(self.preview.frame).screen_bounds();
                                let rel_pos =
                                    Vec2::new(pos.x - screen_bounds.x, pos.y - screen_bounds.y);

                                self.preview.click_mouse_pos = Some(rel_pos);

                                self.interaction_modes[current_im as usize]
                                    .on_left_mouse_button_down(
                                        &self.scene,
                                        &mut self.camera_controller,
                                        engine,
                                        rel_pos,
                                    );
                            }
                        }
                        self.camera_controller.on_mouse_button_down(button);
                    }
                    &WidgetMessage::MouseUp { button, pos, .. } => {
                        ui.release_mouse_capture();

                        if button == MouseButton::Left {
                            self.preview.click_mouse_pos = None;
                            if let Some(current_im) = self.current_interaction_mode {
                                let screen_bounds = ui.node(self.preview.frame).screen_bounds();
                                let rel_pos =
                                    Vec2::new(pos.x - screen_bounds.x, pos.y - screen_bounds.y);
                                self.interaction_modes[current_im as usize]
                                    .on_left_mouse_button_up(
                                        &self.scene,
                                        &mut self.camera_controller,
                                        engine,
                                        rel_pos,
                                    );
                            }
                        }
                        self.camera_controller.on_mouse_button_up(button);
                    }
                    &WidgetMessage::MouseWheel { amount, .. } => {
                        self.camera_controller
                            .on_mouse_wheel(amount, &self.scene, engine);
                    }
                    &WidgetMessage::MouseMove { pos, .. } => {
                        let last_pos = *self.preview.last_mouse_pos.get_or_insert(pos);
                        let mouse_offset = pos - last_pos;
                        self.camera_controller.on_mouse_move(mouse_offset);
                        let screen_bounds = ui.node(self.preview.frame).screen_bounds();
                        let rel_pos = Vec2::new(pos.x - screen_bounds.x, pos.y - screen_bounds.y);

                        if let Some(current_im) = self.current_interaction_mode {
                            self.interaction_modes[current_im as usize].on_mouse_move(
                                mouse_offset,
                                rel_pos,
                                self.camera_controller.camera,
                                &self.scene,
                                engine,
                            );
                        }
                        self.preview.last_mouse_pos = Some(pos);
                    }
                    &WidgetMessage::KeyUp(key) => {
                        self.camera_controller.on_key_up(key);
                    }
                    &WidgetMessage::KeyDown(key) => {
                        self.camera_controller.on_key_down(key);
                        match key {
                            KeyCode::Y => {
                                if ui.keyboard_modifiers().control {
                                    self.message_sender.send(Message::RedoSceneCommand).unwrap();
                                }
                            }
                            KeyCode::Z => {
                                if ui.keyboard_modifiers().control {
                                    self.message_sender.send(Message::UndoSceneCommand).unwrap();
                                }
                            }
                            KeyCode::Key1 => {
                                self.set_interaction_mode(Some(InteractionModeKind::Select), engine)
                            }
                            KeyCode::Key2 => {
                                self.set_interaction_mode(Some(InteractionModeKind::Move), engine)
                            }
                            KeyCode::Key3 => {
                                self.set_interaction_mode(Some(InteractionModeKind::Rotate), engine)
                            }
                            KeyCode::Key4 => {
                                self.set_interaction_mode(Some(InteractionModeKind::Scale), engine)
                            }
                            KeyCode::L => {
                                if ui.keyboard_modifiers().control {
                                    /*
                                    self.message_sender
                                        .send(Message::LoadScene(SCENE_PATH.into()))
                                        .unwrap();*/
                                }
                            }
                            KeyCode::Delete => {
                                if !self.scene.selection.is_empty() {
                                    let mut commands = vec![SceneCommand::ChangeSelection(
                                        ChangeSelectionCommand::new(
                                            Default::default(),
                                            self.scene.selection.clone(),
                                        ),
                                    )];
                                    for &node in self.scene.selection.nodes().iter() {
                                        commands.push(SceneCommand::DeleteNode(
                                            DeleteNodeCommand::new(node),
                                        ));
                                    }

                                    self.message_sender
                                        .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                                            commands,
                                        )))
                                        .unwrap();
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => {}
                },
                _ => (),
            }
        }
    }

    fn sync_to_model(&mut self, engine: &mut GameEngine) {
        self.world_outliner.sync_to_model(&self.scene, engine);
        self.sidebar.sync_to_model(&self.scene, engine);
    }

    fn update(&mut self, engine: &mut GameEngine, dt: f32) {
        while let Ok(message) = self.message_receiver.try_recv() {
            self.world_outliner.handle_message(&message, engine);

            let scene = &mut engine.scenes[self.scene.scene];
            let context = SceneContext {
                graph: &mut scene.graph,
                message_sender: self.message_sender.clone(),
                current_selection: self.scene.selection.clone(),
            };

            match message {
                Message::DoSceneCommand(command) => {
                    self.command_stack.do_command(command, context);
                    self.sync_to_model(engine);
                }
                Message::UndoSceneCommand => {
                    self.command_stack.undo(context);
                    self.sync_to_model(engine);
                }
                Message::RedoSceneCommand => {
                    self.command_stack.redo(context);
                    self.sync_to_model(engine);
                }
                Message::SetSelection(selection) => {
                    self.scene.selection = selection;
                    self.sync_to_model(engine);
                }
                Message::SaveScene(mut path) => {
                    self.scene.path = Some(path.clone());
                    let scene = &mut engine.scenes[self.scene.scene];
                    let editor_root = self.scene.root;
                    let mut pure_scene = scene.clone(&mut |node, _| node != editor_root);
                    let mut visitor = Visitor::new();
                    pure_scene.visit("Scene", &mut visitor).unwrap();
                    if let Err(e) = visitor.save_binary(&path) {
                        self.message_sender
                            .send(Message::Log(e.to_string()))
                            .unwrap();
                    }
                    // Add text output for debugging.
                    path.set_extension("txt");
                    if let Ok(mut file) = File::create(path) {
                        if let Err(e) = file.write(visitor.save_text().as_bytes()) {
                            self.message_sender
                                .send(Message::Log(e.to_string()))
                                .unwrap();
                        }
                    }
                }
                Message::LoadScene(path) => match Visitor::load_binary(&path) {
                    Ok(mut visitor) => {
                        let mut scene = Scene::default();
                        scene.visit("Scene", &mut visitor).unwrap();
                        self.set_scene(engine, scene, path);
                        engine.renderer.flush();
                    }
                    Err(e) => {
                        self.message_sender
                            .send(Message::Log(e.to_string()))
                            .unwrap();
                    }
                },
                Message::SetInteractionMode(mode_kind) => {
                    self.set_interaction_mode(Some(mode_kind), engine);
                }
                Message::Exit => {
                    self.exit = true;
                }
                Message::Log(msg) => {
                    println!("{}", msg);
                }
            }
        }

        // Adjust camera viewport to size of frame.
        let frame_size = engine.renderer.get_frame_size();
        let scene = &mut engine.scenes[self.scene.scene];
        if let Node::Camera(camera) = &mut scene.graph[self.camera_controller.camera] {
            let frame_size = Vec2::new(frame_size.0 as f32, frame_size.1 as f32);
            let viewport = camera.viewport_pixels(frame_size);

            if let UiNode::Image(frame) = engine.user_interface.node(self.preview.frame) {
                let preview_frame_size = frame.actual_size();
                if viewport.w != preview_frame_size.x as i32
                    || viewport.h != preview_frame_size.y as i32
                {
                    camera.set_viewport(Rect {
                        x: 0.0,
                        y: 0.0,
                        w: preview_frame_size.x / frame_size.x,
                        h: preview_frame_size.y / frame_size.y,
                    });
                }
            }
        }

        self.camera_controller.update(&self.scene, engine, dt);

        if let Some(mode) = self.current_interaction_mode {
            self.interaction_modes[mode as usize].update(
                &self.scene,
                self.camera_controller.camera,
                engine,
            );
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let primary_monitor = event_loop.primary_monitor();
    let mut monitor_dimensions = primary_monitor.size();
    monitor_dimensions.height = (monitor_dimensions.height as f32 * 0.7) as u32;
    monitor_dimensions.width = (monitor_dimensions.width as f32 * 0.7) as u32;
    let inner_size = monitor_dimensions.to_logical::<f32>(primary_monitor.scale_factor());

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_inner_size(inner_size)
        .with_title("rusty editor")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

    engine
        .resource_manager
        .lock()
        .unwrap()
        .set_textures_path("data");

    // Set ambient light.
    engine
        .renderer
        .set_ambient_color(Color::opaque(200, 200, 200));

    let mut editor = Editor::new(&mut engine);
    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
            while dt >= fixed_timestep {
                dt -= fixed_timestep;
                elapsed_time += fixed_timestep;
                engine.update(fixed_timestep);
                editor.update(&mut engine, fixed_timestep);

                while let Some(ui_message) = engine.user_interface.poll_message() {
                    editor.handle_message(&ui_message, &mut engine);
                }
            }

            engine.get_window().request_redraw();

            if editor.exit {
                *control_flow = ControlFlow::Exit;
            }
        }
        Event::RedrawRequested(_) => {
            engine.render(fixed_timestep).unwrap();
        }
        Event::WindowEvent { event, .. } => {
            match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    engine.renderer.set_frame_size(size.into());
                    engine
                        .user_interface
                        .send_message(WidgetMessage::width(editor.root_grid, size.width as f32));
                    engine
                        .user_interface
                        .send_message(WidgetMessage::height(editor.root_grid, size.height as f32));
                }
                _ => (),
            }

            if let Some(os_event) = translate_event(&event) {
                engine.user_interface.process_os_event(&os_event);
            }
        }
        _ => *control_flow = ControlFlow::Poll,
    });
}
