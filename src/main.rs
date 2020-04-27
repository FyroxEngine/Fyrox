extern crate rg3d;

pub mod interaction;
pub mod camera;
pub mod command;

use rg3d::{
    scene::{
        base::BaseBuilder,
        node::Node,
        Scene,
    },
    event::{
        Event,
        WindowEvent,
        DeviceEvent,
        VirtualKeyCode,
        ElementState,
        MouseButton,
    },
    event_loop::{
        EventLoop,
        ControlFlow,
    },
    core::{
        color::Color,
        pool::Handle,
        math::{
            vec2::Vec2,
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
        },
        widget::WidgetBuilder,
        text::TextBuilder,
        node::StubNode,
        text_box::TextBoxBuilder,
    },
};
use std::{
    time::Instant,
    sync::{
        mpsc::{Receiver, Sender},
        mpsc
    },
    path::PathBuf
};
use crate::{
    interaction::{
        InteractionMode,
        MoveInteractionMode,
        ScaleInteractionMode,
        RotateInteractionMode,
        InteractionModeTrait
    },
    camera::CameraController,
    command::{
        CommandStack,
        Command,
        CreateNodeCommand,
        NodeKind,
    },
};

type GameEngine = rg3d::engine::Engine<(), StubNode>;
type UiNode = rg3d::gui::node::UINode<(), StubNode>;
type Ui = rg3d::gui::UserInterface<(), StubNode>;
type UiMessage = rg3d::gui::message::UiMessage<(), StubNode>;

struct NodeEditor {
    window: Handle<UiNode>,
    node: Handle<Node>,

    node_name: Handle<UiNode>,
}

impl NodeEditor {
    fn new(ui: &mut Ui) -> Self {
        let node_name;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0))
                    .with_text("Name")
                    .build(ui))
                .with_child({
                    node_name = TextBoxBuilder::new(WidgetBuilder::new()
                        .on_row(0)
                        .on_column(1))
                        .build(ui);
                    node_name
                }))
                .add_column(Column::strict(110.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(32.0))
                .build(ui))
            .with_title(WindowTitle::Text("Node Properties"))
            .build(ui);

        Self {
            window,
            node: Default::default(),
            node_name,
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
        }
    }
}

struct EntityPanel {
    window: Handle<UiNode>,
    create_cube: Handle<UiNode>,
    create_spot_light: Handle<UiNode>,
    create_point_light: Handle<UiNode>,
    message_sender: Sender<Message>,
}

impl EntityPanel {
    pub fn new(ui: &mut Ui, message_sender: Sender<Message>) -> Self {
        let create_cube;
        let create_spot_light;
        let create_point_light;
        let window = WindowBuilder::new(WidgetBuilder::new()
            .with_width(250.0))
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
                }))
                .add_column(Column::stretch())
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
        }
    }

    fn handle_message(&mut self, message: &UiMessage) {
        match &message.data {
            UiMessageData::Button(button) => {
                if let ButtonMessage::Click = button {
                    if message.source() == self.create_cube {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::Cube))))
                            .unwrap();
                    } else if message.source() == self.create_spot_light {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::SpotLight))))
                            .unwrap();
                    } else if message.source() == self.create_point_light {
                        self.message_sender
                            .send(Message::ExecuteCommand(Command::CreateNode(CreateNodeCommand::new(NodeKind::PointLight))))
                            .unwrap();
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
}

fn execute_command(editor_scene: &EditorScene, engine: &mut GameEngine, command: &mut Command, message_sender: Sender<Message>) {
    let scene = &mut engine.scenes[editor_scene.scene];
    let graph = &mut scene.graph;

    match command {
        Command::CreateNode(create_node_command) => create_node_command.execute(graph),
        Command::MoveNode(move_node_command) => move_node_command.execute(graph),
        Command::ScaleNode(scale_node_command) => scale_node_command.execute(graph),
        Command::RotateNode(rotate_node_command) => rotate_node_command.execute(graph),
        Command::ChangeSelection(change_selection_command) => {
            // Just re-cast message so every system will handle it correctly.
            message_sender.send(Message::SetSelection(change_selection_command.execute()))
                .unwrap();
        }
    }
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let ui = &mut engine.user_interface;

        let mut scene = Scene::new();
        let root = scene.graph.add_node(Node::Base(BaseBuilder::new().build()));

        let editor_scene = EditorScene {
            root,
            scene: engine.scenes.add(scene),
        };

        let (message_sender, message_receiver) = mpsc::channel();

        let node_editor = NodeEditor::new(ui);
        let entity_panel = EntityPanel::new(ui, message_sender.clone());

        let interaction_modes = vec![
            InteractionMode::Move(MoveInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            InteractionMode::Scale(ScaleInteractionMode::new(&editor_scene, engine, message_sender.clone())),
            InteractionMode::Rotate(RotateInteractionMode::new(&editor_scene, engine, message_sender.clone())),
        ];

        let mut editor = Self {
            node_editor,
            entity_panel,
            camera_controller: CameraController::new(&editor_scene, engine),
            scene: editor_scene,
            command_stack: CommandStack::new(),
            message_sender,
            message_receiver,
            interaction_modes,
            current_interaction_mode: None,
        };

        editor.set_interaction_mode(Some(0), engine);

        editor
    }

    fn sync_to_model(&mut self, engine: &mut GameEngine) {
        self.node_editor.sync_to_model(&self.scene, engine);
    }

    fn handle_message(&mut self, message: &UiMessage) {
        self.entity_panel.handle_message(message);
    }

    fn handle_raw_input(&mut self, device_event: &DeviceEvent, engine: &mut GameEngine) {
        self.camera_controller.handle_raw_input(&self.scene, device_event, engine);

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

    fn handle_input(&mut self, window_event: &WindowEvent, engine: &mut GameEngine) {
        self.camera_controller.handle_input(window_event, engine);

        match window_event {
            &WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    if let Some(current_im) = self.current_interaction_mode {
                        match &mut self.interaction_modes[current_im] {
                            InteractionMode::Move(move_mode) => {
                                match state {
                                    ElementState::Pressed => {
                                        move_mode.on_left_mouse_button_down(&self.scene, &mut self.camera_controller, self.node_editor.node, engine);
                                    }
                                    ElementState::Released => {
                                        move_mode.on_left_mouse_button_up(&self.scene, engine);
                                    }
                                }
                            }
                            InteractionMode::Scale(scale_mode) => {
                                match state {
                                    ElementState::Pressed => {
                                        scale_mode.on_left_mouse_button_down(&self.scene, &mut self.camera_controller, self.node_editor.node, engine);
                                    }
                                    ElementState::Released => {
                                        scale_mode.on_left_mouse_button_up(&self.scene, engine);
                                    }
                                }
                            }
                            InteractionMode::Rotate(rotate_mode) => {
                                match state {
                                    ElementState::Pressed => {
                                        rotate_mode.on_left_mouse_button_down(&self.scene, &mut self.camera_controller, self.node_editor.node, engine);
                                    }
                                    ElementState::Released => {
                                        rotate_mode.on_left_mouse_button_up(&self.scene, engine);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            &WindowEvent::KeyboardInput { input, .. } => {
                if input.state == ElementState::Pressed {
                    if let Some(keycode) = input.virtual_keycode {
                        match keycode {
                            VirtualKeyCode::Y => {
                                self.message_sender.send(Message::Redo).unwrap();
                            }
                            VirtualKeyCode::Z => {
                                self.message_sender.send(Message::Undo).unwrap();
                            }
                            VirtualKeyCode::Key1 => {
                                self.set_interaction_mode(Some(0), engine)
                            }
                            VirtualKeyCode::Key2 => {
                                self.set_interaction_mode(Some(1), engine)
                            }
                            VirtualKeyCode::Key3 => {
                                self.set_interaction_mode(Some(2), engine)
                            }
                            _ => ()
                        }
                    }
                }
            }
            _ => ()
        }
    }

    fn undo_command(&mut self, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[self.scene.scene];
        let graph = &mut scene.graph;

        if let Some(command) = self.command_stack.undo() {
            println!("Undo command {:?}", command);

            match command {
                Command::CreateNode(create_node_command) => create_node_command.revert(graph),
                Command::MoveNode(move_node_command) => move_node_command.revert(graph),
                Command::ScaleNode(scale_node_command) => scale_node_command.revert(graph),
                Command::RotateNode(rotate_node_command) => rotate_node_command.revert(graph),
                Command::ChangeSelection(change_selection) => {
                    // Just re-cast message so every system will handle it correctly.
                    self.message_sender
                        .send(Message::SetSelection(change_selection.revert()))
                        .unwrap();
                }
            }
        }
    }

    fn redo_command(&mut self, engine: &mut GameEngine) {
        if let Some(command) = self.command_stack.redo() {
            println!("Redo command {:?}", command);
            execute_command(&self.scene, engine, command, self.message_sender.clone());
        }
    }

    fn add_command(&mut self, engine: &mut GameEngine, mut command: Command) {
        execute_command(&self.scene, engine, &mut command, self.message_sender.clone());
        let scene = &mut engine.scenes[self.scene.scene];
        let graph = &mut scene.graph;
        let dropped_commands = self.command_stack.add_command(command);
        for command in dropped_commands {
            println!("Finalizing command {:?}", command);
            match command {
                Command::CreateNode(create_node_command) => create_node_command.finalize(graph),
                _ => ()
            }
        }
    }

    fn update(&mut self, engine: &mut GameEngine, dt: f32) {
        self.camera_controller.update(&self.scene, engine, dt);

        if let Some(mode) = self.current_interaction_mode {
            self.interaction_modes[mode].update(&self.scene, engine);
        }

        while let Ok(message) = self.message_receiver.try_recv() {
            for mode in &mut self.interaction_modes {
                mode.handle_message(&message);
            }

            match message {
                Message::ExecuteCommand(command) => {
                    println!("Executing command: {:?}", &command);
                    self.add_command(engine, command)
                }
                Message::Undo => self.undo_command(engine),
                Message::Redo => self.redo_command(engine),
                Message::SetSelection(node) => self.node_editor.node = node,
                Message::SaveScene(path) => {
                    let scene = &mut engine.scenes[self.scene.scene];
                    let editor_root = self.scene.root;
                    let mut pure_scene = scene.clone(&mut |node, _| node != editor_root);
                    let mut visitor = Visitor::new();
                    pure_scene.visit("Scene", &mut visitor).unwrap();
                    visitor.save_binary(&path).unwrap();
                }
                Message::LoadScene(path) => {

                }
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_title("rusty editor")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop).unwrap();

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

                    editor.sync_to_model(&mut engine);
                    editor.update(&mut engine, dt);
                }

                while let Some(ui_message) = engine.user_interface.poll_message() {
                    editor.handle_message(&ui_message);
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
                        engine.renderer.set_frame_size(dbg!(size.into()));
                    }
                    _ => ()
                }

                if let Some(os_event) = translate_event(&event) {
                    engine.user_interface.process_os_event(&os_event);
                }

                editor.handle_input(&event, &mut engine);
            }
            Event::DeviceEvent { event, .. } => {
                editor.handle_raw_input(&event, &mut engine);
            }
            _ => *control_flow = ControlFlow::Poll,
        }
    });
}