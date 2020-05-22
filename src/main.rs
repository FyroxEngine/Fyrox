extern crate rg3d;

pub mod interaction;
pub mod camera;
pub mod command;
pub mod world_outliner;

use crate::{
    interaction::{
        InteractionMode,
        MoveInteractionMode,
        ScaleInteractionMode,
        RotateInteractionMode,
        InteractionModeTrait,
        InteractionModeKind,
    },
    camera::CameraController,
    command::{
        CommandStack,
        Command,
        CreateNodeCommand,
        NodeKind,
        DeleteNodeCommand,
        ChangeSelectionCommand,
        RotateNodeCommand,
        MoveNodeCommand,
        ScaleNodeCommand
    },
    world_outliner::WorldOutliner,
};
use std::{
    path::PathBuf,
    time::Instant,
    sync::{
        mpsc::{Receiver, Sender},
        mpsc,
    },
    fs::File,
    io::Write,
};
use rg3d::{
    resource::texture::TextureKind,
    renderer::RenderTarget,
    scene::{
        base::BaseBuilder,
        node::Node,
        Scene,
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
    utils::translate_event,
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
        },
        Thickness,
        stack_panel::StackPanelBuilder,
        file_browser::FileBrowserBuilder,
        widget::WidgetBuilder,
        text::TextBuilder,
        node::StubNode,
        text_box::TextBoxBuilder,
        scroll_bar::Orientation,
        HorizontalAlignment,
        VerticalAlignment,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        image::ImageBuilder,
        vec::Vec3EditorBuilder,
    },
    utils::into_any_arc,
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
                .add_column(Column::strict(110.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::stretch())
                .build(ui))
            .with_title(WindowTitle::Text("Node Properties"))
            .build(ui);

        Self {
            window,
            node: Default::default(),
            node_name,
            position,
            rotation,
            sender,
            scale
        }
    }

    fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if self.node.is_some() {
            let scene = &engine.scenes[editor_scene.scene];
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
                destination: self.scale
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
}

impl FileSelector {
    pub fn new(ui: &mut Ui) -> Self {
        let browser;
        WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::Text("Select File"))
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    browser = FileBrowserBuilder::new(WidgetBuilder::new()
                        .with_height(400.0)
                        .on_column(0)
                        .on_column(0))
                        .with_path("./data")
                        .build(ui);
                    browser
                })
                .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Right)
                    .on_column(0)
                    .on_row(1)
                    .with_child(ButtonBuilder::new(WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_width(100.0)
                        .with_height(30.0))
                        .with_content(TextBuilder::new(WidgetBuilder::new())
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text("OK")
                            .build(ui))
                        .build(ui))
                    .with_child(ButtonBuilder::new(WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_width(100.0)
                        .with_height(30.0))
                        .with_content(TextBuilder::new(WidgetBuilder::new())
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text("Cancel")
                            .build(ui))
                        .build(ui)))
                    .with_orientation(Orientation::Horizontal)
                    .build(ui)))
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ui))
            .build(ui);

        Self {
            browser
        }
    }
}

struct EntityPanel {
    window: Handle<UiNode>,
    create_cube: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    message_sender: Sender<Message>,
    save_scene: Handle<UiNode>,
    load_scene: Handle<UiNode>,
}

impl EntityPanel {
    pub fn new(ui: &mut Ui, message_sender: Sender<Message>) -> Self {
        let create_cube;
        let create_spot_light;
        let create_point_light;
        let save_scene;
        let load_scene;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child({
                    create_cube = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(0))
                        .with_text("Create Cube")
                        .build(ui);
                    create_cube
                })
                .with_child({
                    create_spot_light = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(1)
                        .on_column(0))
                        .with_text("Create Spot Light")
                        .build(ui);
                    create_spot_light
                })
                .with_child({
                    create_point_light = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(2)
                        .on_column(0))
                        .with_text("Create Point Light")
                        .build(ui);
                    create_point_light
                })
                .with_child({
                    save_scene = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(3)
                        .on_column(0))
                        .with_text("Save")
                        .build(ui);
                    save_scene
                })
                .with_child({
                    load_scene = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(4)
                        .on_column(0))
                        .with_text("Load")
                        .build(ui);
                    load_scene
                }))
                .add_column(Column::stretch())
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .add_row(Row::strict(32.0))
                .build(ui))
            .with_title(WindowTitle::Text("Entity Panel"))
            .build(ui);

        Self {
            message_sender,
            window,
            create_cube,
            create_spot_light,
            create_point_light,
            save_scene,
            load_scene,
        }
    }

    fn handle_message(&mut self, message: &UiMessage) {
        match &message.data {
            UiMessageData::Button(button) => {
                if let ButtonMessage::Click = button {
                    if message.destination == self.create_cube {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::Cube))))
                            .unwrap();
                    } else if message.destination == self.create_spot_light {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::SpotLight))))
                            .unwrap();
                    } else if message.destination == self.create_point_light {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::PointLight))))
                            .unwrap();
                    } else if message.destination == self.save_scene {
                        self.message_sender
                            .send(Message::SaveScene("test_scene.rgs".into()))
                            .unwrap();
                    } else if message.destination == self.load_scene {
                        self.message_sender
                            .send(Message::LoadScene("test_scene.rgs".into()))
                            .unwrap();
                    }
                }
            }
            _ => ()
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
            .with_title(WindowTitle::Text("Scene Preview"))
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
}

pub struct EditorScene {
    scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    root: Handle<Node>,
}

struct Editor {
    node_editor: NodeEditor,
    entity_panel: EntityPanel,
    camera_controller: CameraController,
    scene: EditorScene,
    command_stack: CommandStack,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    interaction_modes: Vec<InteractionMode>,
    current_interaction_mode: Option<usize>,
    world_outliner: WorldOutliner,
    docking_manager: Handle<UiNode>,
    file_selector: FileSelector,
    preview: ScenePreview,
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
        Command::CreateNode(command) => command.execute(graph),
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
        Command::CreateNode(command) => command.revert(graph),
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
        Command::CreateNode(command) => command.finalize(graph),
        Command::DeleteNode(command) => command.finalize(graph),
        // Intentionally not using _ => () to be notified about new commands by compiler.
        Command::ChangeSelection(_) => {}
        Command::MoveNode(_) => {}
        Command::ScaleNode(_) => {}
        Command::RotateNode(_) => {}
        Command::LinkNodes(_) => {}
    }
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        let preview = ScenePreview::new(engine, message_sender.clone());

        let ui = &mut engine.user_interface;

        let mut scene = Scene::new();
        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        let editor_scene = EditorScene {
            root,
            scene: engine.scenes.add(scene),
        };

        let node_editor = NodeEditor::new(ui, message_sender.clone());
        let entity_panel = EntityPanel::new(ui, message_sender.clone());
        let world_outliner = WorldOutliner::new(ui, message_sender.clone());

        let docking_manager = DockingManagerBuilder::new(WidgetBuilder::new()
            .with_width(engine.renderer.get_frame_size().0 as f32)
            .with_height(engine.renderer.get_frame_size().1 as f32)
            .with_child(TileBuilder::new(WidgetBuilder::new())
                .with_content(TileContent::VerticalTiles {
                    splitter: 0.7,
                    tiles: [
                        TileBuilder::new(WidgetBuilder::new())
                            .with_content(TileContent::HorizontalTiles {
                                splitter: 0.75,
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
                        TileBuilder::new(WidgetBuilder::new())
                            .with_content(TileContent::HorizontalTiles {
                                splitter: 0.5,
                                tiles: [
                                    TileBuilder::new(WidgetBuilder::new())
                                        .with_content(TileContent::Window(world_outliner.window))
                                        .build(ui),
                                    TileBuilder::new(WidgetBuilder::new())
                                        .with_content(TileContent::Window(entity_panel.window))
                                        .build(ui)
                                ],
                            })
                            .build(ui)
                    ],
                })
                .build(ui)))
            .build(ui);

        let file_selector = FileSelector::new(ui);

        let interaction_modes = vec![
            InteractionMode::Move(MoveInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            InteractionMode::Scale(ScaleInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            InteractionMode::Rotate(RotateInteractionMode::new(&editor_scene, engine, message_sender.clone())),
        ];

        let mut editor = Self {
            node_editor,
            entity_panel,
            preview,
            camera_controller: CameraController::new(&editor_scene, engine),
            scene: editor_scene,
            command_stack: CommandStack::new(),
            message_sender,
            message_receiver,
            interaction_modes,
            current_interaction_mode: None,
            world_outliner,
            docking_manager,
            file_selector,
        };

        editor.set_interaction_mode(Some(0), engine);

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
            InteractionMode::Move(MoveInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
            InteractionMode::Scale(ScaleInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
            InteractionMode::Rotate(RotateInteractionMode::new(&self.scene, engine, self.message_sender.clone())),
        ];

        self.world_outliner.clear(&mut engine.user_interface);
        self.camera_controller = CameraController::new(&self.scene, engine);
        self.command_stack = CommandStack::new();

        self.set_interaction_mode(Some(0), engine);
    }

    fn handle_raw_input(&mut self, device_event: &DeviceEvent, engine: &mut GameEngine) {
        match device_event {
            &DeviceEvent::MouseMotion { delta } => {
                let mouse_offset = Vec2::new(delta.0 as f32, delta.1 as f32);
                if let Some(current_im) = self.current_interaction_mode {
                    self.interaction_modes[current_im].on_mouse_move(mouse_offset, self.camera_controller.camera, &self.scene, engine);
                }
            }
            _ => ()
        }
    }

    fn set_interaction_mode(&mut self, index: Option<usize>, engine: &mut GameEngine) {
        if self.current_interaction_mode != index {
            // Deactivate current first.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode].deactivate(&self.scene, engine);
            }

            self.current_interaction_mode = index;
        }
    }

    fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        self.node_editor.handle_message(message, &self.scene, engine);
        self.entity_panel.handle_message(message);

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
                                    self.interaction_modes[current_im].on_left_mouse_button_down(&self.scene, &mut self.camera_controller, self.node_editor.node, engine, rel_pos);
                                }
                            }
                            self.camera_controller.on_mouse_button_down(button);
                        }
                        &WidgetMessage::MouseUp { button, .. } => {
                            ui.release_mouse_capture();
                            if button == MouseButton::Left {
                                if let Some(current_im) = self.current_interaction_mode {
                                    self.interaction_modes[current_im].on_left_mouse_button_up(&self.scene, engine);
                                }
                            }
                            self.camera_controller.on_mouse_button_up(button);
                        }
                        &WidgetMessage::MouseWheel { amount, .. } => {
                            self.camera_controller.on_mouse_wheel(amount, &self.scene, engine);
                        }
                        &WidgetMessage::MouseMove { pos, .. } => {
                            let last_pos = *self.preview.last_mouse_pos.get_or_insert(pos);
                            self.camera_controller.on_mouse_move(pos - last_pos);
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
                                    self.set_interaction_mode(Some(0), engine)
                                }
                                KeyCode::Key2 => {
                                    self.set_interaction_mode(Some(1), engine)
                                }
                                KeyCode::Key3 => {
                                    self.set_interaction_mode(Some(2), engine)
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
                    self.set_interaction_mode(Some(mode_kind as usize), engine);
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
            self.interaction_modes[mode].update(&self.scene, self.camera_controller.camera, engine);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
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
                            .node_mut(editor.docking_manager)
                            .set_width_mut(size.width as f32)
                            .set_height_mut(size.height as f32);
                    }
                    _ => ()
                }

                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }
            }
            Event::DeviceEvent { event, .. } => {
                editor.handle_raw_input(&event, &mut engine);
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}