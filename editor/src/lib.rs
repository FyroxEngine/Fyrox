#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::eval_order_dependence)]
// These are useless.
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::inconsistent_struct_constructor)]

#[macro_use]
extern crate lazy_static;

mod absm;
mod asset;
mod audio;
mod build;
mod camera;
mod command;
mod configurator;
mod curve_editor;
mod gui;
mod inspector;
mod interaction;
mod light;
mod log;
mod material;
mod menu;
mod overlay;
mod preview;
mod scene;
mod scene_viewer;
mod settings;
mod utils;
mod world;

use crate::{
    absm::AbsmEditor,
    asset::{item::AssetItem, item::AssetKind, AssetBrowser},
    audio::AudioPanel,
    build::BuildWindow,
    command::{panel::CommandStackViewer, Command, CommandStack},
    configurator::Configurator,
    curve_editor::CurveEditorWindow,
    inspector::Inspector,
    interaction::{
        move_mode::MoveInteractionMode,
        navmesh::{EditNavmeshMode, NavmeshPanel},
        rotate_mode::RotateInteractionMode,
        scale_mode::ScaleInteractionMode,
        select_mode::SelectInteractionMode,
        terrain::TerrainInteractionMode,
        InteractionMode, InteractionModeKind,
    },
    light::LightPanel,
    log::LogPanel,
    material::MaterialEditor,
    menu::{Menu, MenuContext, Panels},
    overlay::OverlayRenderPass,
    scene::{
        commands::{
            graph::AddModelCommand, make_delete_selection_command, mesh::SetMeshTextureCommand,
            ChangeSelectionCommand, CommandGroup, PasteCommand, SceneCommand, SceneContext,
        },
        is_scene_needs_to_be_saved, EditorScene, Selection,
    },
    scene_viewer::SceneViewer,
    settings::Settings,
    utils::path_fixer::PathFixer,
    world::{graph::selection::GraphSelection, WorldViewer},
};
use fyrox::{
    core::{
        algebra::{Matrix3, Vector2},
        color::Color,
        futures::executor::block_on,
        parking_lot::Mutex,
        pool::{ErasedHandle, Handle},
        scope_profile,
        sstorage::ImmutableString,
    },
    dpi::LogicalSize,
    engine::{resource_manager::ResourceManager, Engine, EngineInitParams, SerializationContext},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        brush::Brush,
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        draw,
        dropdown_list::DropdownListBuilder,
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        message::{KeyCode, MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
        ttf::Font,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface, VerticalAlignment,
    },
    material::{shader::Shader, Material, PropertyValue},
    plugin::PluginConstructor,
    resource::texture::{CompressionOptions, Texture, TextureKind},
    scene::{
        camera::{Camera, Projection},
        mesh::Mesh,
        node::{Node, TypeUuidProvider},
        Scene, SceneLoader,
    },
    utils::{
        into_gui_texture,
        log::{Log, MessageKind},
        translate_cursor_icon, translate_event,
        watcher::FileSystemWatcher,
    },
};
use std::{
    any::TypeId,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
pub const MSG_SYNC_FLAG: u64 = 1;

pub fn send_sync_message(ui: &UserInterface, mut msg: UiMessage) {
    msg.flags = MSG_SYNC_FLAG;
    ui.send_message(msg);
}

type GameEngine = fyrox::engine::Engine;

pub fn load_image(data: &[u8]) -> Option<draw::SharedTexture> {
    Some(into_gui_texture(
        Texture::load_from_memory(data, CompressionOptions::NoCompression, false).ok()?,
    ))
}

lazy_static! {
    static ref GIZMO_SHADER: Shader = {
        Shader::from_str(
            include_str!("../resources/embed/shaders/gizmo.shader",),
            PathBuf::default(),
        )
        .unwrap()
    };
}

pub fn make_color_material(color: Color) -> Arc<Mutex<Material>> {
    let mut material = Material::from_shader(GIZMO_SHADER.clone(), None);
    material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(color),
        )
        .unwrap();
    Arc::new(Mutex::new(material))
}

pub fn set_mesh_diffuse_color(mesh: &mut Mesh, color: Color) {
    for surface in mesh.surfaces() {
        surface
            .material()
            .lock()
            .set_property(
                &ImmutableString::new("diffuseColor"),
                PropertyValue::Color(color),
            )
            .unwrap();
    }
}

pub fn create_terrain_layer_material() -> Arc<Mutex<Material>> {
    let mut material = Material::standard_terrain();
    material
        .set_property(
            &ImmutableString::new("texCoordScale"),
            PropertyValue::Vector2(Vector2::new(10.0, 10.0)),
        )
        .unwrap();
    Arc::new(Mutex::new(material))
}

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    ClearSceneCommandStack,
    SelectionChanged,
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    CloseScene,
    SetInteractionMode(InteractionModeKind),
    Configure {
        working_directory: PathBuf,
    },
    NewScene,
    Exit {
        force: bool,
    },
    OpenSettings,
    OpenMaterialEditor(Arc<Mutex<Material>>),
    ShowInAssetBrowser(PathBuf),
    SetWorldViewerFilter(String),
    LocateObject {
        type_id: TypeId,
        handle: ErasedHandle,
    },
    SelectObject {
        type_id: TypeId,
        handle: ErasedHandle,
    },
    SetEditorCameraProjection(Projection),
    SwitchToPlayMode,
    SwitchToEditMode,
    SwitchMode,
    OpenLoadSceneDialog,
    OpenSaveSceneDialog,
    OpenSaveSceneConfirmationDialog(SaveSceneConfirmationDialogAction),
}

impl Message {
    pub fn do_scene_command<C: Command>(cmd: C) -> Self {
        Self::DoSceneCommand(SceneCommand::new(cmd))
    }
}

pub fn make_scene_file_filter() -> Filter {
    Filter::new(|p: &Path| {
        if let Some(ext) = p.extension() {
            ext.to_string_lossy().as_ref() == "rgs"
        } else {
            p.is_dir()
        }
    })
}

pub fn make_save_file_selector(ctx: &mut BuildContext) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::Text("Save Scene As".into()))
            .open(false),
    )
    .with_mode(FileBrowserMode::Save {
        default_file_name: PathBuf::from("unnamed.rgs"),
    })
    .with_path("./")
    .with_filter(make_scene_file_filter())
    .build(ctx)
}

pub enum Mode {
    Edit,
    Build {
        process: std::process::Child,
    },
    Play {
        process: std::process::Child,
        active: Arc<AtomicBool>,
    },
}

impl Mode {
    pub fn is_edit(&self) -> bool {
        matches!(self, Mode::Edit { .. })
    }
}

pub struct GameLoopData {
    clock: Instant,
    elapsed_time: f32,
}

pub struct StartupData {
    /// Working directory that should be set when starting the editor. If it is empty, then
    /// current working directory won't be changed.
    pub working_directory: PathBuf,

    /// A scene to load at the editor start. If it is empty, no scene will be loaded.
    pub scene: PathBuf,
}

#[derive(Debug)]
pub enum SaveSceneConfirmationDialogAction {
    /// Do nothing.
    None,
    /// Opens `Load Scene` dialog.
    LoadScene,
    /// Immediately creates new scene.
    MakeNewScene,
    /// Closes current scene.
    CloseScene,
}

struct SaveSceneConfirmationDialog {
    save_message_box: Handle<UiNode>,
    action: SaveSceneConfirmationDialogAction,
}

impl SaveSceneConfirmationDialog {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let save_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(100.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::Text("Unsaved changes".to_owned())),
        )
        .with_text("There are unsaved changes. Do you wish to save them before continue?")
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        Self {
            save_message_box,
            action: SaveSceneConfirmationDialogAction::None,
        }
    }

    pub fn open(&mut self, ui: &UserInterface, action: SaveSceneConfirmationDialogAction) {
        ui.send_message(MessageBoxMessage::open(
            self.save_message_box,
            MessageDirection::ToWidget,
            None,
            None,
        ));

        self.action = action;
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        editor_scene: Option<&EditorScene>,
    ) {
        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.save_message_box {
                match result {
                    MessageBoxResult::No => match self.action {
                        SaveSceneConfirmationDialogAction::None => {}
                        SaveSceneConfirmationDialogAction::LoadScene => {
                            sender.send(Message::OpenLoadSceneDialog).unwrap()
                        }
                        SaveSceneConfirmationDialogAction::MakeNewScene => {
                            sender.send(Message::NewScene).unwrap()
                        }
                        SaveSceneConfirmationDialogAction::CloseScene => {
                            sender.send(Message::CloseScene).unwrap()
                        }
                    },
                    MessageBoxResult::Yes => {
                        if let Some(editor_scene) = editor_scene {
                            if let Some(path) = editor_scene.path.clone() {
                                // If the scene was already saved into some file - save it
                                // immediately and perform the requested action.
                                sender.send(Message::SaveScene(path)).unwrap();

                                match self.action {
                                    SaveSceneConfirmationDialogAction::None => {}
                                    SaveSceneConfirmationDialogAction::LoadScene => {
                                        sender.send(Message::OpenLoadSceneDialog).unwrap()
                                    }
                                    SaveSceneConfirmationDialogAction::MakeNewScene => {
                                        sender.send(Message::NewScene).unwrap()
                                    }
                                    SaveSceneConfirmationDialogAction::CloseScene => {
                                        sender.send(Message::CloseScene).unwrap()
                                    }
                                }

                                self.action = SaveSceneConfirmationDialogAction::None;
                            } else {
                                // Otherwise, open save scene dialog and do the action after the
                                // scene was saved.
                                match self.action {
                                    SaveSceneConfirmationDialogAction::None => {}
                                    SaveSceneConfirmationDialogAction::LoadScene
                                    | SaveSceneConfirmationDialogAction::MakeNewScene
                                    | SaveSceneConfirmationDialogAction::CloseScene => {
                                        sender.send(Message::OpenSaveSceneDialog).unwrap()
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn handle_message(&mut self, message: &Message, sender: &Sender<Message>) {
        if let Message::SaveScene(_) = message {
            match std::mem::replace(&mut self.action, SaveSceneConfirmationDialogAction::None) {
                SaveSceneConfirmationDialogAction::None => {}
                SaveSceneConfirmationDialogAction::LoadScene => {
                    sender.send(Message::OpenLoadSceneDialog).unwrap();
                }
                SaveSceneConfirmationDialogAction::MakeNewScene => {
                    sender.send(Message::NewScene).unwrap()
                }
                SaveSceneConfirmationDialogAction::CloseScene => {
                    sender.send(Message::CloseScene).unwrap();
                }
            }
        }
    }
}

pub struct Editor {
    game_loop_data: GameLoopData,
    engine: Engine,
    scene: Option<EditorScene>,
    command_stack: CommandStack,
    message_sender: Sender<Message>,
    message_receiver: Receiver<Message>,
    interaction_modes: Vec<Box<dyn InteractionMode>>,
    current_interaction_mode: Option<InteractionModeKind>,
    world_viewer: WorldViewer,
    root_grid: Handle<UiNode>,
    scene_viewer: SceneViewer,
    asset_browser: AssetBrowser,
    exit_message_box: Handle<UiNode>,
    save_file_selector: Handle<UiNode>,
    save_scene_dialog: SaveSceneConfirmationDialog,
    light_panel: LightPanel,
    menu: Menu,
    exit: bool,
    configurator: Configurator,
    log: LogPanel,
    command_stack_viewer: CommandStackViewer,
    validation_message_box: Handle<UiNode>,
    navmesh_panel: NavmeshPanel,
    settings: Settings,
    path_fixer: PathFixer,
    material_editor: MaterialEditor,
    pub inspector: Inspector,
    curve_editor: CurveEditorWindow,
    audio_panel: AudioPanel,
    #[allow(dead_code)] // TODO
    absm_editor: AbsmEditor,
    mode: Mode,
    build_window: BuildWindow,
}

impl Editor {
    pub fn new(event_loop: &EventLoop<()>, startup_data: Option<StartupData>) -> Self {
        let (log_message_sender, log_message_receiver) = channel();

        Log::add_listener(log_message_sender);

        let inner_size = if let Some(primary_monitor) = event_loop.primary_monitor() {
            let mut monitor_dimensions = primary_monitor.size();
            monitor_dimensions.height = (monitor_dimensions.height as f32 * 0.7) as u32;
            monitor_dimensions.width = (monitor_dimensions.width as f32 * 0.7) as u32;
            monitor_dimensions.to_logical::<f32>(primary_monitor.scale_factor())
        } else {
            LogicalSize::new(1024.0, 768.0)
        };

        let window_builder = fyrox::window::WindowBuilder::new()
            .with_inner_size(inner_size)
            .with_title("Fyroxed")
            .with_resizable(true);

        let serialization_context = Arc::new(SerializationContext::new());
        let mut engine = Engine::new(EngineInitParams {
            window_builder,
            resource_manager: ResourceManager::new(serialization_context.clone()),
            serialization_context,
            events_loop: event_loop,
            vsync: true,
        })
        .unwrap();

        // High-DPI screen support
        let logical_size = engine
            .get_window()
            .inner_size()
            .to_logical(engine.get_window().scale_factor());
        set_ui_scaling(
            &engine.user_interface,
            engine.get_window().scale_factor() as f32,
        );

        let overlay_pass = OverlayRenderPass::new(engine.renderer.pipeline_state());
        engine.renderer.add_render_pass(overlay_pass);

        let (message_sender, message_receiver) = mpsc::channel();

        engine.user_interface.default_font.set(
            Font::from_memory(
                include_bytes!("../resources/embed/arial.ttf").to_vec(),
                14.0,
                Font::default_char_set(),
            )
            .unwrap(),
        );

        let configurator = Configurator::new(
            message_sender.clone(),
            &mut engine.user_interface.build_ctx(),
        );

        let mut settings = Settings::default();

        match Settings::load() {
            Ok(s) => {
                settings = s;

                println!("Editor settings were loaded successfully!");

                match engine
                    .renderer
                    .set_quality_settings(&settings.graphics.quality)
                {
                    Ok(_) => {
                        println!("Graphics settings were applied successfully!");
                    }
                    Err(e) => {
                        println!("Failed to apply graphics settings! Reason: {:?}", e)
                    }
                }
            }
            Err(e) => {
                println!(
                    "Failed to load settings, fallback to default. Reason: {:?}",
                    e
                )
            }
        }

        let scene_viewer = SceneViewer::new(&mut engine, message_sender.clone());
        let asset_browser = AssetBrowser::new(&mut engine);
        let menu = Menu::new(&mut engine, message_sender.clone());
        let light_panel = LightPanel::new(&mut engine);
        let audio_panel = AudioPanel::new(&mut engine);

        let ctx = &mut engine.user_interface.build_ctx();
        let navmesh_panel = NavmeshPanel::new(ctx, message_sender.clone());
        let world_outliner = WorldViewer::new(ctx, message_sender.clone());
        let command_stack_viewer = CommandStackViewer::new(ctx, message_sender.clone());
        let log = LogPanel::new(ctx, log_message_receiver);
        let inspector = Inspector::new(ctx, message_sender.clone());

        let root_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(logical_size.width)
                .with_height(logical_size.height)
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
                                                                    scene_viewer.window(),
                                                                ))
                                                                .build(ctx),
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    inspector.window,
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
                                                    .with_content(TileContent::HorizontalTiles {
                                                        splitter: 0.5,
                                                        tiles: [
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(TileContent::Window(
                                                                    log.window,
                                                                ))
                                                                .build(ctx),
                                                            TileBuilder::new(WidgetBuilder::new())
                                                                .with_content(
                                                                    TileContent::HorizontalTiles {
                                                                        splitter: 0.5,
                                                                        tiles: [
                                                                            TileBuilder::new(
                                                                                WidgetBuilder::new(
                                                                                ),
                                                                            )
                                                                            .with_content(
                                                                                TileContent::Window(
                                                                                    navmesh_panel
                                                                                        .window,
                                                                                ),
                                                                            )
                                                                            .build(ctx),
                                                                            audio_panel.window,
                                                                        ],
                                                                    },
                                                                )
                                                                .build(ctx),
                                                        ],
                                                    })
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

        let path_fixer = PathFixer::new(ctx);

        let curve_editor = CurveEditorWindow::new(ctx);

        let save_scene_dialog = SaveSceneConfirmationDialog::new(ctx);

        let build_window = BuildWindow::new(ctx);

        let absm_editor = AbsmEditor::new(&mut engine, message_sender.clone());

        let material_editor = MaterialEditor::new(&mut engine);

        let mut editor = Self {
            engine,
            navmesh_panel,
            scene_viewer,
            scene: None,
            command_stack: CommandStack::new(false),
            message_sender,
            message_receiver,
            interaction_modes: Default::default(),
            current_interaction_mode: None,
            world_viewer: world_outliner,
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
            settings,
            path_fixer,
            material_editor,
            inspector,
            curve_editor,
            audio_panel,
            save_scene_dialog,
            mode: Mode::Edit,
            game_loop_data: GameLoopData {
                clock: Instant::now(),
                elapsed_time: 0.0,
            },
            absm_editor,
            build_window,
        };

        editor.set_interaction_mode(Some(InteractionModeKind::Move));

        if let Some(data) = startup_data {
            editor
                .message_sender
                .send(Message::Configure {
                    working_directory: if data.working_directory == PathBuf::default() {
                        std::env::current_dir().unwrap()
                    } else {
                        data.working_directory
                    },
                })
                .unwrap();

            if data.scene != PathBuf::default() {
                editor
                    .message_sender
                    .send(Message::LoadScene(data.scene))
                    .unwrap();
            }
        } else {
            // Open configurator as usual.
            editor
                .engine
                .user_interface
                .send_message(WindowMessage::open_modal(
                    editor.configurator.window,
                    MessageDirection::ToWidget,
                    true,
                ));
        }

        editor
    }

    fn set_scene(&mut self, mut scene: Scene, path: Option<PathBuf>) {
        // Discard previous scene.
        if let Some(previous_editor_scene) = self.scene.as_ref() {
            self.engine.scenes.remove(previous_editor_scene.scene);
        }
        self.scene = None;
        self.sync_to_model();
        self.poll_ui_messages();

        for mut interaction_mode in self.interaction_modes.drain(..) {
            interaction_mode.on_drop(&mut self.engine);
        }

        // Setup new one.
        scene.render_target = Some(Texture::new_render_target(0, 0));
        self.scene_viewer
            .set_render_target(&self.engine.user_interface, scene.render_target.clone());

        let editor_scene = EditorScene::from_native_scene(scene, &mut self.engine, path.clone());

        self.interaction_modes = vec![
            Box::new(SelectInteractionMode::new(
                self.scene_viewer.frame(),
                self.scene_viewer.selection_frame(),
                self.message_sender.clone(),
            )),
            Box::new(MoveInteractionMode::new(
                &editor_scene,
                &mut self.engine,
                self.message_sender.clone(),
            )),
            Box::new(ScaleInteractionMode::new(
                &editor_scene,
                &mut self.engine,
                self.message_sender.clone(),
            )),
            Box::new(RotateInteractionMode::new(
                &editor_scene,
                &mut self.engine,
                self.message_sender.clone(),
            )),
            Box::new(EditNavmeshMode::new(
                &editor_scene,
                &mut self.engine,
                self.message_sender.clone(),
            )),
            Box::new(TerrainInteractionMode::new(
                &editor_scene,
                &mut self.engine,
                self.message_sender.clone(),
            )),
        ];

        self.command_stack = CommandStack::new(false);
        self.scene = Some(editor_scene);

        self.set_interaction_mode(Some(InteractionModeKind::Move));

        self.scene_viewer.set_title(
            &self.engine.user_interface,
            format!(
                "Scene Preview - {}",
                path.map_or("Unnamed Scene".to_string(), |p| p
                    .to_string_lossy()
                    .to_string())
            ),
        );
        self.scene_viewer
            .reset_camera_projection(&self.engine.user_interface);
        self.engine.renderer.flush();
    }

    fn set_interaction_mode(&mut self, mode: Option<InteractionModeKind>) {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_ref() {
            if self.current_interaction_mode != mode {
                // Deactivate current first.
                if let Some(current_mode) = self.current_interaction_mode {
                    self.interaction_modes[current_mode as usize].deactivate(editor_scene, engine);
                }

                self.current_interaction_mode = mode;

                // Activate new.
                if let Some(current_mode) = self.current_interaction_mode {
                    self.interaction_modes[current_mode as usize].activate(editor_scene, engine);
                }
            }
        }
    }

    pub fn handle_hotkeys(&mut self, message: &UiMessage) {
        // A message could be handled already somewhere else (for example in a TextBox or any other
        // widget, that handles keyboard input), we must not respond to such messages.
        if message.handled() {
            return;
        }

        let modifiers = self.engine.user_interface.keyboard_modifiers();
        let sender = self.message_sender.clone();
        let engine = &mut self.engine;

        if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            match key {
                KeyCode::Y if modifiers.control => {
                    sender.send(Message::RedoSceneCommand).unwrap();
                }
                KeyCode::Z if modifiers.control => {
                    sender.send(Message::UndoSceneCommand).unwrap();
                }
                KeyCode::Key1 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Select))
                        .unwrap();
                }
                KeyCode::Key2 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Move))
                        .unwrap();
                }
                KeyCode::Key3 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Rotate))
                        .unwrap();
                }
                KeyCode::Key4 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Scale))
                        .unwrap();
                }
                KeyCode::Key5 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Navmesh))
                        .unwrap();
                }
                KeyCode::Key6 => {
                    sender
                        .send(Message::SetInteractionMode(InteractionModeKind::Terrain))
                        .unwrap();
                }
                KeyCode::L if modifiers.control => {
                    sender.send(Message::OpenLoadSceneDialog).unwrap();
                }
                KeyCode::S if modifiers.control => {
                    if let Some(scene) = self.scene.as_ref() {
                        if let Some(path) = scene.path.as_ref() {
                            self.message_sender
                                .send(Message::SaveScene(path.clone()))
                                .unwrap();
                        } else {
                            // Scene wasn't saved yet, open Save As dialog.
                            engine
                                .user_interface
                                .send_message(WindowMessage::open_modal(
                                    self.save_file_selector,
                                    MessageDirection::ToWidget,
                                    true,
                                ));
                        }
                    }
                }
                KeyCode::C if modifiers.control => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        if let Selection::Graph(graph_selection) = &editor_scene.selection {
                            editor_scene.clipboard.fill_from_selection(
                                graph_selection,
                                editor_scene.scene,
                                engine,
                            );
                        }
                    }
                }
                KeyCode::V if modifiers.control => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        if !editor_scene.clipboard.is_empty() {
                            sender
                                .send(Message::do_scene_command(PasteCommand::new()))
                                .unwrap();
                        }
                    }
                }
                KeyCode::N if modifiers.control => {
                    sender.send(Message::NewScene).unwrap();
                }
                KeyCode::Q if modifiers.control => {
                    sender.send(Message::CloseScene).unwrap();
                }
                KeyCode::Delete => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        if !editor_scene.selection.is_empty() {
                            if let Selection::Graph(_) = editor_scene.selection {
                                sender
                                    .send(Message::DoSceneCommand(make_delete_selection_command(
                                        editor_scene,
                                        engine,
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &mut UiMessage) {
        scope_profile!();

        // Prevent infinite message loops.
        if message.has_flags(MSG_SYNC_FLAG) {
            return;
        }

        let engine = &mut self.engine;

        self.absm_editor.handle_ui_message(message, engine);
        self.save_scene_dialog.handle_ui_message(
            message,
            &self.message_sender,
            self.scene.as_ref(),
        );
        self.configurator.handle_ui_message(message, engine);
        self.menu.handle_ui_message(
            message,
            MenuContext {
                engine,
                editor_scene: self.scene.as_mut(),
                panels: Panels {
                    inspector_window: self.inspector.window,
                    world_outliner_window: self.world_viewer.window,
                    asset_window: self.asset_browser.window,
                    light_panel: self.light_panel.window,
                    log_panel: self.log.window,
                    configurator_window: self.configurator.window,
                    path_fixer: self.path_fixer.window,
                    curve_editor: &self.curve_editor,
                    absm_editor: &self.absm_editor,
                },
                settings: &mut self.settings,
            },
        );

        self.build_window
            .handle_ui_message(message, &self.message_sender, &engine.user_interface);
        self.log.handle_ui_message(message, engine);
        self.asset_browser
            .handle_ui_message(message, engine, self.message_sender.clone());
        self.command_stack_viewer.handle_ui_message(message);
        self.curve_editor.handle_ui_message(message, engine);
        self.path_fixer.handle_ui_message(
            message,
            &mut engine.user_interface,
            engine.serialization_context.clone(),
            engine.resource_manager.clone(),
        );
        self.scene_viewer.handle_ui_message(
            message,
            engine,
            self.scene.as_mut(),
            self.current_interaction_mode
                .and_then(|i| self.interaction_modes.get_mut(i as usize)),
            &self.settings,
            &self.mode,
        );

        if let Some(editor_scene) = self.scene.as_mut() {
            self.audio_panel
                .handle_ui_message(message, editor_scene, &self.message_sender, engine);

            self.navmesh_panel.handle_message(
                message,
                editor_scene,
                engine,
                if let Some(edit_mode) = self.interaction_modes
                    [InteractionModeKind::Navmesh as usize]
                    .as_any_mut()
                    .downcast_mut()
                {
                    edit_mode
                } else {
                    unreachable!()
                },
            );

            self.inspector
                .handle_ui_message(message, editor_scene, engine, &self.message_sender);

            if let Some(current_im) = self.current_interaction_mode {
                self.interaction_modes[current_im as usize].handle_ui_message(
                    message,
                    editor_scene,
                    engine,
                );
            }

            self.world_viewer
                .handle_ui_message(message, editor_scene, engine);

            self.light_panel
                .handle_ui_message(message, editor_scene, engine);

            self.material_editor
                .handle_ui_message(message, engine, &self.message_sender);

            if let Some(MessageBoxMessage::Close(result)) = message.data::<MessageBoxMessage>() {
                if message.destination() == self.exit_message_box {
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
                                    engine
                                        .user_interface
                                        .send_message(WindowMessage::open_modal(
                                            self.save_file_selector,
                                            MessageDirection::ToWidget,
                                            true,
                                        ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(FileSelectorMessage::Commit(path)) =
                message.data::<FileSelectorMessage>()
            {
                if message.destination() == self.save_file_selector {
                    self.message_sender
                        .send(Message::SaveScene(path.clone()))
                        .unwrap();
                    self.message_sender
                        .send(Message::Exit { force: true })
                        .unwrap();
                }
            }
        }

        self.handle_hotkeys(message);
    }

    fn set_play_mode(&mut self) {
        if let Some(scene) = self.scene.as_ref() {
            if let Some(path) = scene.path.as_ref().cloned() {
                self.save_current_scene(path.clone());

                match std::process::Command::new("cargo")
                    .stdout(Stdio::piped())
                    .arg("run")
                    .arg("--package")
                    .arg("executor")
                    .arg("--release")
                    .arg("--")
                    .arg("--override-scene")
                    .arg(path)
                    .spawn()
                {
                    Ok(mut process) => {
                        let active = Arc::new(AtomicBool::new(true));

                        // Capture output from child process.
                        let mut stdout = process.stdout.take().unwrap();
                        let reader_active = active.clone();
                        std::thread::spawn(move || {
                            while reader_active.load(Ordering::SeqCst) {
                                for line in BufReader::new(&mut stdout).lines().take(10).flatten() {
                                    Log::info(line);
                                }
                            }
                        });

                        self.mode = Mode::Play { active, process };

                        self.on_mode_changed();
                    }
                    Err(e) => Log::err(format!("Failed to enter play mode: {:?}", e)),
                }
            } else {
                Log::err("Save you scene first!");
            }
        } else {
            Log::err("Cannot enter build mode when there is no scene!");
        }
    }

    fn set_build_mode(&mut self) {
        if let Mode::Edit = self.mode {
            if let Some(scene) = self.scene.as_ref() {
                if scene.path.is_some() {
                    match std::process::Command::new("cargo")
                        .stdout(Stdio::piped())
                        .arg("build")
                        .arg("--package")
                        .arg("executor")
                        .arg("--release")
                        .spawn()
                    {
                        Ok(mut process) => {
                            self.build_window.listen(
                                process.stdout.take().unwrap(),
                                &self.engine.user_interface,
                            );

                            self.mode = Mode::Build { process };

                            self.on_mode_changed();
                        }
                        Err(e) => Log::err(format!("Failed to enter build mode: {:?}", e)),
                    }
                } else {
                    Log::err("Save you scene first!");
                }
            } else {
                Log::err("Cannot enter build mode when there is no scene!");
            }
        } else {
            Log::err("Cannot enter build mode when from non-Edit mode!");
        }
    }

    fn set_editor_mode(&mut self) {
        if let Mode::Play { mut process, .. } | Mode::Build { mut process } =
            std::mem::replace(&mut self.mode, Mode::Edit)
        {
            Log::verify(process.kill());

            self.on_mode_changed();
        }
    }

    fn on_mode_changed(&mut self) {
        let engine = &mut self.engine;
        let ui = &engine.user_interface;
        self.scene_viewer.on_mode_changed(ui, &self.mode);
        self.world_viewer.on_mode_changed(ui, &self.mode);
        self.asset_browser.on_mode_changed(ui, &self.mode);
        self.command_stack_viewer.on_mode_changed(ui, &self.mode);
        self.inspector.on_mode_changed(ui, &self.mode);
        self.audio_panel.on_mode_changed(ui, &self.mode);
        self.navmesh_panel.on_mode_changed(ui, &self.mode);
        self.menu.on_mode_changed(ui, &self.mode);
    }

    fn sync_to_model(&mut self) {
        scope_profile!();

        let engine = &mut self.engine;

        self.menu
            .sync_to_model(self.scene.as_ref(), &mut engine.user_interface);

        if let Some(editor_scene) = self.scene.as_mut() {
            self.scene_viewer.sync_to_model(editor_scene, engine);
            self.inspector.sync_to_model(editor_scene, engine);
            self.navmesh_panel.sync_to_model(editor_scene, engine);
            self.world_viewer.sync_to_model(editor_scene, engine);
            self.material_editor
                .sync_to_model(&mut engine.user_interface);
            self.audio_panel.sync_to_model(editor_scene, engine);
            self.command_stack_viewer.sync_to_model(
                &mut self.command_stack,
                &SceneContext {
                    scene: &mut engine.scenes[editor_scene.scene],
                    message_sender: self.message_sender.clone(),
                    editor_scene,
                    resource_manager: engine.resource_manager.clone(),
                    serialization_context: engine.serialization_context.clone(),
                },
                &mut engine.user_interface,
            )
        } else {
            self.inspector.clear(&engine.user_interface);
            self.world_viewer.clear(&engine.user_interface);
        }
    }

    fn post_update(&mut self) {
        if let Some(scene) = self.scene.as_mut() {
            self.world_viewer.post_update(scene, &mut self.engine);
        }
    }

    fn handle_resize(&mut self) {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_ref() {
            let scene = &mut engine.scenes[editor_scene.scene];

            // Create new render target if preview frame has changed its size.
            if let TextureKind::Rectangle { width, height } =
                scene.render_target.clone().unwrap().data_ref().kind()
            {
                let frame_size = self.scene_viewer.frame_bounds(&engine.user_interface).size;
                if width != frame_size.x as u32 || height != frame_size.y as u32 {
                    scene.render_target = Some(Texture::new_render_target(
                        frame_size.x as u32,
                        frame_size.y as u32,
                    ));
                    self.scene_viewer
                        .set_render_target(&engine.user_interface, scene.render_target.clone());
                }
            }
        }
    }

    fn do_scene_command(&mut self, command: SceneCommand) -> bool {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_mut() {
            self.command_stack.do_command(
                command.into_inner(),
                SceneContext {
                    scene: &mut engine.scenes[editor_scene.scene],
                    message_sender: self.message_sender.clone(),
                    editor_scene,
                    resource_manager: engine.resource_manager.clone(),
                    serialization_context: engine.serialization_context.clone(),
                },
            );

            editor_scene.has_unsaved_changes = true;

            true
        } else {
            false
        }
    }

    fn undo_scene_command(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_mut() {
            self.command_stack.undo(SceneContext {
                scene: &mut engine.scenes[editor_scene.scene],
                message_sender: self.message_sender.clone(),
                editor_scene,
                resource_manager: engine.resource_manager.clone(),
                serialization_context: engine.serialization_context.clone(),
            });

            editor_scene.has_unsaved_changes = true;

            true
        } else {
            false
        }
    }

    fn redo_scene_command(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_mut() {
            self.command_stack.redo(SceneContext {
                scene: &mut engine.scenes[editor_scene.scene],
                message_sender: self.message_sender.clone(),
                editor_scene,
                resource_manager: engine.resource_manager.clone(),
                serialization_context: engine.serialization_context.clone(),
            });

            editor_scene.has_unsaved_changes = true;

            true
        } else {
            false
        }
    }

    fn clear_scene_command_stack(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_mut() {
            self.command_stack.clear(SceneContext {
                scene: &mut engine.scenes[editor_scene.scene],
                message_sender: self.message_sender.clone(),
                editor_scene,
                resource_manager: engine.resource_manager.clone(),
                serialization_context: engine.serialization_context.clone(),
            });
            true
        } else {
            false
        }
    }

    fn save_current_scene(&mut self, path: PathBuf) {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.as_mut() {
            match editor_scene.save(path.clone(), engine) {
                Ok(message) => {
                    self.scene_viewer.set_title(
                        &engine.user_interface,
                        format!("Scene Preview - {}", path.display()),
                    );
                    Log::info(message);

                    editor_scene.has_unsaved_changes = false;
                }
                Err(message) => {
                    Log::err(message.clone());
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

    fn load_scene(&mut self, scene_path: PathBuf) {
        let engine = &mut self.engine;
        let result = {
            block_on(SceneLoader::from_file(
                &scene_path,
                engine.serialization_context.clone(),
            ))
        };
        match result {
            Ok(loader) => {
                let scene = block_on(loader.finish(engine.resource_manager.clone()));

                self.set_scene(scene, Some(scene_path));
            }
            Err(e) => {
                Log::err(e.to_string());
            }
        }
    }

    fn exit(&mut self, force: bool) {
        let engine = &mut self.engine;
        if force {
            self.exit = true;
        } else if is_scene_needs_to_be_saved(self.scene.as_ref()) {
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

    fn close_current_scene(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scene.take() {
            engine.scenes.remove(editor_scene.scene);

            // Preview frame has scene frame texture assigned, it must be cleared explicitly,
            // otherwise it will show last rendered frame in preview which is not what we want.
            self.scene_viewer
                .set_render_target(&engine.user_interface, None);
            // Set default title scene
            self.scene_viewer
                .set_title(&engine.user_interface, "Scene Preview".to_string());

            true
        } else {
            false
        }
    }

    fn create_new_scene(&mut self) {
        let mut scene = Scene::new();

        scene.ambient_lighting_color = Color::opaque(200, 200, 200);

        self.set_scene(scene, None);
    }

    fn configure(&mut self, working_directory: PathBuf) {
        let engine = &mut self.engine;

        assert!(self.scene.is_none());

        self.asset_browser.clear_preview(engine);

        std::env::set_current_dir(working_directory.clone()).unwrap();

        engine
            .get_window()
            .set_title(&format!("Fyroxed: {}", working_directory.to_string_lossy()));

        match FileSystemWatcher::new(&working_directory, Duration::from_secs(1)) {
            Ok(watcher) => {
                engine.resource_manager.state().set_watcher(Some(watcher));
            }
            Err(e) => {
                Log::err(format!("Unable to create resource watcher. Reason {:?}", e));
            }
        }

        engine.resource_manager.state().destroy_unused_resources();

        engine.renderer.flush();

        self.asset_browser
            .set_working_directory(engine, &working_directory);

        Log::info(format!(
            "New working directory was successfully set: {:?}",
            working_directory
        ));
    }

    fn select_object(&mut self, type_id: TypeId, handle: ErasedHandle) {
        if let Some(scene) = self.scene.as_ref() {
            let new_selection = if type_id == TypeId::of::<Node>() {
                Some(Selection::Graph(GraphSelection::single_or_empty(
                    handle.into(),
                )))
            } else {
                None
            };

            if let Some(new_selection) = new_selection {
                self.message_sender
                    .send(Message::DoSceneCommand(SceneCommand::new(
                        ChangeSelectionCommand::new(new_selection, scene.selection.clone()),
                    )))
                    .unwrap()
            }
        }
    }

    fn open_material_editor(&mut self, material: Arc<Mutex<Material>>) {
        let engine = &mut self.engine;

        self.material_editor.set_material(Some(material), engine);

        engine.user_interface.send_message(WindowMessage::open(
            self.material_editor.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    fn poll_ui_messages(&mut self) {
        scope_profile!();

        while let Some(mut ui_message) = self.engine.user_interface.poll_message() {
            self.handle_ui_message(&mut ui_message);
        }
    }

    fn update(&mut self, dt: f32) {
        scope_profile!();

        match self.mode {
            Mode::Play {
                ref mut process,
                ref active,
            } => {
                match process.try_wait() {
                    Ok(status) => {
                        if let Some(status) = status {
                            // Stop reader thread.
                            active.store(false, Ordering::SeqCst);

                            self.mode = Mode::Edit;
                            self.on_mode_changed();

                            Log::info(format!("Game was closed: {:?}", status))
                        }
                    }
                    Err(err) => Log::err(format!("Failed to wait for game process: {:?}", err)),
                }
            }
            Mode::Build { ref mut process } => {
                self.build_window.update(&self.engine.user_interface);

                match process.try_wait() {
                    Ok(status) => {
                        if let Some(status) = status {
                            self.build_window.reset(&self.engine.user_interface);

                            // https://doc.rust-lang.org/cargo/commands/cargo-build.html#exit-status
                            let err_code = 101;
                            let code = status.code().unwrap_or(err_code);
                            if code == err_code {
                                Log::info("Failed to build the game!");
                                self.mode = Mode::Edit;
                                self.on_mode_changed();
                            } else {
                                self.set_play_mode();
                            }
                        }
                    }
                    Err(err) => Log::err(format!("Failed to wait for game process: {:?}", err)),
                }
            }
            _ => {}
        }

        self.absm_editor.update(&mut self.engine);
        self.log.update(&mut self.engine);

        let mut needs_sync = false;

        while let Ok(message) = self.message_receiver.try_recv() {
            self.path_fixer
                .handle_message(&message, &self.engine.user_interface);

            self.save_scene_dialog
                .handle_message(&message, &self.message_sender);

            if let Some(editor_scene) = self.scene.as_ref() {
                self.inspector
                    .handle_message(&message, editor_scene, &mut self.engine);
            }

            match message {
                Message::DoSceneCommand(command) => {
                    needs_sync |= self.do_scene_command(command);
                }
                Message::UndoSceneCommand => {
                    needs_sync |= self.undo_scene_command();
                }
                Message::RedoSceneCommand => {
                    needs_sync |= self.redo_scene_command();
                }
                Message::ClearSceneCommandStack => {
                    needs_sync |= self.clear_scene_command_stack();
                }
                Message::SelectionChanged => {
                    self.world_viewer.sync_selection = true;
                }
                Message::SaveScene(path) => self.save_current_scene(path),
                Message::LoadScene(scene_path) => {
                    self.load_scene(scene_path);
                    needs_sync = true;
                }
                Message::SetInteractionMode(mode_kind) => {
                    self.set_interaction_mode(Some(mode_kind))
                }
                Message::Exit { force } => self.exit(force),
                Message::CloseScene => {
                    needs_sync |= self.close_current_scene();
                }
                Message::NewScene => {
                    self.create_new_scene();
                    needs_sync = true;
                }
                Message::Configure { working_directory } => {
                    self.configure(working_directory);
                    needs_sync = true;
                }
                Message::OpenSettings => {
                    self.menu.file_menu.settings.open(
                        &mut self.engine.user_interface,
                        &self.settings,
                        &self.message_sender,
                    );
                }
                Message::OpenMaterialEditor(material) => self.open_material_editor(material),
                Message::ShowInAssetBrowser(path) => {
                    self.asset_browser
                        .locate_path(&self.engine.user_interface, path);
                }
                Message::SetWorldViewerFilter(filter) => {
                    self.world_viewer
                        .set_filter(filter, &self.engine.user_interface);
                }
                Message::LocateObject { type_id, handle } => {
                    self.world_viewer
                        .try_locate_object(type_id, handle, &self.engine)
                }
                Message::SelectObject { type_id, handle } => {
                    self.select_object(type_id, handle);
                }
                Message::SetEditorCameraProjection(projection) => {
                    if let Some(editor_scene) = self.scene.as_ref() {
                        editor_scene.camera_controller.set_projection(
                            &mut self.engine.scenes[editor_scene.scene].graph,
                            projection,
                        );
                    }
                }
                Message::SwitchMode => match self.mode {
                    Mode::Edit => self.set_build_mode(),
                    _ => self.set_editor_mode(),
                },
                Message::SwitchToPlayMode => self.set_play_mode(),
                Message::SwitchToEditMode => self.set_editor_mode(),
                Message::OpenLoadSceneDialog => {
                    self.menu
                        .open_load_file_selector(&mut self.engine.user_interface);
                }
                Message::OpenSaveSceneDialog => {
                    self.menu
                        .open_save_file_selector(&mut self.engine.user_interface);
                }
                Message::OpenSaveSceneConfirmationDialog(action) => {
                    self.save_scene_dialog
                        .open(&self.engine.user_interface, action);
                }
            }
        }

        if needs_sync {
            self.sync_to_model();
        }

        self.handle_resize();

        if let Some(editor_scene) = self.scene.as_mut() {
            editor_scene.draw_debug(&mut self.engine, &self.settings.debugging);

            let scene = &mut self.engine.scenes[editor_scene.scene];

            let camera = scene.graph[editor_scene.camera_controller.camera].as_camera_mut();

            camera
                .projection_mut()
                .set_z_near(self.settings.graphics.z_near);
            camera
                .projection_mut()
                .set_z_far(self.settings.graphics.z_far);

            let graph = &mut scene.graph;

            editor_scene
                .camera_controller
                .update(graph, &self.settings.camera, dt);

            if let Some(mode) = self.current_interaction_mode {
                self.interaction_modes[mode as usize].update(
                    editor_scene,
                    editor_scene.camera_controller.camera,
                    &mut self.engine,
                );
            }
        }

        self.material_editor.update(&mut self.engine);
        self.asset_browser.update(&mut self.engine);
    }

    pub fn add_game_plugin<P>(&mut self, plugin: P)
    where
        P: PluginConstructor + TypeUuidProvider + 'static,
    {
        self.engine.add_plugin_constructor(plugin)
    }

    pub fn run(mut self, event_loop: EventLoop<()>) -> ! {
        event_loop.run(move |event, _, control_flow| match event {
            Event::MainEventsCleared => {
                update(&mut self, control_flow);

                if self.exit {
                    *control_flow = ControlFlow::Exit;

                    // Kill any active child process on exit.
                    match self.mode {
                        Mode::Edit => {}
                        Mode::Build { ref mut process }
                        | Mode::Play {
                            ref mut process, ..
                        } => {
                            let _ = process.kill();
                        }
                    }
                }
            }
            Event::RedrawRequested(_) => {
                // Temporarily disable cameras in currently edited scene. This is needed to prevent any
                // scene camera to interfere with the editor camera.
                let mut camera_state = Vec::new();
                if let Some(editor_scene) = self.scene.as_ref() {
                    let scene = &mut self.engine.scenes[editor_scene.scene];
                    let has_preview_camera =
                        scene.graph.is_valid_handle(editor_scene.preview_camera);
                    for (handle, camera) in scene.graph.pair_iter_mut().filter_map(|(h, n)| {
                        if has_preview_camera && h != editor_scene.preview_camera
                            || !has_preview_camera && h != editor_scene.camera_controller.camera
                        {
                            n.cast_mut::<Camera>().map(|c| (h, c))
                        } else {
                            None
                        }
                    }) {
                        camera_state.push((handle, camera.is_enabled()));
                        camera.set_enabled(false);
                    }
                }

                self.engine.render().unwrap();

                // Revert state of the cameras.
                if let Some(scene) = self.scene.as_ref() {
                    for (handle, enabled) in camera_state {
                        self.engine.scenes[scene.scene].graph[handle]
                            .as_camera_mut()
                            .set_enabled(enabled);
                    }
                }
            }
            Event::WindowEvent { ref event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        self.message_sender
                            .send(Message::Exit { force: false })
                            .unwrap();
                    }
                    WindowEvent::Resized(size) => {
                        if let Err(e) = self.engine.set_frame_size((*size).into()) {
                            fyrox::utils::log::Log::writeln(
                                MessageKind::Error,
                                format!("Failed to set renderer size! Reason: {:?}", e),
                            );
                        }

                        let logical_size = size.to_logical(self.engine.get_window().scale_factor());
                        self.engine
                            .user_interface
                            .send_message(WidgetMessage::width(
                                self.root_grid,
                                MessageDirection::ToWidget,
                                logical_size.width,
                            ));
                        self.engine
                            .user_interface
                            .send_message(WidgetMessage::height(
                                self.root_grid,
                                MessageDirection::ToWidget,
                                logical_size.height,
                            ));
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        set_ui_scaling(&self.engine.user_interface, *scale_factor as f32);
                    }
                    _ => (),
                }

                if let Some(os_event) = translate_event(event) {
                    self.engine.user_interface.process_os_event(&os_event);
                }
            }
            _ => *control_flow = ControlFlow::Poll,
        });
    }
}

fn set_ui_scaling(ui: &UserInterface, scale: f32) {
    // High-DPI screen support
    ui.send_message(WidgetMessage::render_transform(
        ui.root(),
        MessageDirection::ToWidget,
        Matrix3::new_scaling(scale),
    ));
}

fn update(editor: &mut Editor, control_flow: &mut ControlFlow) {
    scope_profile!();

    let mut dt =
        editor.game_loop_data.clock.elapsed().as_secs_f32() - editor.game_loop_data.elapsed_time;
    while dt >= FIXED_TIMESTEP {
        dt -= FIXED_TIMESTEP;
        editor.game_loop_data.elapsed_time += FIXED_TIMESTEP;

        editor.engine.pre_update(FIXED_TIMESTEP, control_flow);

        editor.update(FIXED_TIMESTEP);

        editor.poll_ui_messages();

        editor.engine.post_update(FIXED_TIMESTEP);

        editor.post_update();

        if dt >= 1.5 * FIXED_TIMESTEP {
            break;
        }
    }

    let window = editor.engine.get_window();
    window.set_cursor_icon(translate_cursor_icon(editor.engine.user_interface.cursor()));
    window.request_redraw();
}
