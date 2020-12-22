#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]

extern crate rg3d;
#[macro_use]
extern crate lazy_static;

pub mod asset;
pub mod camera;
pub mod command;
pub mod configurator;
pub mod gui;
pub mod interaction;
pub mod light;
pub mod log;
pub mod menu;
pub mod physics;
pub mod preview;
pub mod scene;
pub mod settings;
pub mod sidebar;
pub mod world_outliner;

use crate::scene::{make_delete_selection_command, SetParticleSystemTextureCommand};
use crate::{
    asset::{AssetBrowser, AssetKind},
    camera::CameraController,
    command::{CommandStack, CommandStackViewer},
    configurator::Configurator,
    gui::{BuildContext, EditorUiMessage, EditorUiNode, UiMessage, UiNode},
    interaction::{
        InteractionMode, InteractionModeKind, MoveInteractionMode, RotateInteractionMode,
        ScaleInteractionMode, SelectInteractionMode,
    },
    light::LightPanel,
    log::Log,
    menu::{Menu, MenuContext},
    physics::Physics,
    scene::{
        AddNodeCommand, CommandGroup, EditorScene, LoadModelCommand, SceneCommand, SceneContext,
        SetMeshTextureCommand, SetSpriteTextureCommand,
    },
    sidebar::SideBar,
    world_outliner::WorldOutliner,
};
use rg3d::{
    core::{
        algebra::Vector2, color::Color, math::aabb::AxisAlignedBoundingBox, pool::Handle,
        scope_profile,
    },
    engine::resource_manager::ResourceManager,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::ButtonBuilder,
        canvas::CanvasBuilder,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        draw,
        file_browser::{FileSelectorBuilder, Filter},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, FileSelectorMessage, ImageMessage, KeyCode, MessageBoxMessage,
            MessageDirection, MouseButton, UiMessageData, WidgetMessage, WindowMessage,
        },
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxResult},
        stack_panel::StackPanelBuilder,
        ttf::Font,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness,
    },
    resource::texture::{Texture, TextureKind, TextureState},
    scene::{base::BaseBuilder, graph::Graph, node::Node, Scene, SceneDrawingContext},
    utils::{into_gui_texture, translate_cursor_icon, translate_event},
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    time::Instant,
};

type GameEngine = rg3d::engine::Engine<EditorUiMessage, EditorUiNode>;

lazy_static! {
    /// When editor starting, it remembers the path from where it was launched.
    /// Working directory can be changed multiple time during runtime, but we
    /// load some resources (images mostly) from editors resource folder.
    static ref STARTUP_WORKING_DIR: Mutex<PathBuf> = Mutex::new(std::env::current_dir().unwrap());
}

pub fn load_image<P: AsRef<Path>>(
    path: P,
    resource_manager: ResourceManager,
) -> Option<draw::SharedTexture> {
    if let Ok(absolute_path) = STARTUP_WORKING_DIR
        .lock()
        .unwrap()
        .join(path)
        .canonicalize()
    {
        Some(into_gui_texture(
            resource_manager.request_texture(&absolute_path),
        ))
    } else {
        None
    }
}

pub struct ScenePreview {
    frame: Handle<UiNode>,
    window: Handle<UiNode>,
    last_mouse_pos: Option<Vector2<f32>>,
    click_mouse_pos: Option<Vector2<f32>>,
    selection_frame: Handle<UiNode>,
    // Side bar stuff
    select_mode: Handle<UiNode>,
    move_mode: Handle<UiNode>,
    rotate_mode: Handle<UiNode>,
    scale_mode: Handle<UiNode>,
    sender: Sender<Message>,
}

pub fn make_relative_path<P: AsRef<Path>>(path: P) -> PathBuf {
    // Strip working directory from file name.
    let relative_path = path
        .as_ref()
        .canonicalize()
        .unwrap()
        .strip_prefix(std::env::current_dir().unwrap().canonicalize().unwrap())
        .unwrap()
        .to_owned();

    rg3d::core::replace_slashes(relative_path)
}

impl ScenePreview {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>) -> Self {
        let ctx = &mut engine.user_interface.build_ctx();

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
                            frame = ImageBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(1)
                                    .with_allow_drop(true),
                            )
                            .with_flip(true)
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
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(load_image(
                                                "resources/select.png",
                                                engine.resource_manager.clone(),
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
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(load_image(
                                                "resources/move_arrow.png",
                                                engine.resource_manager.clone(),
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
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_width(32.0)
                                                    .with_height(32.0),
                                            )
                                            .with_opt_texture(load_image(
                                                "resources/rotate_arrow.png",
                                                engine.resource_manager.clone(),
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
                                            .with_texture(into_gui_texture(
                                                engine
                                                    .resource_manager
                                                    .request_texture("resources/scale_arrow.png"),
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
    fn handle_ui_message(&mut self, message: &UiMessage) {
        if let UiMessageData::Button(msg) = &message.data() {
            if let ButtonMessage::Click = msg {
                if message.destination() == self.scale_mode {
                    self.sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                        .unwrap();
                } else if message.destination() == self.rotate_mode {
                    self.sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                        .unwrap();
                } else if message.destination() == self.move_mode {
                    self.sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Move))
                        .unwrap();
                } else if message.destination() == self.select_mode {
                    self.sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Select))
                        .unwrap();
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    SelectionChanged,
    SyncToModel,
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    CloseScene,
    SetInteractionMode(InteractionModeKind),
    Log(String),
    Configure {
        working_directory: PathBuf,
        textures_path: PathBuf,
    },
    NewScene,
    Exit {
        force: bool,
    },
}

pub fn make_scene_file_filter() -> Rc<RefCell<Filter>> {
    Rc::new(RefCell::new(|p: &Path| {
        if let Some(ext) = p.extension() {
            ext.to_string_lossy().as_ref() == "rgs"
        } else {
            p.is_dir()
        }
    }))
}

pub fn make_save_file_selector(ctx: &mut BuildContext) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::Text("Save Scene As".into()))
            .open(false),
    )
    .with_path("./")
    .with_filter(make_scene_file_filter())
    .build(ctx)
}

struct Editor {
    sidebar: SideBar,
    scene: Option<EditorScene>,
    command_stack: CommandStack<SceneCommand>,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    interaction_modes: Vec<Box<dyn InteractionMode>>,
    current_interaction_mode: Option<InteractionModeKind>,
    world_outliner: WorldOutliner,
    root_grid: Handle<UiNode>,
    preview: ScenePreview,
    asset_browser: AssetBrowser,
    exit_message_box: Handle<UiNode>,
    save_file_selector: Handle<UiNode>,
    light_panel: LightPanel,
    menu: Menu,
    exit: bool,
    configurator: Configurator,
    log: Log,
    command_stack_viewer: CommandStackViewer,
    validation_message_box: Handle<UiNode>,
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        *rg3d::gui::DEFAULT_FONT.0.lock().unwrap() =
            Font::from_file("resources/arial.ttf", 14.0, Font::default_char_set()).unwrap();

        let configurator = Configurator::new(
            message_sender.clone(),
            &mut engine.user_interface.build_ctx(),
        );
        engine
            .user_interface
            .send_message(WindowMessage::open_modal(
                configurator.window,
                MessageDirection::ToWidget,
                true,
            ));

        let preview = ScenePreview::new(engine, message_sender.clone());
        let asset_browser = AssetBrowser::new(engine);
        let menu = Menu::new(engine, message_sender.clone());
        let light_panel = LightPanel::new(engine);

        let ctx = &mut engine.user_interface.build_ctx();
        let node_editor =
            SideBar::new(ctx, message_sender.clone(), engine.resource_manager.clone());
        let world_outliner = WorldOutliner::new(ctx, message_sender.clone());
        let command_stack_viewer =
            CommandStackViewer::new(ctx, engine.resource_manager.clone(), message_sender.clone());
        let log = Log::new(ctx);

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
                                        .with_content(TileContent::HorizontalTiles {
                                            splitter: 0.66,
                                            tiles: [
                                                TileBuilder::new(WidgetBuilder::new())
                                                    .with_content(TileContent::HorizontalTiles {
                                                        splitter: 0.80,
                                                        tiles: [
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    asset_browser.window,
                                                                ))
                                                                .build(ctx),
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    command_stack_viewer.window,
                                                                ))
                                                                .build(ctx),
                                                        ],
                                                    })
                                                    .build(ctx),
                                                TileBuilder::new(WidgetBuilder::new())
                                                    .with_content(TileContent::Window(log.window))
                                                    .build(ctx),
                                            ],
                                        })
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

        let save_file_selector = make_save_file_selector(ctx);

        let exit_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(100.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::Text("Unsaved changes".to_owned())),
        )
        .with_text("There are unsaved changes. Do you wish to save them before exit?")
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        let validation_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::Text("Validation failed!".to_owned())),
        )
        .with_buttons(MessageBoxButtons::Ok)
        .build(ctx);

        let mut editor = Self {
            sidebar: node_editor,
            preview,
            scene: None,
            command_stack: CommandStack::new(false),
            message_sender,
            message_receiver,
            interaction_modes: Default::default(),
            current_interaction_mode: None,
            world_outliner,
            root_grid,
            menu,
            exit: false,
            asset_browser,
            exit_message_box,
            save_file_selector,
            configurator,
            log,
            light_panel,
            command_stack_viewer,
            validation_message_box,
        };

        editor.set_interaction_mode(Some(InteractionModeKind::Move), engine);

        editor
    }

    fn set_scene(&mut self, engine: &mut GameEngine, mut scene: Scene, path: Option<PathBuf>) {
        if let Some(previous_editor_scene) = self.scene.as_ref() {
            engine.scenes.remove(previous_editor_scene.scene);
        }
        self.scene = None;
        self.sync_to_model(engine);
        poll_ui_messages(self, engine);

        // Disable binder so we'll have full control over node's transform even if
        // it has a physical body.
        scene.physics_binder.enabled = false;

        scene.render_target = Some(Texture::new_render_target(0, 0));
        engine.user_interface.send_message(ImageMessage::texture(
            self.preview.frame,
            MessageDirection::ToWidget,
            Some(into_gui_texture(scene.render_target.clone().unwrap())),
        ));

        let root = BaseBuilder::new().build(&mut scene.graph);

        let graph = &mut scene.graph;
        let camera_controller = CameraController::new(graph, root);

        let editor_scene = EditorScene {
            path,
            root,
            camera_controller,
            physics: Physics::new(&scene),
            scene: engine.scenes.add(scene),
            selection: Default::default(),
            clipboard: Default::default(),
        };

        self.interaction_modes = vec![
            Box::new(SelectInteractionMode::new(
                self.preview.frame,
                self.preview.selection_frame,
                self.message_sender.clone(),
            )),
            Box::new(MoveInteractionMode::new(
                &editor_scene,
                engine,
                self.message_sender.clone(),
            )),
            Box::new(ScaleInteractionMode::new(
                &editor_scene,
                engine,
                self.message_sender.clone(),
            )),
            Box::new(RotateInteractionMode::new(
                &editor_scene,
                engine,
                self.message_sender.clone(),
            )),
        ];

        self.command_stack = CommandStack::new(false);
        self.scene = Some(editor_scene);

        self.set_interaction_mode(Some(InteractionModeKind::Move), engine);
        self.sync_to_model(engine);

        engine.renderer.flush();
    }

    fn set_interaction_mode(&mut self, mode: Option<InteractionModeKind>, engine: &mut GameEngine) {
        if let Some(editor_scene) = self.scene.as_ref() {
            if self.current_interaction_mode != mode {
                // Deactivate current first.
                if let Some(current_mode) = self.current_interaction_mode {
                    self.interaction_modes[current_mode as usize].deactivate(editor_scene, engine);
                }

                self.current_interaction_mode = mode;
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        self.configurator.handle_ui_message(message, engine);
        self.menu.handle_ui_message(
            message,
            MenuContext {
                engine,
                editor_scene: self.scene.as_mut(),
                sidebar_window: self.sidebar.window,
                world_outliner_window: self.world_outliner.window,
                asset_window: self.asset_browser.window,
                configurator_window: self.configurator.window,
                light_panel: self.light_panel.window,
                log_panel: self.log.window,
            },
        );

        self.log.handle_ui_message(message, engine);
        self.asset_browser.handle_ui_message(message, engine);
        self.command_stack_viewer.handle_ui_message(message);

        if let Some(editor_scene) = self.scene.as_mut() {
            self.sidebar
                .handle_ui_message(message, editor_scene, engine);

            self.world_outliner
                .handle_ui_message(message, &editor_scene, engine);

            self.light_panel
                .handle_ui_message(message, &editor_scene, engine);

            self.preview.handle_ui_message(message);

            let frame_size = engine
                .user_interface
                .node(self.preview.frame)
                .screen_bounds()
                .size;

            if message.destination() == self.preview.frame {
                if let UiMessageData::Widget(msg) = &message.data() {
                    match *msg {
                        WidgetMessage::MouseDown { button, pos, .. } => {
                            engine.user_interface.capture_mouse(self.preview.frame);
                            if button == MouseButton::Left {
                                if let Some(current_im) = self.current_interaction_mode {
                                    let screen_bounds = engine
                                        .user_interface
                                        .node(self.preview.frame)
                                        .screen_bounds();
                                    let rel_pos = pos - screen_bounds.position;

                                    self.preview.click_mouse_pos = Some(rel_pos);

                                    self.interaction_modes[current_im as usize]
                                        .on_left_mouse_button_down(
                                            editor_scene,
                                            engine,
                                            rel_pos,
                                            frame_size,
                                        );
                                }
                            }
                            editor_scene.camera_controller.on_mouse_button_down(button);
                        }
                        WidgetMessage::MouseUp { button, pos, .. } => {
                            engine.user_interface.release_mouse_capture();

                            if button == MouseButton::Left {
                                self.preview.click_mouse_pos = None;
                                if let Some(current_im) = self.current_interaction_mode {
                                    let screen_bounds = engine
                                        .user_interface
                                        .node(self.preview.frame)
                                        .screen_bounds();
                                    let rel_pos = pos - screen_bounds.position;
                                    self.interaction_modes[current_im as usize]
                                        .on_left_mouse_button_up(
                                            editor_scene,
                                            engine,
                                            rel_pos,
                                            frame_size,
                                        );
                                }
                            }
                            editor_scene.camera_controller.on_mouse_button_up(button);
                        }
                        WidgetMessage::MouseWheel { amount, .. } => {
                            let graph = &mut engine.scenes[editor_scene.scene].graph;
                            editor_scene.camera_controller.on_mouse_wheel(amount, graph);
                        }
                        WidgetMessage::MouseMove { pos, .. } => {
                            let last_pos = *self.preview.last_mouse_pos.get_or_insert(pos);
                            let mouse_offset = pos - last_pos;
                            editor_scene.camera_controller.on_mouse_move(mouse_offset);
                            let screen_bounds = engine
                                .user_interface
                                .node(self.preview.frame)
                                .screen_bounds();
                            let rel_pos = pos - screen_bounds.position;

                            if let Some(current_im) = self.current_interaction_mode {
                                self.interaction_modes[current_im as usize].on_mouse_move(
                                    mouse_offset,
                                    rel_pos,
                                    editor_scene.camera_controller.camera,
                                    editor_scene,
                                    engine,
                                    frame_size,
                                );
                            }
                            self.preview.last_mouse_pos = Some(pos);
                        }
                        WidgetMessage::KeyUp(key) => {
                            editor_scene.camera_controller.on_key_up(key);
                        }
                        WidgetMessage::KeyDown(key) => {
                            editor_scene.camera_controller.on_key_down(key);
                            match key {
                                KeyCode::Y => {
                                    if engine.user_interface.keyboard_modifiers().control {
                                        self.message_sender
                                            .send(Message::RedoSceneCommand)
                                            .unwrap();
                                    }
                                }
                                KeyCode::Z => {
                                    if engine.user_interface.keyboard_modifiers().control {
                                        self.message_sender
                                            .send(Message::UndoSceneCommand)
                                            .unwrap();
                                    }
                                }
                                KeyCode::Key1 => self.set_interaction_mode(
                                    Some(InteractionModeKind::Select),
                                    engine,
                                ),
                                KeyCode::Key2 => self
                                    .set_interaction_mode(Some(InteractionModeKind::Move), engine),
                                KeyCode::Key3 => self.set_interaction_mode(
                                    Some(InteractionModeKind::Rotate),
                                    engine,
                                ),
                                KeyCode::Key4 => self
                                    .set_interaction_mode(Some(InteractionModeKind::Scale), engine),
                                KeyCode::L
                                    if engine.user_interface.keyboard_modifiers().control =>
                                {
                                    self.menu
                                        .open_load_file_selector(&mut engine.user_interface);
                                }
                                KeyCode::C
                                    if engine.user_interface.keyboard_modifiers().control =>
                                {
                                    editor_scene.clipboard.clone_selection(
                                        &editor_scene.selection,
                                        &engine.scenes[editor_scene.scene].graph,
                                    );
                                }
                                KeyCode::V
                                    if engine.user_interface.keyboard_modifiers().control =>
                                {
                                    let commands: CommandGroup = CommandGroup::from(
                                        editor_scene
                                            .clipboard
                                            .nodes()
                                            .iter()
                                            .map(|node| {
                                                SceneCommand::AddNode(AddNodeCommand::new(
                                                    node.raw_copy(),
                                                ))
                                            })
                                            .collect::<Vec<SceneCommand>>(),
                                    );
                                    if !commands.is_empty() {
                                        self.message_sender
                                            .send(Message::DoSceneCommand(
                                                SceneCommand::CommandGroup(commands),
                                            ))
                                            .unwrap();
                                    }
                                }
                                KeyCode::Delete => {
                                    if !editor_scene.selection.is_empty() {
                                        self.message_sender
                                            .send(Message::DoSceneCommand(
                                                make_delete_selection_command(editor_scene, engine),
                                            ))
                                            .unwrap();
                                    }
                                }
                                _ => (),
                            }
                        }
                        WidgetMessage::Drop(handle) => {
                            if handle.is_some() {
                                if let UiNode::User(u) = engine.user_interface.node(handle) {
                                    if let EditorUiNode::AssetItem(item) = u {
                                        // Make sure all resources loaded with relative paths only.
                                        // This will make scenes portable.
                                        let relative_path = make_relative_path(&item.path);

                                        match item.kind {
                                            AssetKind::Model => {
                                                // Import model.
                                                self.message_sender
                                                    .send(Message::DoSceneCommand(
                                                        SceneCommand::LoadModel(
                                                            LoadModelCommand::new(relative_path),
                                                        ),
                                                    ))
                                                    .unwrap();
                                            }
                                            AssetKind::Texture => {
                                                let cursor_pos =
                                                    engine.user_interface.cursor_position();
                                                let screen_bounds = engine
                                                    .user_interface
                                                    .node(self.preview.frame)
                                                    .screen_bounds();
                                                let rel_pos = cursor_pos - screen_bounds.position;
                                                let graph =
                                                    &engine.scenes[editor_scene.scene].graph;
                                                let handle = editor_scene.camera_controller.pick(
                                                    rel_pos,
                                                    graph,
                                                    editor_scene.root,
                                                    frame_size,
                                                    false,
                                                    |_, _| true,
                                                );
                                                if handle.is_some() {
                                                    let tex = engine
                                                        .resource_manager
                                                        .request_texture(&relative_path);
                                                    let texture = tex.clone();
                                                    let texture = texture.state();
                                                    if let TextureState::Ok(_) = *texture {
                                                        match &mut engine.scenes[editor_scene.scene]
                                                            .graph[handle]
                                                        {
                                                            Node::Mesh(_) => {
                                                                self.message_sender
                                                                        .send(Message::DoSceneCommand(
                                                                            SceneCommand::SetMeshTexture(
                                                                                SetMeshTextureCommand::new(
                                                                                    handle, tex,
                                                                                ),
                                                                            ),
                                                                        ))
                                                                        .unwrap();
                                                            }
                                                            Node::Sprite(_) => {
                                                                self.message_sender
                                                                        .send(Message::DoSceneCommand(
                                                                            SceneCommand::SetSpriteTexture(
                                                                                SetSpriteTextureCommand::new(
                                                                                    handle, Some(tex),
                                                                                ),
                                                                            ),
                                                                        ))
                                                                        .unwrap();
                                                            }
                                                            Node::ParticleSystem(_) => {
                                                                self.message_sender
                                                                    .send(Message::DoSceneCommand(
                                                                        SceneCommand::SetParticleSystemTexture(
                                                                            SetParticleSystemTextureCommand::new(
                                                                                handle, Some(tex),
                                                                            ),
                                                                        ),
                                                                    ))
                                                                    .unwrap();
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            match &message.data() {
                UiMessageData::MessageBox(msg)
                    if message.destination() == self.exit_message_box =>
                {
                    if let MessageBoxMessage::Close(result) = msg {
                        match result {
                            MessageBoxResult::No => {
                                self.message_sender
                                    .send(Message::Exit { force: true })
                                    .unwrap();
                            }
                            MessageBoxResult::Yes => {
                                if let Some(scene) = self.scene.as_ref() {
                                    if let Some(path) = scene.path.as_ref() {
                                        self.message_sender
                                            .send(Message::SaveScene(path.clone()))
                                            .unwrap();
                                        self.message_sender
                                            .send(Message::Exit { force: true })
                                            .unwrap();
                                    } else {
                                        // Scene wasn't saved yet, open Save As dialog.
                                        engine.user_interface.send_message(
                                            WindowMessage::open_modal(
                                                self.save_file_selector,
                                                MessageDirection::ToWidget,
                                                true,
                                            ),
                                        );
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                UiMessageData::FileSelector(msg)
                    if message.destination() == self.save_file_selector =>
                {
                    if let FileSelectorMessage::Commit(path) = msg {
                        self.message_sender
                            .send(Message::SaveScene(path.clone()))
                            .unwrap();
                        self.message_sender
                            .send(Message::Exit { force: true })
                            .unwrap();
                    }
                }

                _ => (),
            }
        }
    }

    fn sync_to_model(&mut self, engine: &mut GameEngine) {
        self.menu
            .sync_to_model(self.scene.as_ref(), &mut engine.user_interface);

        if let Some(editor_scene) = self.scene.as_mut() {
            self.world_outliner.sync_to_model(editor_scene, engine);
            self.sidebar.sync_to_model(editor_scene, engine);
            self.command_stack_viewer.sync_to_model(
                &mut self.command_stack,
                &SceneContext {
                    scene: &mut engine.scenes[editor_scene.scene],
                    message_sender: self.message_sender.clone(),
                    editor_scene,
                    resource_manager: engine.resource_manager.clone(),
                },
                &mut engine.user_interface,
            )
        } else {
            self.world_outliner.clear(&mut engine.user_interface);
        }
    }

    fn post_update(&mut self, engine: &mut GameEngine) {
        if let Some(scene) = self.scene.as_mut() {
            self.world_outliner.post_update(scene, engine);
        }
    }

    fn update(&mut self, engine: &mut GameEngine, dt: f32) {
        scope_profile!();

        let mut needs_sync = false;

        while let Ok(message) = self.message_receiver.try_recv() {
            self.log.handle_message(&message, engine);

            match message {
                Message::DoSceneCommand(command) => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        self.command_stack.do_command(
                            command,
                            SceneContext {
                                scene: &mut engine.scenes[editor_scene.scene],
                                message_sender: self.message_sender.clone(),
                                editor_scene,
                                resource_manager: engine.resource_manager.clone(),
                            },
                        );
                        needs_sync = true;
                    }
                }
                Message::UndoSceneCommand => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        self.command_stack.undo(SceneContext {
                            scene: &mut engine.scenes[editor_scene.scene],
                            message_sender: self.message_sender.clone(),
                            editor_scene,
                            resource_manager: engine.resource_manager.clone(),
                        });
                        needs_sync = true;
                    }
                }
                Message::RedoSceneCommand => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        self.command_stack.redo(SceneContext {
                            scene: &mut engine.scenes[editor_scene.scene],
                            message_sender: self.message_sender.clone(),
                            editor_scene,
                            resource_manager: engine.resource_manager.clone(),
                        });
                        needs_sync = true;
                    }
                }
                Message::SelectionChanged => {
                    self.world_outliner.sync_selection = true;
                }
                Message::SyncToModel => {
                    needs_sync = true;
                }
                Message::SaveScene(path) => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        match editor_scene.save(path, engine) {
                            Ok(message) => {
                                self.message_sender.send(Message::Log(message)).unwrap();
                            }
                            Err(message) => {
                                self.message_sender
                                    .send(Message::Log(message.clone()))
                                    .unwrap();

                                engine.user_interface.send_message(MessageBoxMessage::open(
                                    self.validation_message_box,
                                    MessageDirection::ToWidget,
                                    None,
                                    Some(message),
                                ));
                            }
                        }
                    }
                }
                Message::LoadScene(scene_path) => {
                    let result = {
                        rg3d::futures::executor::block_on(Scene::from_file(
                            &scene_path,
                            engine.resource_manager.clone(),
                        ))
                    };
                    match result {
                        Ok(scene) => {
                            self.set_scene(engine, scene, Some(scene_path));
                        }
                        Err(e) => {
                            self.message_sender
                                .send(Message::Log(e.to_string()))
                                .unwrap();
                        }
                    }
                }
                Message::SetInteractionMode(mode_kind) => {
                    self.set_interaction_mode(Some(mode_kind), engine);
                }
                Message::Exit { force } => {
                    if force {
                        self.exit = true;
                    } else if self.scene.is_some() {
                        engine.user_interface.send_message(MessageBoxMessage::open(
                            self.exit_message_box,
                            MessageDirection::ToWidget,
                            None,
                            None,
                        ));
                    } else {
                        self.exit = true;
                    }
                }
                Message::Log(msg) => {
                    println!("{}", msg);
                }
                Message::CloseScene => {
                    if let Some(editor_scene) = self.scene.take() {
                        engine.scenes.remove(editor_scene.scene);
                        needs_sync = true;

                        // Preview frame has scene frame texture assigned, it must be cleared explicitly,
                        // otherwise it will show last rendered frame in preview which is not what we want.
                        engine.user_interface.send_message(ImageMessage::texture(
                            self.preview.frame,
                            MessageDirection::ToWidget,
                            None,
                        ));
                    }
                }
                Message::NewScene => {
                    self.set_scene(engine, Scene::new(), None);
                }
                Message::Configure {
                    working_directory,
                    textures_path,
                } => {
                    assert!(self.scene.is_none());

                    self.asset_browser.clear_preview(engine);

                    std::env::set_current_dir(working_directory.clone()).unwrap();

                    let relative_tex_path = make_relative_path(textures_path);

                    engine
                        .resource_manager
                        .state()
                        .set_textures_path(relative_tex_path.clone());

                    engine.resource_manager.state().purge_unused_resources();

                    engine.renderer.flush();

                    self.asset_browser
                        .set_working_directory(engine, &working_directory);

                    self.message_sender
                        .send(Message::Log(format!("New working directory and path to textures were successfully set:\n\tWD: {:?}\n\tTP: {:?}", working_directory, relative_tex_path))).unwrap();

                    needs_sync = true;
                }
            }
        }

        if needs_sync {
            self.sync_to_model(engine);
        }

        if let Some(editor_scene) = self.scene.as_mut() {
            // Adjust camera viewport to size of frame.
            let scene = &mut engine.scenes[editor_scene.scene];

            scene.drawing_context.clear_lines();

            // Create new render target if preview frame has changed its size.
            let (rt_width, rt_height) = if let TextureKind::Rectangle { width, height } =
                scene.render_target.clone().unwrap().data_ref().kind()
            {
                (width, height)
            } else {
                unreachable!();
            };
            if let UiNode::Image(frame) = engine.user_interface.node(self.preview.frame) {
                let frame_size = frame.actual_size();
                if rt_width != frame_size.x as u32 || rt_height != frame_size.y as u32 {
                    let rt = Texture::new_render_target(frame_size.x as u32, frame_size.y as u32);
                    scene.render_target = Some(rt.clone());
                    engine.user_interface.send_message(ImageMessage::texture(
                        self.preview.frame,
                        MessageDirection::ToWidget,
                        Some(into_gui_texture(rt)),
                    ));
                }
            }

            for &node in editor_scene.selection.nodes() {
                let node = &scene.graph[node];
                let aabb = match node {
                    Node::Base(_) => AxisAlignedBoundingBox::unit(),
                    Node::Light(_) => AxisAlignedBoundingBox::unit(),
                    Node::Camera(_) => AxisAlignedBoundingBox::unit(),
                    Node::Mesh(ref mesh) => mesh.bounding_box(),
                    Node::Sprite(_) => AxisAlignedBoundingBox::unit(),
                    Node::ParticleSystem(_) => AxisAlignedBoundingBox::unit(),
                };
                scene
                    .drawing_context
                    .draw_oob(&aabb, node.global_transform(), Color::GREEN);
            }

            fn draw_recursively(
                node: Handle<Node>,
                graph: &Graph,
                ctx: &mut SceneDrawingContext,
                editor_scene: &EditorScene,
            ) {
                // Ignore editor nodes.
                if node == editor_scene.root {
                    return;
                }

                let node = &graph[node];
                if let Node::Base(_) = node {
                    ctx.draw_oob(
                        &AxisAlignedBoundingBox::unit(),
                        node.global_transform(),
                        Color::opaque(255, 127, 39),
                    );
                }

                for &child in node.children() {
                    draw_recursively(child, graph, ctx, editor_scene)
                }
            }

            // Draw pivots.
            draw_recursively(
                scene.graph.get_root(),
                &scene.graph,
                &mut scene.drawing_context,
                editor_scene,
            );

            editor_scene
                .physics
                .draw(&mut scene.drawing_context, &scene.graph);

            let graph = &mut scene.graph;

            editor_scene.camera_controller.update(graph, dt);

            if let Some(mode) = self.current_interaction_mode {
                self.interaction_modes[mode as usize].update(
                    editor_scene,
                    editor_scene.camera_controller.camera,
                    engine,
                );
            }
        }
    }
}

fn poll_ui_messages(editor: &mut Editor, engine: &mut GameEngine) {
    scope_profile!();

    while let Some(ui_message) = engine.user_interface.poll_message() {
        editor.handle_ui_message(&ui_message, engine);
    }
}

fn update(
    editor: &mut Editor,
    engine: &mut GameEngine,
    elapsed_time: &mut f32,
    fixed_timestep: f32,
    clock: &Instant,
) {
    scope_profile!();

    let mut dt = clock.elapsed().as_secs_f32() - *elapsed_time;
    while dt >= fixed_timestep {
        dt -= fixed_timestep;
        *elapsed_time += fixed_timestep;
        engine.update(fixed_timestep);
        editor.update(engine, fixed_timestep);

        poll_ui_messages(editor, engine);

        editor.post_update(engine);
    }

    let window = engine.get_window();
    window.set_cursor_icon(translate_cursor_icon(engine.user_interface.cursor()));
    window.request_redraw();
}

fn main() {
    let event_loop = EventLoop::new();

    let primary_monitor = event_loop.primary_monitor().unwrap();
    let mut monitor_dimensions = primary_monitor.size();
    monitor_dimensions.height = (monitor_dimensions.height as f32 * 0.7) as u32;
    monitor_dimensions.width = (monitor_dimensions.width as f32 * 0.7) as u32;
    let inner_size = monitor_dimensions.to_logical::<f32>(primary_monitor.scale_factor());

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_inner_size(inner_size)
        .with_title("rusty editor")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop, true).unwrap();

    engine.resource_manager.state().set_textures_path("data");

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
            update(
                &mut editor,
                &mut engine,
                &mut elapsed_time,
                fixed_timestep,
                &clock,
            );

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
                    editor
                        .message_sender
                        .send(Message::Exit { force: false })
                        .unwrap();
                }
                WindowEvent::Resized(size) => {
                    engine.renderer.set_frame_size(size.into());
                    engine.user_interface.send_message(WidgetMessage::width(
                        editor.root_grid,
                        MessageDirection::ToWidget,
                        size.width as f32,
                    ));
                    engine.user_interface.send_message(WidgetMessage::height(
                        editor.root_grid,
                        MessageDirection::ToWidget,
                        size.height as f32,
                    ));
                }
                _ => (),
            }

            if let Some(os_event) = translate_event(&event) {
                engine.user_interface.process_os_event(&os_event);
            }
        }
        Event::LoopDestroyed => {
            rg3d::core::profiler::print();
        }
        _ => *control_flow = ControlFlow::Poll,
    });
}
