extern crate rg3d;

pub mod interaction;
pub mod camera;
pub mod command;
pub mod world_outliner;

use crate::{
    interaction::{
        MoveInteractionMode,
        ScaleInteractionMode,
        RotateInteractionMode,
        InteractionMode,
        InteractionModeKind,
    },
    camera::CameraController,
    command::{
        CommandStack,
        Command,
        AddNodeCommand,
        DeleteNodeCommand,
        ChangeSelectionCommand,
        RotateNodeCommand,
        MoveNodeCommand,
        ScaleNodeCommand,
    },
    world_outliner::WorldOutliner,
};
use rg3d::{
    resource::texture::TextureKind,
    renderer::{
        RenderTarget,
        surface::{Surface, SurfaceSharedData},
    },
    scene::{
        base::BaseBuilder,
        node::Node,
        Scene,
        mesh::Mesh,
        light::{LightKind, LightBuilder, SpotLight, PointLight},
    },
    event::{
        Event,
        WindowEvent,
        DeviceEvent,
    },
    event_loop::{
        EventLoop,
        ControlFlow,
    },
    core::{
        color::Color,
        pool::Handle,
        math::{
            Rect,
            vec2::Vec2,
            vec3::Vec3,
            quat::{Quat, RotationOrder},
        },
        visitor::{Visitor, Visit},
    },
    utils::{
        translate_event,
        into_any_arc,
    },
    gui::{
        grid::{GridBuilder, Column, Row},
        window::{WindowBuilder, WindowTitle},
        button::ButtonBuilder,
        message::{
            UiMessageData,
            ButtonMessage,
            WidgetMessage,
            KeyCode,
            MouseButton,
            Vec3EditorMessage,
            MenuItemMessage,
        },
        messagebox::{MessageBoxBuilder, MessageBoxButtons},
        Thickness,
        stack_panel::StackPanelBuilder,
        file_browser::FileBrowserBuilder,
        widget::WidgetBuilder,
        text::TextBuilder,
        node::StubNode,
        text_box::TextBoxBuilder,
        Orientation,
        HorizontalAlignment,
        VerticalAlignment,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        image::ImageBuilder,
        vec::Vec3EditorBuilder,
        ttf::Font,
        menu::{MenuBuilder, MenuItemBuilder, MenuItemContent},
    },
};
use std::{
    cell::RefCell,
    rc::Rc,
    path::{
        PathBuf,
        Path,
    },
    time::Instant,
    sync::{
        mpsc::{Receiver, Sender},
        mpsc,
        Arc,
        Mutex,
    },
    fs::File,
    io::Write,
};

type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type Ui = rg3d::gui::UserInterface<(), StubNode>;
type UiMessage = rg3d::gui::message::UiMessage<(), StubNode>;

struct NodeEditor {
    window: Handle<UiNode>,
    node: Handle<Node>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    sender: Sender<Message>,
}

fn make_text_mark(ui: &mut Ui, text: &str, row: usize) -> Handle<UiNode> {
    TextBuilder::new(WidgetBuilder::new()
        .with_vertical_alignment(VerticalAlignment::Center)
        .with_margin(Thickness::left(4.0))
        .on_row(row)
        .on_column(0))
        .with_text(text)
        .build(ui)
}

fn make_vec3_input_field(ui: &mut Ui, row: usize) -> Handle<UiNode> {
    Vec3EditorBuilder::new(WidgetBuilder::new()
        .with_margin(Thickness::uniform(1.0))
        .on_row(row)
        .on_column(1))
        .build(ui)
}

impl NodeEditor {
    fn new(ui: &mut Ui, sender: Sender<Message>) -> Self {
        let node_name;
        let position;
        let rotation;
        let scale;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child(make_text_mark(ui, "Name", 0))
                .with_child({
                    node_name = TextBoxBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(1)
                        .with_margin(Thickness::uniform(1.0)))
                        .build(ui);
                    node_name
                })
                .with_child(make_text_mark(ui, "Position", 1))
                .with_child({
                    position = make_vec3_input_field(ui, 1);
                    position
                })
                .with_child(make_text_mark(ui, "Rotation", 2))
                .with_child({
                    rotation = make_vec3_input_field(ui, 2);
                    rotation
                })
                .with_child(make_text_mark(ui, "Scale", 3))
                .with_child({
                    scale = make_vec3_input_field(ui, 3);
                    scale
                }))
                .add_column(Column::strict(70.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .build(ui))
            .with_title(WindowTitle::text("Node Properties"))
            .build(ui);

        Self {
            window,
            node: Default::default(),
            node_name,
            position,
            rotation,
            sender,
            scale,
        }
    }

    fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        if scene.graph.is_valid_handle(self.node) {
            let node = &scene.graph[self.node];

            let ui = &mut engine.user_interface;

            if let UiNode::TextBox(node_name) = ui.node_mut(self.node_name) {
                node_name.set_text(node.name());
            }
            ui.send_message(UiMessage {
                handled: true,
                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(node.local_transform().position())),
                destination: self.position,
            });

            let euler = node.local_transform().rotation().to_euler();
            let euler_degrees = Vec3::new(euler.x.to_degrees(), euler.y.to_degrees(), euler.z.to_degrees());
            ui.send_message(UiMessage {
                handled: true,
                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(euler_degrees)),
                destination: self.rotation,
            });

            ui.send_message(UiMessage {
                handled: true,
                data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(node.local_transform().scale())),
                destination: self.scale,
            });
        }
    }

    fn handle_message(&mut self, message: &UiMessage, editor_scene: &EditorScene, engine: &GameEngine) {
        let graph = &engine.scenes[editor_scene.scene].graph;
        if self.node.is_some() && !message.handled {
            match &message.data {
                UiMessageData::Vec3Editor(msg) => {
                    if let &Vec3EditorMessage::Value(value) = msg {
                        let transform = graph[self.node].local_transform();
                        if message.destination == self.rotation {
                            let old_rotation = transform.rotation();
                            let euler = Vec3::new(value.x.to_radians(), value.y.to_radians(), value.z.to_radians());
                            let new_rotation = Quat::from_euler(euler, RotationOrder::XYZ);
                            if !old_rotation.approx_eq(new_rotation, 0.001) {
                                self.sender
                                    .send(Message::ExecuteCommand(Command::RotateNode(RotateNodeCommand::new(self.node, old_rotation, new_rotation))))
                                    .unwrap();
                            }
                        } else if message.destination == self.position {
                            let old_position = transform.position();
                            if old_position != value {
                                self.sender
                                    .send(Message::ExecuteCommand(Command::MoveNode(MoveNodeCommand::new(self.node, old_position, value))))
                                    .unwrap();
                            }
                        } else if message.destination == self.scale {
                            let old_scale = transform.scale();
                            if old_scale != value {
                                self.sender
                                    .send(Message::ExecuteCommand(Command::ScaleNode(ScaleNodeCommand::new(self.node, old_scale, value))))
                                    .unwrap();
                            }
                        }
                    }
                }
                _ => ()
            }
        }
    }
}

pub struct FileSelector {
    browser: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl FileSelector {
    pub fn new(ui: &mut Ui) -> Self {
        let browser;
        let ok;
        let cancel;
        WindowBuilder::new(WidgetBuilder::new())
            .open(false)
            .with_title(WindowTitle::text("Select File"))
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    browser = FileBrowserBuilder::new(WidgetBuilder::new()
                        .with_height(400.0)
                        .on_column(0)
                        .on_column(0))
                        .with_filter(Rc::new(RefCell::new(|path: &Path| {
                            path.extension()
                                .map_or(false, |ext| {
                                    ext.to_string_lossy()
                                        .to_owned()
                                        .to_lowercase() == "fbx"
                                })
                        })))
                        .with_path("./data")
                        .build(ui);
                    browser
                })
                .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Right)
                    .on_column(0)
                    .on_row(1)
                    .with_child({
                        ok = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0)
                            .with_height(30.0))
                            .with_text("OK")
                            .build(ui);
                        ok
                    })
                    .with_child({
                        cancel = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_width(100.0)
                            .with_height(30.0))
                            .with_text("Cancel")
                            .build(ui);
                        cancel
                    }))
                    .with_orientation(Orientation::Horizontal)
                    .build(ui)))
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ui))
            .build(ui);

        Self {
            browser,
            ok,
            cancel,
        }
    }
}

pub struct ScenePreview {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    last_mouse_pos: Option<Vec2>,
    // Side bar stuff
    move_mode: Handle<UiNode>,
    rotate_mode: Handle<UiNode>,
    scale_mode: Handle<UiNode>,
    sender: Sender<Message>,
}

impl ScenePreview {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>) -> Self {
        let ui = &mut engine.user_interface;

        let frame;
        let move_mode;
        let rotate_mode;
        let scale_mode;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_close(false)
            .can_minimize(false)
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    frame = ImageBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(1))
                        .with_flip(true)
                        .with_texture(engine.renderer.frame_texture())
                        .build(ui);
                    frame
                })
                .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0))
                    .on_row(0)
                    .on_column(0)
                    .with_child({
                        move_mode = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0)))
                            .with_content(ImageBuilder::new(WidgetBuilder::new()
                                .with_width(32.0)
                                .with_height(32.0))
                                .with_opt_texture(into_any_arc(engine.resource_manager.lock().unwrap().request_texture("resources/move_arrow.png", TextureKind::RGBA8)))
                                .build(ui))
                            .build(ui);
                        move_mode
                    })
                    .with_child({
                        rotate_mode = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0)))
                            .with_content(ImageBuilder::new(WidgetBuilder::new()
                                .with_width(32.0)
                                .with_height(32.0))
                                .with_opt_texture(into_any_arc(engine.resource_manager.lock().unwrap().request_texture("resources/rotate_arrow.png", TextureKind::RGBA8)))
                                .build(ui))
                            .build(ui);
                        rotate_mode
                    })
                    .with_child({
                        scale_mode = ButtonBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0)))
                            .with_content(ImageBuilder::new(WidgetBuilder::new()
                                .with_width(32.0)
                                .with_height(32.0))
                                .with_opt_texture(into_any_arc(engine.resource_manager.lock().unwrap().request_texture("resources/scale_arrow.png", TextureKind::RGBA8)))
                                .build(ui))
                            .build(ui);
                        scale_mode
                    }))
                    .build(ui)))
                .add_row(Row::stretch())
                .add_column(Column::auto())
                .add_column(Column::stretch())
                .build(ui))
            .with_title(WindowTitle::text("Scene Preview"))
            .build(ui);

        Self {
            sender,
            window,
            frame,
            last_mouse_pos: None,
            move_mode,
            rotate_mode,
            scale_mode,
        }
    }
}

impl ScenePreview {
    fn handle_message(&mut self, message: &UiMessage) {
        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination == self.scale_mode {
                        self.sender.send(Message::SetInteractionMode(InteractionModeKind::Scale)).unwrap();
                    } else if message.destination == self.rotate_mode {
                        self.sender.send(Message::SetInteractionMode(InteractionModeKind::Rotate)).unwrap();
                    } else if message.destination == self.move_mode {
                        self.sender.send(Message::SetInteractionMode(InteractionModeKind::Move)).unwrap();
                    }
                }
            }
            _ => ()
        }
    }
}

#[derive(Debug)]
pub enum Message {
    ExecuteCommand(Command),
    Undo,
    Redo,
    SetSelection(Handle<Node>),
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    SetInteractionMode(InteractionModeKind),
    Exit,
}

pub struct EditorScene {
    scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    root: Handle<Node>,
}

struct Editor {
    node_editor: NodeEditor,
    camera_controller: CameraController,
    scene: EditorScene,
    command_stack: CommandStack,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    interaction_modes: Vec<Box<dyn InteractionMode>>,
    current_interaction_mode: Option<InteractionModeKind>,
    world_outliner: WorldOutliner,
    root_grid: Handle<UiNode>,
    file_selector: FileSelector,
    preview: ScenePreview,
    menu: Menu,
    exit: bool,
}

fn execute_command(editor_scene: &EditorScene, engine: &mut GameEngine, command: &mut Command, message_sender: Sender<Message>, current_selection: Handle<Node>) {
    let scene = &mut engine.scenes[editor_scene.scene];
    let graph = &mut scene.graph;

    match command {
        Command::CommandGroup(group) => {
            for command in group {
                execute_command(editor_scene, engine, command, message_sender.clone(), current_selection)
            }
        }
        Command::AddNode(command) => command.execute(graph),
        Command::MoveNode(command) => command.execute(graph),
        Command::ScaleNode(command) => command.execute(graph),
        Command::RotateNode(command) => command.execute(graph),
        Command::ChangeSelection(command) => {
            let selection = command.execute();
            if selection != current_selection {
                // Just re-post message so every system will handle it correctly.
                message_sender
                    .send(Message::SetSelection(selection))
                    .unwrap();
            }
        }
        Command::LinkNodes(command) => command.execute(graph),
        Command::DeleteNode(command) => command.execute(graph)
    }
}

fn revert_command(editor_scene: &EditorScene, engine: &mut GameEngine, command: &mut Command, message_sender: Sender<Message>, current_selection: Handle<Node>) {
    let scene = &mut engine.scenes[editor_scene.scene];
    let graph = &mut scene.graph;

    match command {
        Command::CommandGroup(group) => {
            for command in group {
                revert_command(editor_scene, engine, command, message_sender.clone(), current_selection)
            }
        }
        Command::AddNode(command) => command.revert(graph),
        Command::MoveNode(command) => command.revert(graph),
        Command::ScaleNode(command) => command.revert(graph),
        Command::RotateNode(command) => command.revert(graph),
        Command::ChangeSelection(command) => {
            let selection = command.revert();
            if selection != current_selection {
                // Just re-post message so every system will handle it correctly.
                message_sender
                    .send(Message::SetSelection(selection))
                    .unwrap();
            }
        }
        Command::LinkNodes(command) => command.revert(graph),
        Command::DeleteNode(command) => command.revert(graph),
    }
}

fn finalize_command(editor_scene: &EditorScene, engine: &mut GameEngine, command: Command) {
    let scene = &mut engine.scenes[editor_scene.scene];
    let graph = &mut scene.graph;

    match command {
        Command::CommandGroup(group) => {
            for command in group {
                finalize_command(editor_scene, engine, command);
            }
        }
        Command::AddNode(command) => command.finalize(graph),
        Command::DeleteNode(command) => command.finalize(graph),
        // Intentionally not using _ => () to be notified about new commands by compiler.
        Command::ChangeSelection(_) => {}
        Command::MoveNode(_) => {}
        Command::ScaleNode(_) => {}
        Command::RotateNode(_) => {}
        Command::LinkNodes(_) => {}
    }
}

struct Menu {
    menu: Handle<UiNode>,
    save: Handle<UiNode>,
    save_as: Handle<UiNode>,
    load: Handle<UiNode>,
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
}

impl Menu {
    pub fn new(ui: &mut Ui, message_sender: Sender<Message>) -> Self {
        let min_size = Vec2::new(120.0, 20.0);
        let min_size_menu = Vec2::new(40.0, 20.0);
        let save;
        let save_as;
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
        let menu = MenuBuilder::new(WidgetBuilder::new()
            .on_row(0))
            .with_items(vec![
                MenuItemBuilder::new(WidgetBuilder::new()
                    .with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text("File"))
                    .with_items(vec![
                        {
                            save = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Save Scene", "Ctrl+S"))
                                .build(ui);
                            save
                        },
                        {
                            save_as = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Save Scene As...", "Ctrl+Shift+S"))
                                .build(ui);
                            save_as
                        },
                        {
                            load = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Load Scene...", "Ctrl+L"))
                                .build(ui);
                            load
                        },
                        {
                            exit = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Exit", "Alt+F4"))
                                .build(ui);
                            exit
                        }
                    ])
                    .build(ui),
                MenuItemBuilder::new(WidgetBuilder::new()
                    .with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text_with_shortcut("Edit", ""))
                    .with_items(vec![
                        {
                            undo = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Undo", "Ctrl+Z"))
                                .build(ui);
                            undo
                        },
                        {
                            redo = MenuItemBuilder::new(WidgetBuilder::new()
                                .with_min_size(min_size))
                                .with_content(MenuItemContent::text_with_shortcut("Redo", "Ctrl+Y"))
                                .build(ui);
                            redo
                        },
                    ])
                    .build(ui),
                MenuItemBuilder::new(WidgetBuilder::new()
                    .with_min_size(min_size_menu))
                    .with_content(MenuItemContent::text_with_shortcut("Create", ""))
                    .with_items(vec![
                        MenuItemBuilder::new(WidgetBuilder::new()
                            .with_min_size(min_size))
                            .with_content(MenuItemContent::text("Mesh"))
                            .with_items(vec![
                                {
                                    create_cube = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Cube"))
                                        .build(ui);
                                    create_cube
                                },
                                {
                                    create_sphere = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Sphere"))
                                        .build(ui);
                                    create_sphere
                                },
                                {
                                    create_cylinder = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Cylinder"))
                                        .build(ui);
                                    create_cylinder
                                },
                                {
                                    create_cone = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Cone"))
                                        .build(ui);
                                    create_cone
                                },
                            ])
                            .build(ui),
                        MenuItemBuilder::new(WidgetBuilder::new()
                            .with_min_size(min_size))
                            .with_content(MenuItemContent::text("Light"))
                            .with_items(vec![
                                {
                                    create_spot_light = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Spot Light"))
                                        .build(ui);
                                    create_spot_light
                                },
                                {
                                    create_point_light = MenuItemBuilder::new(WidgetBuilder::new()
                                        .with_min_size(min_size))
                                        .with_content(MenuItemContent::text("Point Light"))
                                        .build(ui);
                                    create_point_light
                                }
                            ])
                            .build(ui),
                    ])
                    .build(ui)
            ])
            .build(ui);

        Self {
            menu,
            save,
            save_as,
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
        }
    }

    fn handle_message(&mut self, message: &UiMessage) {
        match &message.data {
            UiMessageData::MenuItem(msg) => {
                if let MenuItemMessage::Click = msg {
                    if message.destination == self.create_cube {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cube");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(SurfaceSharedData::make_cube(Default::default())))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.create_spot_light {
                        let kind = LightKind::Spot(SpotLight::new(10.0, 45.0, 2.0));
                        let mut light = LightBuilder::new(kind, BaseBuilder::new()).build();
                        light.set_name("SpotLight");
                        let node = Node::Light(light);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.create_point_light {
                        let kind = LightKind::Point(PointLight::new(10.0));
                        let mut light = LightBuilder::new(kind, BaseBuilder::new()).build();
                        light.set_name("PointLight");
                        let node = Node::Light(light);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.create_cone {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cone");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(SurfaceSharedData::make_cone(16, 1.0, 1.0, Default::default())))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.create_cylinder {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Cylinder");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(SurfaceSharedData::make_cylinder(16, 1.0, 1.0, true, Default::default())))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.create_sphere {
                        let mut mesh = Mesh::default();
                        mesh.set_name("Sphere");
                        mesh.add_surface(Surface::new(Arc::new(Mutex::new(SurfaceSharedData::make_sphere(16, 16, 1.0)))));
                        let node = Node::Mesh(mesh);
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::AddNode(AddNodeCommand::new(node))))
                            .unwrap();
                    } else if message.destination == self.save {
                        self.message_sender
                            .send(Message::SaveScene("test_scene.rgs".into()))
                            .unwrap();
                    } else if message.destination == self.load {
                        self.message_sender
                            .send(Message::LoadScene("test_scene.rgs".into()))
                            .unwrap();
                    } else if message.destination == self.undo {
                        self.message_sender
                            .send(Message::Undo)
                            .unwrap();
                    } else if message.destination == self.redo {
                        self.message_sender
                            .send(Message::Redo)
                            .unwrap();
                    } else if message.destination == self.exit {
                        self.message_sender
                            .send(Message::Exit)
                            .unwrap();
                    }
                }
            }
            _ => ()
        }
    }
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        let preview = ScenePreview::new(engine, message_sender.clone());

        let ui = &mut engine.user_interface;
        *rg3d::gui::DEFAULT_FONT.lock().unwrap() = Font::from_file("resources/arial.ttf", 14.0, Font::default_char_set()).unwrap();

        let mut scene = Scene::new();
        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        let editor_scene = EditorScene {
            root,
            scene: engine.scenes.add(scene),
        };

        let node_editor = NodeEditor::new(ui, message_sender.clone());
        let world_outliner = WorldOutliner::new(ui, message_sender.clone());
        let menu = Menu::new(ui, message_sender.clone());

        let root_grid = GridBuilder::new(WidgetBuilder::new()
            .with_width(engine.renderer.get_frame_size().0 as f32)
            .with_height(engine.renderer.get_frame_size().1 as f32)
            .with_child(menu.menu)
            .with_child(DockingManagerBuilder::new(WidgetBuilder::new()
                .on_row(1)
                .with_child(TileBuilder::new(WidgetBuilder::new())
                    .with_content(TileContent::HorizontalTiles {
                        splitter: 0.25,
                        tiles: [
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(world_outliner.window))
                                .build(ui),
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::HorizontalTiles {
                                    splitter: 0.66,
                                    tiles: [
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(preview.window))
                                            .build(ui),
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(node_editor.window))
                                            .build(ui)
                                    ],
                                })
                                .build(ui),
                        ],
                    })
                    .build(ui)))
                .build(ui)))
            .add_row(Row::strict(25.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .build(ui);

        let file_selector = FileSelector::new(ui);

        let msg = MessageBoxBuilder::new(WindowBuilder::new(WidgetBuilder::new()
            .with_max_size(Vec2::new(430.0, 220.0)))
            .with_title(WindowTitle::Text("Welcome".to_owned())))
            .with_text(
                "Hello! Welcome to rusty editor - scene editor for rg3d engine.\n\
                      This editor is far from completion, some parts may (and probably\n\
                      will) work weird or even not work, currently editor is in active\n\
                      development as well as the rg3d-ui library it is based on.\n\n\
                      [W][S][A][D] - move camera\n\
                      [RMB] - rotate camera\n\
                      [LMB] - pick entities\n\n\
                      To start you can use Create menu option to make some basic\n\
                      objects.")
            .with_buttons(MessageBoxButtons::Ok)
            .build(ui);
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::Widget(WidgetMessage::Center),
            destination: msg
        });

        let interaction_modes: Vec<Box<dyn InteractionMode>> = vec![
            Box::new(MoveInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            Box::new(ScaleInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            Box::new(RotateInteractionMode::new(&editor_scene, engine, message_sender.clone())),
        ];

        let mut editor = Self {
            node_editor,
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
            file_selector,
            menu,
            exit: false
        };

        editor.set_interaction_mode(Some(InteractionModeKind::Move), engine);

        editor
    }

    fn set_scene(&mut self, engine: &mut GameEngine, mut scene: Scene) {
        engine.scenes.remove(self.scene.scene);

        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        self.scene = EditorScene {
            root,
            scene: engine.scenes.add(scene),
        };

        self.interaction_modes = vec![
            Box::new(MoveInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
            Box::new(ScaleInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
            Box::new(RotateInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
        ];

        self.world_outliner.clear(&mut engine.user_interface);
        self.camera_controller = CameraController::new(&self.scene, engine);
        self.command_stack = CommandStack::new();

        self.set_interaction_mode(Some(InteractionModeKind::Move), engine);
    }

    fn set_interaction_mode(&mut self, mode: Option<InteractionModeKind>, engine: &mut GameEngine) {
        if self.current_interaction_mode != mode {
            // Deactivate current first.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode as usize].deactivate(&self.scene, engine);
            }

            self.current_interaction_mode = mode;

            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode as usize].activate(self.node_editor.node);
            }
        }
    }

    fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        self.node_editor.handle_message(message, &self.scene, engine);
        self.menu.handle_message(message);

        let ui = &mut engine.user_interface;
        self.world_outliner.handle_ui_message(message, ui, self.node_editor.node);
        self.preview.handle_message(message);

        if message.destination == self.preview.frame {
            match &message.data {
                UiMessageData::Widget(msg) => {
                    match msg {
                        &WidgetMessage::MouseDown { button, pos, .. } => {
                            ui.capture_mouse(self.preview.frame);
                            if button == MouseButton::Left {
                                if let Some(current_im) = self.current_interaction_mode {
                                    let screen_bounds = ui.node(self.preview.frame).screen_bounds();
                                    let rel_pos = Vec2::new(pos.x - screen_bounds.x, pos.y - screen_bounds.y);
                                    self.interaction_modes[current_im as usize].on_left_mouse_button_down(&self.scene, &mut self.camera_controller, self.node_editor.node, engine, rel_pos);
                                }
                            }
                            self.camera_controller.on_mouse_button_down(button);
                        }
                        &WidgetMessage::MouseUp { button, .. } => {
                            ui.release_mouse_capture();
                            if button == MouseButton::Left {
                                if let Some(current_im) = self.current_interaction_mode {
                                    self.interaction_modes[current_im as usize].on_left_mouse_button_up(&self.scene, engine);
                                }
                            }
                            self.camera_controller.on_mouse_button_up(button);
                        }
                        &WidgetMessage::MouseWheel { amount, .. } => {
                            self.camera_controller.on_mouse_wheel(amount, &self.scene, engine);
                        }
                        &WidgetMessage::MouseMove { pos, .. } => {
                            let last_pos = *self.preview.last_mouse_pos.get_or_insert(pos);
                            let mouse_offset = pos - last_pos;
                            self.camera_controller.on_mouse_move(mouse_offset);
                            let screen_bounds = ui.node(self.preview.frame).screen_bounds();
                            let rel_pos = Vec2::new(pos.x - screen_bounds.x, pos.y - screen_bounds.y);
                            if let Some(current_im) = self.current_interaction_mode {
                                self.interaction_modes[current_im as usize].on_mouse_move(mouse_offset, rel_pos, self.camera_controller.camera, &self.scene, engine);
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
                                    self.message_sender.send(Message::Redo).unwrap();
                                }
                                KeyCode::Z => {
                                    self.message_sender.send(Message::Undo).unwrap();
                                }
                                KeyCode::Key1 => {
                                    self.set_interaction_mode(Some(InteractionModeKind::Move), engine)
                                }
                                KeyCode::Key2 => {
                                    self.set_interaction_mode(Some(InteractionModeKind::Rotate), engine)
                                }
                                KeyCode::Key3 => {
                                    self.set_interaction_mode(Some(InteractionModeKind::Scale), engine)
                                }
                                KeyCode::Delete => {
                                    if self.node_editor.node.is_some() {
                                        let command = Command::CommandGroup(vec![
                                            Command::ChangeSelection(ChangeSelectionCommand::new(Handle::NONE, self.node_editor.node)),
                                            Command::DeleteNode(DeleteNodeCommand::new(self.node_editor.node))
                                        ]);
                                        self.message_sender
                                            .send(Message::ExecuteCommand(command))
                                            .unwrap();
                                    }
                                }
                                _ => ()
                            }
                        }
                        _ => {}
                    }
                }
                _ => ()
            }
        }
    }

    fn undo_command(&mut self, engine: &mut GameEngine) {
        if let Some(command) = self.command_stack.undo() {
            println!("Undo command {:?}", command);
            revert_command(&self.scene, engine, command, self.message_sender.clone(), self.node_editor.node);
        }
        self.sync_to_model(engine);
    }

    fn redo_command(&mut self, engine: &mut GameEngine) {
        if let Some(command) = self.command_stack.redo() {
            println!("Redo command {:?}", command);
            execute_command(&self.scene, engine, command, self.message_sender.clone(), self.node_editor.node);
        }
        self.sync_to_model(engine);
    }

    fn add_command(&mut self, engine: &mut GameEngine, mut command: Command) {
        execute_command(&self.scene, engine, &mut command, self.message_sender.clone(), self.node_editor.node);
        let dropped_commands = self.command_stack.add_command(command);
        for command in dropped_commands {
            println!("Finalizing command {:?}", command);
            finalize_command(&self.scene, engine, command);
        }
        self.sync_to_model(engine);
    }

    fn sync_to_model(&mut self, engine: &mut GameEngine) {
        self.world_outliner.sync_to_model(&self.scene, engine);
        self.node_editor.sync_to_model(&self.scene, engine);
    }

    fn update(&mut self, engine: &mut GameEngine, dt: f32) {
        while let Ok(message) = self.message_receiver.try_recv() {
            for mode in &mut self.interaction_modes {
                mode.handle_message(&message);
            }

            self.world_outliner.handle_message(&message, engine);

            match message {
                Message::ExecuteCommand(command) => {
                    println!("Executing command: {:?}", &command);
                    self.add_command(engine, command)
                }
                Message::Undo => self.undo_command(engine),
                Message::Redo => self.redo_command(engine),
                Message::SetSelection(node) => self.node_editor.node = node,
                Message::SaveScene(mut path) => {
                    let scene = &mut engine.scenes[self.scene.scene];
                    let editor_root = self.scene.root;
                    let mut pure_scene = scene.clone(&mut |node, _| node != editor_root);
                    let mut visitor = Visitor::new();
                    pure_scene.visit("Scene", &mut visitor).unwrap();
                    visitor.save_binary(&path).unwrap();
                    // Add text output for debugging.
                    path.set_extension("txt");
                    if let Ok(mut file) = File::create(path) {
                        file.write(visitor.save_text().as_bytes()).unwrap();
                    }
                }
                Message::LoadScene(path) => {
                    if let Ok(mut visitor) = Visitor::load_binary(&path) {
                        let mut scene = Scene::default();
                        scene.visit("Scene", &mut visitor).unwrap();
                        self.set_scene(engine, scene);
                        engine.renderer.flush();
                    }
                }
                Message::SetInteractionMode(mode_kind) => {
                    self.set_interaction_mode(Some(mode_kind), engine);
                }
                Message::Exit => {
                    self.exit = true;
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
                if viewport.w != preview_frame_size.x as i32 || viewport.h != preview_frame_size.y as i32 {
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
            self.interaction_modes[mode as usize].update(&self.scene, self.camera_controller.camera, engine);
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

    engine.renderer.set_render_target(RenderTarget::Texture);
    engine.resource_manager.lock().unwrap().set_textures_path("data");

    // Set ambient light.
    engine.renderer.set_ambient_color(Color::opaque(200, 200, 200));

    let mut editor = Editor::new(&mut engine);
    let clock = Instant::now();
    let fixed_timestep = 1.0 / 60.0;
    let mut elapsed_time = 0.0;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                let mut dt = clock.elapsed().as_secs_f32() - elapsed_time;
                while dt >= fixed_timestep {
                    dt -= fixed_timestep;
                    elapsed_time += fixed_timestep;
                    engine.update(fixed_timestep);

                    editor.update(&mut engine, dt);
                }

                while let Some(ui_message) = engine.user_interface.poll_message() {
                    editor.handle_message(&ui_message, &mut engine);
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
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Resized(size) => {
                        engine.renderer.set_frame_size(size.into());
                        engine.user_interface
                            .node_mut(editor.root_grid)
                            .set_width_mut(size.width as f32)
                            .set_height_mut(size.height as f32);
                    }
                    _ => ()
                }

                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}