#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::mixed_read_write_in_expression)]
// These are useless.
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::inconsistent_struct_constructor)]

#[macro_use]
extern crate lazy_static;

pub mod absm;
pub mod animation;
pub mod asset;
pub mod audio;
pub mod build;
pub mod camera;
pub mod command;
pub mod configurator;
pub mod curve_editor;
pub mod gui;
pub mod inspector;
pub mod interaction;
pub mod light;
pub mod log;
pub mod material;
pub mod menu;
pub mod message;
pub mod overlay;
pub mod particle;
pub mod plugin;
pub mod preview;
pub mod scene;
pub mod scene_viewer;
pub mod settings;
pub mod utils;
pub mod world;

use crate::{
    absm::AbsmEditor,
    animation::AnimationEditor,
    asset::{item::AssetItem, item::AssetKind, AssetBrowser},
    audio::{preview::AudioPreviewPanel, AudioPanel},
    build::BuildWindow,
    camera::panel::CameraPreviewControlPanel,
    command::{panel::CommandStackViewer, Command, CommandStack},
    configurator::Configurator,
    curve_editor::CurveEditorWindow,
    inspector::{editors::handle::HandlePropertyEditorMessage, Inspector},
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
    message::MessageSender,
    overlay::OverlayRenderPass,
    particle::ParticleSystemPreviewControlPanel,
    plugin::EditorPlugin,
    scene::{
        commands::{
            graph::AddModelCommand, make_delete_selection_command, mesh::SetMeshTextureCommand,
            ChangeSelectionCommand, CommandGroup, PasteCommand, SceneCommand, SceneContext,
        },
        dialog::NodeRemovalDialog,
        selector::HierarchyNode,
        settings::SceneSettingsWindow,
        EditorScene, Selection,
    },
    scene_viewer::SceneViewer,
    settings::Settings,
    utils::ragdoll::RagdollWizard,
    utils::{doc::DocWindow, path_fixer::PathFixer},
    world::{graph::selection::GraphSelection, WorldViewer},
};
use fyrox::{
    asset::{io::FsResourceIo, manager::ResourceManager},
    core::{
        algebra::{Matrix3, Vector2},
        color::Color,
        futures::executor::block_on,
        log::{Log, MessageKind},
        pool::{ErasedHandle, Handle},
        scope_profile,
        sstorage::ImmutableString,
        visitor::Visitor,
        watcher::FileSystemWatcher,
    },
    dpi::{PhysicalPosition, PhysicalSize},
    engine::{Engine, EngineInitParams, GraphicsContextParams, SerializationContext},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    fxhash::FxHashMap,
    gui::{
        brush::Brush,
        dock::{
            DockingManager, DockingManagerBuilder, DockingManagerMessage, TileBuilder, TileContent,
        },
        draw,
        dropdown_list::DropdownListBuilder,
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        key::HotKey,
        message::{MessageDirection, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
        ttf::Font,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface, VerticalAlignment,
    },
    material::{
        shader::{ShaderResource, ShaderResourceExtension},
        Material, MaterialResource, PropertyValue,
    },
    plugin::PluginConstructor,
    resource::texture::{
        CompressionOptions, TextureImportOptions, TextureKind, TextureMinificationFilter,
        TextureResource, TextureResourceExtension,
    },
    scene::{
        camera::Camera, graph::GraphUpdateSwitches, mesh::Mesh, node::Node, Scene, SceneLoader,
    },
    utils::{into_gui_texture, translate_cursor_icon, translate_event},
    window::{Icon, WindowAttributes},
};
use std::{
    any::TypeId,
    cell::RefCell,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver},
        Arc,
    },
    time::{Duration, Instant},
};

pub use message::Message;

pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
pub const MSG_SYNC_FLAG: u64 = 1;

pub fn send_sync_message(ui: &UserInterface, mut msg: UiMessage) {
    msg.flags = MSG_SYNC_FLAG;
    ui.send_message(msg);
}

pub fn load_image(data: &[u8]) -> Option<draw::SharedTexture> {
    Some(into_gui_texture(
        TextureResource::load_from_memory(
            data,
            TextureImportOptions::default()
                .with_compression(CompressionOptions::NoCompression)
                .with_minification_filter(TextureMinificationFilter::Linear),
        )
        .ok()?,
    ))
}

lazy_static! {
    static ref GIZMO_SHADER: ShaderResource = {
        ShaderResource::from_str(
            include_str!("../resources/embed/shaders/gizmo.shader",),
            PathBuf::default(),
        )
        .unwrap()
    };
}

pub fn make_color_material(color: Color) -> MaterialResource {
    let mut material = Material::from_shader(GIZMO_SHADER.clone(), None);
    material
        .set_property(
            &ImmutableString::new("diffuseColor"),
            PropertyValue::Color(color),
        )
        .unwrap();
    MaterialResource::new(material)
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

pub fn create_terrain_layer_material() -> MaterialResource {
    let mut material = Material::standard_terrain();
    material
        .set_property(
            &ImmutableString::new("texCoordScale"),
            PropertyValue::Vector2(Vector2::new(10.0, 10.0)),
        )
        .unwrap();
    MaterialResource::new(material)
}

#[derive(Debug)]
pub enum BuildProfile {
    Debug,
    Release,
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
    lag: f32,
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
    OpenLoadSceneDialog,
    /// Load specified scene.
    LoadScene(PathBuf),
    /// Immediately creates new scene.
    MakeNewScene,
    /// Closes the specified scene.
    CloseScene(Handle<Scene>),
}

pub struct SaveSceneConfirmationDialog {
    save_message_box: Handle<UiNode>,
    action: SaveSceneConfirmationDialogAction,
    scene: Handle<Scene>,
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
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        Self {
            save_message_box,
            action: SaveSceneConfirmationDialogAction::None,
            scene: Default::default(),
        }
    }

    pub fn open(
        &mut self,
        ui: &UserInterface,
        scene: Handle<Scene>,
        scenes: &SceneContainer,
        action: SaveSceneConfirmationDialogAction,
    ) {
        self.scene = scene;
        self.action = action;

        if let Some(entry) = scenes.entry_by_scene_handle(self.scene) {
            ui.send_message(MessageBoxMessage::open(
                self.save_message_box,
                MessageDirection::ToWidget,
                None,
                Some(format!(
                    "There are unsaved changes in the {} scene. \
                Do you wish to save them before continue?",
                    entry.editor_scene.name(),
                )),
            ));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        scenes: &SceneContainer,
    ) {
        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.save_message_box {
                match result {
                    MessageBoxResult::No => match self.action {
                        SaveSceneConfirmationDialogAction::None => {}
                        SaveSceneConfirmationDialogAction::OpenLoadSceneDialog => {
                            sender.send(Message::OpenLoadSceneDialog)
                        }
                        SaveSceneConfirmationDialogAction::MakeNewScene => {
                            sender.send(Message::NewScene)
                        }
                        SaveSceneConfirmationDialogAction::CloseScene(scene) => {
                            sender.send(Message::CloseScene(scene))
                        }
                        SaveSceneConfirmationDialogAction::LoadScene(ref path) => {
                            sender.send(Message::LoadScene(path.clone()))
                        }
                    },
                    MessageBoxResult::Yes => {
                        if let Some(entry) = scenes.entry_by_scene_handle(self.scene) {
                            if let Some(path) = entry.editor_scene.path.clone() {
                                // If the scene was already saved into some file - save it
                                // immediately and perform the requested action.
                                sender.send(Message::SaveScene {
                                    scene: self.scene,
                                    path,
                                });

                                match self.action {
                                    SaveSceneConfirmationDialogAction::None => {}
                                    SaveSceneConfirmationDialogAction::OpenLoadSceneDialog => {
                                        sender.send(Message::OpenLoadSceneDialog)
                                    }
                                    SaveSceneConfirmationDialogAction::MakeNewScene => {
                                        sender.send(Message::NewScene)
                                    }
                                    SaveSceneConfirmationDialogAction::CloseScene(scene) => {
                                        sender.send(Message::CloseScene(scene))
                                    }
                                    SaveSceneConfirmationDialogAction::LoadScene(ref path) => {
                                        sender.send(Message::LoadScene(path.clone()))
                                    }
                                }

                                self.action = SaveSceneConfirmationDialogAction::None;
                            } else {
                                // Otherwise, open save scene dialog and do the action after the
                                // scene was saved.
                                match self.action {
                                    SaveSceneConfirmationDialogAction::None => {}
                                    SaveSceneConfirmationDialogAction::OpenLoadSceneDialog
                                    | SaveSceneConfirmationDialogAction::LoadScene(_)
                                    | SaveSceneConfirmationDialogAction::MakeNewScene
                                    | SaveSceneConfirmationDialogAction::CloseScene(_) => {
                                        sender.send(Message::OpenSaveSceneDialog)
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

    fn handle_message(&mut self, message: &Message, sender: &MessageSender) {
        if let Message::SaveScene { scene, .. } = message {
            if *scene == self.scene {
                self.scene = Handle::NONE;

                match std::mem::replace(&mut self.action, SaveSceneConfirmationDialogAction::None) {
                    SaveSceneConfirmationDialogAction::None => {}
                    SaveSceneConfirmationDialogAction::OpenLoadSceneDialog => {
                        sender.send(Message::OpenLoadSceneDialog);
                    }
                    SaveSceneConfirmationDialogAction::MakeNewScene => {
                        sender.send(Message::NewScene)
                    }
                    SaveSceneConfirmationDialogAction::CloseScene(scene) => {
                        sender.send(Message::CloseScene(scene));
                    }
                    SaveSceneConfirmationDialogAction::LoadScene(path) => {
                        sender.send(Message::LoadScene(path))
                    }
                }
            }
        }
    }
}

pub struct EditorSceneEntry {
    pub editor_scene: EditorScene,
    pub command_stack: CommandStack,
    pub interaction_modes: Vec<Box<dyn InteractionMode>>,
    pub current_interaction_mode: Option<InteractionModeKind>,
}

impl EditorSceneEntry {
    fn set_interaction_mode(&mut self, engine: &mut Engine, mode: Option<InteractionModeKind>) {
        if self.current_interaction_mode != mode {
            // Deactivate current first.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode as usize]
                    .deactivate(&self.editor_scene, engine);
            }

            self.current_interaction_mode = mode;

            // Activate new.
            if let Some(current_mode) = self.current_interaction_mode {
                self.interaction_modes[current_mode as usize].activate(&self.editor_scene, engine);
            }
        }
    }

    fn on_drop(&mut self, engine: &mut Engine) {
        for mut interaction_mode in self.interaction_modes.drain(..) {
            interaction_mode.on_drop(engine);
        }
    }
}

#[derive(Default)]
pub struct SceneContainer {
    scenes: Vec<EditorSceneEntry>,
    current_scene: Option<usize>,
}

impl SceneContainer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn current_scene_entry_ref(&self) -> Option<&EditorSceneEntry> {
        self.current_scene.and_then(|i| self.scenes.get(i))
    }

    pub fn current_scene_entry_mut(&mut self) -> Option<&mut EditorSceneEntry> {
        self.current_scene.and_then(|i| self.scenes.get_mut(i))
    }

    pub fn current_editor_scene_ref(&self) -> Option<&EditorScene> {
        self.current_scene_entry_ref().map(|e| &e.editor_scene)
    }

    pub fn current_editor_scene_mut(&mut self) -> Option<&mut EditorScene> {
        self.current_scene_entry_mut().map(|e| &mut e.editor_scene)
    }

    pub fn first_unsaved_scene(&self) -> Option<&EditorSceneEntry> {
        self.scenes.iter().find(|s| s.editor_scene.need_save())
    }

    pub fn unsaved_scene_count(&self) -> usize {
        self.scenes
            .iter()
            .filter(|s| s.editor_scene.need_save())
            .count()
    }

    pub fn len(&self) -> usize {
        self.scenes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EditorSceneEntry> {
        self.scenes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut EditorSceneEntry> {
        self.scenes.iter_mut()
    }

    pub fn try_get(&self, index: usize) -> Option<&EditorSceneEntry> {
        self.scenes.get(index)
    }

    pub fn try_get_mut(&mut self, index: usize) -> Option<&mut EditorSceneEntry> {
        self.scenes.get_mut(index)
    }

    pub fn current_scene_index(&self) -> Option<usize> {
        self.current_scene
    }

    pub fn set_current_scene(&mut self, scene: Handle<Scene>) -> bool {
        if let Some(index) = self
            .scenes
            .iter()
            .position(|e| e.editor_scene.scene == scene)
        {
            self.current_scene = Some(index);

            true
        } else {
            false
        }
    }

    pub fn entry_by_scene_handle(&self, handle: Handle<Scene>) -> Option<&EditorSceneEntry> {
        self.scenes.iter().find(|e| e.editor_scene.scene == handle)
    }

    pub fn entry_by_scene_handle_mut(
        &mut self,
        handle: Handle<Scene>,
    ) -> Option<&mut EditorSceneEntry> {
        self.scenes
            .iter_mut()
            .find(|e| e.editor_scene.scene == handle)
    }

    pub fn add_scene_and_select(
        &mut self,
        scene: Scene,
        path: Option<PathBuf>,
        engine: &mut Engine,
        settings: &Settings,
        message_sender: MessageSender,
        scene_viewer: &SceneViewer,
    ) {
        self.current_scene = Some(self.scenes.len());

        let editor_scene = EditorScene::from_native_scene(scene, engine, path, settings);

        let mut entry = EditorSceneEntry {
            interaction_modes: vec![
                Box::new(SelectInteractionMode::new(
                    scene_viewer.frame(),
                    scene_viewer.selection_frame(),
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
                Box::new(EditNavmeshMode::new(
                    &editor_scene,
                    engine,
                    message_sender.clone(),
                )),
                Box::new(TerrainInteractionMode::new(
                    &editor_scene,
                    engine,
                    message_sender,
                )),
            ],
            editor_scene,
            command_stack: CommandStack::new(false),
            current_interaction_mode: None,
        };

        entry.set_interaction_mode(engine, Some(InteractionModeKind::Move));

        self.scenes.push(entry);
    }

    pub fn take_scene(&mut self, scene: Handle<Scene>) -> Option<EditorSceneEntry> {
        let scene = self
            .scenes
            .iter()
            .position(|e| e.editor_scene.scene == scene)
            .map(|i| self.scenes.remove(i));
        self.current_scene = if self.scenes.is_empty() {
            None
        } else {
            // TODO: Maybe set it to the previous one?
            Some(0)
        };
        scene
    }
}

pub struct UpdateLoopState(u32);

impl Default for UpdateLoopState {
    fn default() -> Self {
        // Run at least a second from the start to ensure that all OS-specific stuff was done.
        Self(60)
    }
}

impl UpdateLoopState {
    fn request_update_in_next_frame(&mut self) {
        if !self.is_warming_up() {
            self.0 = 2;
        }
    }

    fn request_update_in_current_frame(&mut self) {
        if !self.is_warming_up() {
            self.0 = 1;
        }
    }

    fn is_warming_up(&self) -> bool {
        self.0 > 2
    }

    fn decrease_counter(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }

    fn is_suspended(&self) -> bool {
        self.0 == 0
    }
}

pub struct Editor {
    pub game_loop_data: GameLoopData,
    pub scenes: SceneContainer,
    pub message_sender: MessageSender,
    pub message_receiver: Receiver<Message>,
    pub world_viewer: WorldViewer,
    pub root_grid: Handle<UiNode>,
    pub scene_viewer: SceneViewer,
    pub asset_browser: AssetBrowser,
    pub exit_message_box: Handle<UiNode>,
    pub save_file_selector: Handle<UiNode>,
    pub save_scene_dialog: SaveSceneConfirmationDialog,
    pub light_panel: LightPanel,
    pub menu: Menu,
    pub exit: bool,
    pub configurator: Configurator,
    pub log: LogPanel,
    pub command_stack_viewer: CommandStackViewer,
    pub validation_message_box: Handle<UiNode>,
    pub navmesh_panel: NavmeshPanel,
    pub settings: Settings,
    pub path_fixer: PathFixer,
    pub material_editor: MaterialEditor,
    pub inspector: Inspector,
    pub curve_editor: CurveEditorWindow,
    pub audio_panel: AudioPanel,
    pub absm_editor: AbsmEditor,
    pub mode: Mode,
    pub build_window: BuildWindow,
    pub build_profile: BuildProfile,
    pub scene_settings: SceneSettingsWindow,
    pub animation_editor: AnimationEditor,
    pub particle_system_control_panel: ParticleSystemPreviewControlPanel,
    pub camera_control_panel: CameraPreviewControlPanel,
    pub overlay_pass: Rc<RefCell<OverlayRenderPass>>,
    pub audio_preview_panel: AudioPreviewPanel,
    pub doc_window: DocWindow,
    pub docking_manager: Handle<UiNode>,
    pub node_removal_dialog: NodeRemovalDialog,
    pub engine: Engine,
    pub plugins: Vec<Option<Box<dyn EditorPlugin>>>,
    pub focused: bool,
    pub update_loop_state: UpdateLoopState,
    pub is_suspended: bool,
    pub ragdoll_wizard: RagdollWizard,
}

impl Editor {
    pub fn new(event_loop: &EventLoop<()>, startup_data: Option<StartupData>) -> Self {
        let (log_message_sender, log_message_receiver) = channel();

        Log::add_listener(log_message_sender);

        let mut settings = Settings::default();

        match Settings::load() {
            Ok(s) => {
                settings = s;

                Log::info("Editor settings were loaded successfully!");
            }
            Err(e) => Log::warn(format!(
                "Failed to load settings, fallback to default. Reason: {:?}",
                e
            )),
        }

        let inner_size = PhysicalSize::new(
            settings.windows.window_size.x,
            settings.windows.window_size.y,
        );

        let mut window_attributes = WindowAttributes::default();
        window_attributes.inner_size = Some(inner_size.into());
        window_attributes.position = Some(
            PhysicalPosition::new(
                settings.windows.window_position.x,
                settings.windows.window_position.y,
            )
            .into(),
        );
        window_attributes.resizable = true;
        window_attributes.title = "FyroxEd".to_string();
        let graphics_context_params = GraphicsContextParams {
            window_attributes,
            vsync: true,
        };

        let serialization_context = Arc::new(SerializationContext::new());
        let mut engine = Engine::new(EngineInitParams {
            graphics_context_params,
            resource_manager: ResourceManager::new(),
            serialization_context,
        })
        .unwrap();

        // Editor cannot run on Android so we can safely initialize graphics context here.
        engine.initialize_graphics_context(event_loop).unwrap();

        let graphics_context = engine.graphics_context.as_initialized_mut();

        if let Ok(icon_img) = TextureResource::load_from_memory(
            include_bytes!("../resources/embed/icon.png"),
            TextureImportOptions::default()
                .with_compression(CompressionOptions::NoCompression)
                .with_minification_filter(TextureMinificationFilter::Linear),
        ) {
            let data = icon_img.data_ref();
            if let TextureKind::Rectangle { width, height } = data.kind() {
                if let Ok(img) = Icon::from_rgba(data.data().to_vec(), width, height) {
                    graphics_context.window.set_window_icon(Some(img));
                }
            }
        }

        // High-DPI screen support
        Log::info(format!(
            "UI scaling of your OS is: {}",
            graphics_context.window.scale_factor()
        ));

        let logical_size = graphics_context
            .window
            .inner_size()
            .to_logical(graphics_context.window.scale_factor());
        set_ui_scaling(
            &engine.user_interface,
            graphics_context.window.scale_factor() as f32,
        );

        let overlay_pass = OverlayRenderPass::new(graphics_context.renderer.pipeline_state());
        graphics_context
            .renderer
            .add_render_pass(overlay_pass.clone());

        let (message_sender, message_receiver) = mpsc::channel();
        let message_sender = MessageSender(message_sender);

        engine.user_interface.default_font.set(
            Font::from_memory(
                include_bytes!("../resources/embed/arial.ttf").as_slice(),
                14.0,
                Font::default_char_set(),
            )
            .unwrap(),
        );

        let configurator = Configurator::new(
            message_sender.clone(),
            &mut engine.user_interface.build_ctx(),
        );

        match graphics_context
            .renderer
            .set_quality_settings(&settings.graphics.quality)
        {
            Ok(_) => {
                Log::info("Graphics settings were applied successfully!");
            }
            Err(e) => Log::err(format!(
                "Failed to apply graphics settings! Reason: {:?}",
                e
            )),
        }

        let scene_viewer = SceneViewer::new(&mut engine, message_sender.clone());
        let asset_browser = AssetBrowser::new(&mut engine);
        let menu = Menu::new(&mut engine, message_sender.clone(), &settings);
        let light_panel = LightPanel::new(&mut engine, message_sender.clone());
        let audio_panel = AudioPanel::new(&mut engine, message_sender.clone());

        let ctx = &mut engine.user_interface.build_ctx();
        let navmesh_panel = NavmeshPanel::new(ctx, message_sender.clone());
        let world_outliner = WorldViewer::new(ctx, message_sender.clone(), &settings);
        let command_stack_viewer = CommandStackViewer::new(ctx, message_sender.clone());
        let log = LogPanel::new(ctx, log_message_receiver);
        let inspector = Inspector::new(ctx, message_sender.clone());
        let animation_editor = AnimationEditor::new(ctx);
        let absm_editor = AbsmEditor::new(ctx, message_sender.clone());
        let particle_system_control_panel = ParticleSystemPreviewControlPanel::new(ctx);
        let camera_control_panel = CameraPreviewControlPanel::new(ctx);
        let audio_preview_panel = AudioPreviewPanel::new(ctx);
        let doc_window = DocWindow::new(ctx);
        let node_removal_dialog = NodeRemovalDialog::new(ctx);
        let ragdoll_wizard = RagdollWizard::new(ctx, message_sender.clone());

        let docking_manager;
        let root_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(logical_size.width)
                .with_height(logical_size.height)
                .with_child(menu.menu)
                .with_child({
                    docking_manager =
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
                                                        .with_content(
                                                            TileContent::HorizontalTiles {
                                                                splitter: 0.66,
                                                                tiles: [
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            scene_viewer.window(),
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            inspector.window,
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                ],
                                                            },
                                                        )
                                                        .build(ctx),
                                                ],
                                            })
                                            .build(ctx),
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::HorizontalTiles {
                                                splitter: 0.66,
                                                tiles: [
                                                    TileBuilder::new(WidgetBuilder::new())
                                                        .with_content(
                                                            TileContent::HorizontalTiles {
                                                                splitter: 0.80,
                                                                tiles: [
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            asset_browser.window,
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            command_stack_viewer
                                                                                .window,
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                ],
                                                            },
                                                        )
                                                        .build(ctx),
                                                    TileBuilder::new(WidgetBuilder::new())
                                                        .with_content(
                                                            TileContent::HorizontalTiles {
                                                                splitter: 0.5,
                                                                tiles: [
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            log.window,
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                    TileBuilder::new(
                                                                        WidgetBuilder::new(),
                                                                    )
                                                                    .with_content(
                                                                        TileContent::Window(
                                                                            audio_panel.window,
                                                                        ),
                                                                    )
                                                                    .build(ctx),
                                                                ],
                                                            },
                                                        )
                                                        .build(ctx),
                                                ],
                                            })
                                            .build(ctx),
                                    ],
                                })
                                .build(ctx)
                        }))
                        .with_floating_windows(vec![
                            animation_editor.window,
                            absm_editor.window,
                            particle_system_control_panel.window,
                            camera_control_panel.window,
                            audio_preview_panel.window,
                            navmesh_panel.window,
                            doc_window.window,
                        ])
                        .build(ctx);
                    docking_manager
                }),
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

        let scene_settings = SceneSettingsWindow::new(ctx, message_sender.clone());

        let material_editor = MaterialEditor::new(&mut engine, message_sender.clone());

        if let Some(layout) = settings.windows.layout.as_ref() {
            engine
                .user_interface
                .send_message(DockingManagerMessage::layout(
                    docking_manager,
                    MessageDirection::ToWidget,
                    layout.clone(),
                ));
        }

        let editor = Self {
            docking_manager,
            animation_editor,
            engine,
            navmesh_panel,
            scene_viewer,
            scenes: SceneContainer::new(),
            message_sender,
            message_receiver,
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
                lag: 0.0,
            },
            absm_editor,
            build_window,
            build_profile: BuildProfile::Debug,
            scene_settings,
            particle_system_control_panel,
            camera_control_panel,
            overlay_pass,
            audio_preview_panel,
            node_removal_dialog,
            doc_window,
            plugins: Default::default(),
            // Apparently, some window managers (like Wayland), does not send `Focused` event after the window
            // was created. So we must assume that the editor is focused by default, otherwise editor's thread
            // will sleep forever and the window won't come up.
            focused: true,
            update_loop_state: UpdateLoopState::default(),
            is_suspended: false,
            ragdoll_wizard,
        };

        if let Some(data) = startup_data {
            editor.message_sender.send(Message::Configure {
                working_directory: if data.working_directory == PathBuf::default() {
                    std::env::current_dir().unwrap()
                } else {
                    data.working_directory
                },
            });

            if data.scene != PathBuf::default() {
                editor.message_sender.send(Message::LoadScene(data.scene));
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

    fn reload_settings(&mut self) {
        match Settings::load() {
            Ok(settings) => {
                self.settings = settings;

                Log::info("Editor settings were reloaded successfully!");
            }
            Err(e) => {
                self.settings = Default::default();

                Log::warn(format!(
                    "Failed to load settings, fallback to default. Reason: {:?}",
                    e
                ))
            }
        }

        self.menu
            .file_menu
            .update_recent_files_list(&mut self.engine.user_interface, &self.settings);

        match self
            .engine
            .graphics_context
            .as_initialized_mut()
            .renderer
            .set_quality_settings(&self.settings.graphics.quality)
        {
            Ok(_) => {
                Log::info("Graphics settings were applied successfully!");
            }
            Err(e) => Log::info(format!(
                "Failed to apply graphics settings! Reason: {:?}",
                e
            )),
        }
    }

    fn add_scene(&mut self, mut scene: Scene, path: Option<PathBuf>) {
        self.try_leave_preview_mode();

        self.sync_to_model();
        self.poll_ui_messages();

        // Setup new one.
        scene.rendering_options.render_target = Some(TextureResource::new_render_target(0, 0));

        self.scenes.add_scene_and_select(
            scene,
            path.clone(),
            &mut self.engine,
            &self.settings,
            self.message_sender.clone(),
            &self.scene_viewer,
        );

        if let Some(path) = path.as_ref() {
            if !self.settings.recent.scenes.contains(path) {
                self.settings.recent.scenes.push(path.clone());
                self.menu
                    .file_menu
                    .update_recent_files_list(&mut self.engine.user_interface, &self.settings);
            }
        }

        self.scene_viewer
            .reset_camera_projection(&self.engine.user_interface);
        self.engine
            .graphics_context
            .as_initialized_mut()
            .renderer
            .flush();

        self.on_scene_changed();
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
            let hot_key = HotKey::Some {
                code: *key,
                modifiers,
            };

            let mut processed = false;
            if let Some(scene) = self.scenes.current_scene_entry_mut() {
                if let Some(current_interaction_mode) = scene.current_interaction_mode {
                    processed |= scene.interaction_modes[current_interaction_mode as usize]
                        .on_hot_key(&hot_key, &mut scene.editor_scene, engine, &self.settings);
                }
            }

            if !processed {
                let key_bindings = &self.settings.key_bindings;

                if hot_key == key_bindings.redo {
                    sender.send(Message::RedoSceneCommand);
                } else if hot_key == key_bindings.undo {
                    sender.send(Message::UndoSceneCommand);
                } else if hot_key == key_bindings.enable_select_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Select));
                } else if hot_key == key_bindings.enable_move_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Move));
                } else if hot_key == key_bindings.enable_rotate_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Rotate));
                } else if hot_key == key_bindings.enable_scale_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Scale));
                } else if hot_key == key_bindings.enable_navmesh_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Navmesh));
                } else if hot_key == key_bindings.enable_terrain_mode {
                    sender.send(Message::SetInteractionMode(InteractionModeKind::Terrain));
                } else if hot_key == key_bindings.load_scene {
                    sender.send(Message::OpenLoadSceneDialog);
                } else if hot_key == key_bindings.save_scene {
                    if let Some(entry) = self.scenes.current_scene_entry_ref() {
                        if let Some(path) = entry.editor_scene.path.as_ref() {
                            self.message_sender.send(Message::SaveScene {
                                scene: entry.editor_scene.scene,
                                path: path.clone(),
                            });
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
                } else if hot_key == key_bindings.copy_selection {
                    if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                        if let Selection::Graph(graph_selection) = &editor_scene.selection {
                            editor_scene.clipboard.fill_from_selection(
                                graph_selection,
                                editor_scene.scene,
                                engine,
                            );
                        }
                    }
                } else if hot_key == key_bindings.paste {
                    if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                        if !editor_scene.clipboard.is_empty() {
                            sender.do_scene_command(PasteCommand::new(
                                editor_scene.scene_content_root,
                            ));
                        }
                    }
                } else if hot_key == key_bindings.new_scene {
                    sender.send(Message::NewScene);
                } else if hot_key == key_bindings.close_scene {
                    if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                        sender.send(Message::CloseScene(editor_scene.scene));
                    }
                } else if hot_key == key_bindings.remove_selection {
                    if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                        if !editor_scene.selection.is_empty() {
                            if let Selection::Graph(_) = editor_scene.selection {
                                if self.settings.general.show_node_removal_dialog
                                    && editor_scene.is_current_selection_has_external_refs(
                                        &engine.scenes[editor_scene.scene].graph,
                                    )
                                {
                                    sender.send(Message::OpenNodeRemovalDialog);
                                } else {
                                    sender.send(Message::DoSceneCommand(
                                        make_delete_selection_command(editor_scene, engine),
                                    ));
                                }
                            }
                        }
                    }
                } else if hot_key == key_bindings.focus {
                    if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                        if let Selection::Graph(selection) = &editor_scene.selection {
                            if let Some(first) = selection.nodes.first() {
                                sender.send(Message::FocusObject(*first));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &mut UiMessage) {
        scope_profile!();

        // Prevent infinite message loops.
        if message.has_flags(MSG_SYNC_FLAG) {
            return;
        }

        for_each_plugin!(self.plugins => on_ui_message(message, self));

        let engine = &mut self.engine;

        self.save_scene_dialog
            .handle_ui_message(message, &self.message_sender, &self.scenes);

        let mut current_scene_entry = self.scenes.current_scene_entry_mut();

        self.configurator.handle_ui_message(message, engine);
        self.menu.handle_ui_message(
            message,
            MenuContext {
                engine,
                editor_scene: current_scene_entry.as_mut().map(|e| &mut e.editor_scene),
                panels: Panels {
                    inspector_window: self.inspector.window,
                    world_outliner_window: self.world_viewer.window,
                    asset_window: self.asset_browser.window,
                    light_panel: self.light_panel.window,
                    log_panel: self.log.window,
                    navmesh_panel: self.navmesh_panel.window,
                    audio_panel: self.audio_panel.window,
                    configurator_window: self.configurator.window,
                    path_fixer: self.path_fixer.window,
                    curve_editor: &self.curve_editor,
                    absm_editor: &self.absm_editor,
                    command_stack_panel: self.command_stack_viewer.window,
                    scene_settings: &self.scene_settings,
                    animation_editor: &self.animation_editor,
                    ragdoll_wizard: &self.ragdoll_wizard,
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
            &mut self.scenes,
            &self.settings,
            &self.mode,
        );

        let mut current_scene_entry = self.scenes.current_scene_entry_mut();
        self.animation_editor.handle_ui_message(
            message,
            current_scene_entry.as_mut().map(|e| &mut e.editor_scene),
            engine,
            &self.message_sender,
        );

        if let Some(current_scene_entry) = current_scene_entry {
            let editor_scene = &mut current_scene_entry.editor_scene;

            self.ragdoll_wizard.handle_ui_message(
                message,
                &mut engine.user_interface,
                &mut engine.scenes[editor_scene.scene].graph,
                editor_scene,
                &self.message_sender,
            );
            self.particle_system_control_panel
                .handle_ui_message(message, editor_scene, engine);
            self.camera_control_panel
                .handle_ui_message(message, editor_scene, engine);
            self.audio_preview_panel
                .handle_ui_message(message, editor_scene, engine);
            self.absm_editor
                .handle_ui_message(message, engine, &self.message_sender, editor_scene);
            self.audio_panel
                .handle_ui_message(message, editor_scene, &self.message_sender, engine);
            self.node_removal_dialog.handle_ui_message(
                editor_scene,
                message,
                engine,
                &self.message_sender,
            );
            self.scene_settings
                .handle_ui_message(message, &self.message_sender);

            self.navmesh_panel.handle_message(message, editor_scene);

            self.inspector
                .handle_ui_message(message, editor_scene, engine, &self.message_sender);

            if let Some(current_im) = current_scene_entry.current_interaction_mode {
                current_scene_entry.interaction_modes[current_im as usize].handle_ui_message(
                    message,
                    editor_scene,
                    engine,
                );
            }

            self.world_viewer
                .handle_ui_message(message, editor_scene, engine, &mut self.settings);

            self.light_panel
                .handle_ui_message(message, editor_scene, engine);

            self.material_editor
                .handle_ui_message(message, engine, &self.message_sender);

            if let Some(FileSelectorMessage::Commit(path)) = message.data::<FileSelectorMessage>() {
                if message.destination() == self.save_file_selector {
                    self.message_sender.send(Message::SaveScene {
                        scene: editor_scene.scene,
                        path: path.clone(),
                    });
                    self.message_sender.send(Message::Exit { force: true });
                }
            }
        }

        if let Some(MessageBoxMessage::Close(result)) = message.data() {
            if message.destination() == self.exit_message_box {
                match result {
                    MessageBoxResult::No => {
                        self.message_sender.send(Message::Exit { force: true });
                    }
                    MessageBoxResult::Yes => {
                        if let Some(first_unsaved) = self.scenes.first_unsaved_scene() {
                            let editor_scene = &first_unsaved.editor_scene;
                            if editor_scene.need_save() {
                                if let Some(path) = editor_scene.path.as_ref() {
                                    self.message_sender.send(Message::SaveScene {
                                        scene: editor_scene.scene,
                                        path: path.clone(),
                                    });

                                    self.message_sender
                                        .send(Message::CloseScene(editor_scene.scene));

                                    self.message_sender.send(Message::Exit {
                                        force: self.scenes.unsaved_scene_count() == 1,
                                    });
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
                    }
                    _ => {}
                }
            }
        }

        self.handle_hotkeys(message);
    }

    fn set_play_mode(&mut self) {
        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
            if let Some(path) = editor_scene.path.as_ref().cloned() {
                self.save_scene(editor_scene.scene, path.clone());

                let mut process = std::process::Command::new("cargo");

                process
                    .stdout(Stdio::piped())
                    .arg("run")
                    .arg("--package")
                    .arg("executor");

                if let BuildProfile::Release = self.build_profile {
                    process.arg("--release");
                };

                process.arg("--").arg("--override-scene").arg(path);

                match process.spawn() {
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
            if let Some(scene) = self.scenes.current_editor_scene_ref() {
                if scene.path.is_some() {
                    let mut process = std::process::Command::new("cargo");
                    process
                        .stderr(Stdio::piped())
                        .arg("build")
                        .arg("--package")
                        .arg("executor");

                    if let BuildProfile::Release = self.build_profile {
                        process.arg("--release");
                    }

                    match process.spawn() {
                        Ok(mut process) => {
                            self.build_window.listen(
                                process.stderr.take().unwrap(),
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
        for_each_plugin!(self.plugins => on_mode_changed(self));

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

        for_each_plugin!(self.plugins => on_sync_to_model(self));

        let engine = &mut self.engine;

        self.menu.sync_to_model(
            self.scenes.current_editor_scene_ref(),
            &mut engine.user_interface,
        );

        self.scene_viewer.sync_to_model(&self.scenes, engine);

        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut current_scene_entry.editor_scene;
            self.animation_editor.sync_to_model(editor_scene, engine);
            self.absm_editor.sync_to_model(editor_scene, engine);
            self.scene_settings.sync_to_model(editor_scene, engine);
            self.inspector.sync_to_model(editor_scene, engine);
            self.world_viewer
                .sync_to_model(editor_scene, engine, &self.settings);
            self.material_editor
                .sync_to_model(&mut engine.user_interface);
            self.audio_panel.sync_to_model(editor_scene, engine);
            self.navmesh_panel.sync_to_model(engine, editor_scene);
            self.command_stack_viewer.sync_to_model(
                &mut current_scene_entry.command_stack,
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
        if let Some(scene) = self.scenes.current_editor_scene_mut() {
            self.world_viewer
                .post_update(scene, &mut self.engine, &self.settings);
        }

        for_each_plugin!(self.plugins => on_post_update(self));
    }

    fn handle_resize(&mut self) {
        let engine = &mut self.engine;
        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
            let scene = &mut engine.scenes[editor_scene.scene];

            // Create new render target if preview frame has changed its size.
            if let TextureKind::Rectangle { width, height } = scene
                .rendering_options
                .render_target
                .clone()
                .unwrap()
                .data_ref()
                .kind()
            {
                let frame_size = self.scene_viewer.frame_bounds(&engine.user_interface).size;
                if width != frame_size.x as u32 || height != frame_size.y as u32 {
                    scene.rendering_options.render_target =
                        Some(TextureResource::new_render_target(
                            frame_size.x as u32,
                            frame_size.y as u32,
                        ));
                    self.scene_viewer.set_render_target(
                        &engine.user_interface,
                        scene.rendering_options.render_target.clone(),
                    );
                }
            }
        }
    }

    fn do_scene_command(&mut self, command: SceneCommand) -> bool {
        let engine = &mut self.engine;
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut current_scene_entry.editor_scene;

            current_scene_entry.command_stack.do_command(
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
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut current_scene_entry.editor_scene;

            current_scene_entry.command_stack.undo(SceneContext {
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
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut current_scene_entry.editor_scene;

            current_scene_entry.command_stack.redo(SceneContext {
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
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut current_scene_entry.editor_scene;

            current_scene_entry.command_stack.clear(SceneContext {
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

    fn try_leave_preview_mode(&mut self) {
        if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
            let engine = &mut self.engine;
            self.particle_system_control_panel
                .leave_preview_mode(editor_scene, engine);
            self.camera_control_panel
                .leave_preview_mode(editor_scene, engine);
            self.audio_preview_panel
                .leave_preview_mode(editor_scene, engine);
            self.animation_editor
                .try_leave_preview_mode(editor_scene, engine);
            self.absm_editor
                .try_leave_preview_mode(editor_scene, engine);
        }
    }

    pub fn is_in_preview_mode(&mut self) -> bool {
        let mut is_any_plugin_in_preview_mode = false;
        let mut i = 0;
        while i < self.plugins.len() {
            if let Some(plugin) = self.plugins.get_mut(i).and_then(|p| p.take()) {
                is_any_plugin_in_preview_mode |= plugin.is_in_preview_mode(self);

                if let Some(entry) = self.plugins.get_mut(i) {
                    *entry = Some(plugin);
                }
            }

            i += 1;
        }

        let stays_active = match self.mode {
            Mode::Edit => false,
            Mode::Build { .. } => true,
            Mode::Play { .. } => false,
        };

        self.particle_system_control_panel.is_in_preview_mode()
            || self.camera_control_panel.is_in_preview_mode()
            || self.audio_preview_panel.is_in_preview_mode()
            || self.animation_editor.is_in_preview_mode()
            || self.absm_editor.is_in_preview_mode()
            || self.light_panel.is_in_preview_mode()
            || is_any_plugin_in_preview_mode
            || self
                .scenes
                .current_editor_scene_ref()
                .map_or(false, |s| s.camera_controller.is_interacting())
            || stays_active
    }

    fn save_scene(&mut self, scene: Handle<Scene>, path: PathBuf) {
        self.try_leave_preview_mode();

        let engine = &mut self.engine;
        if let Some(entry) = self.scenes.entry_by_scene_handle_mut(scene) {
            let editor_scene = &mut entry.editor_scene;

            if !self.settings.recent.scenes.contains(&path) {
                self.settings.recent.scenes.push(path.clone());
                self.menu
                    .file_menu
                    .update_recent_files_list(&mut engine.user_interface, &self.settings);
            }

            match editor_scene.save(path.clone(), &self.settings, engine) {
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

        self.sync_to_model();
    }

    fn load_scene(&mut self, scene_path: PathBuf) {
        for scene in self.scenes.scenes.iter() {
            if scene
                .editor_scene
                .path
                .as_ref()
                .map_or(false, |p| p == &scene_path)
            {
                self.set_current_scene(scene.editor_scene.scene);
                return;
            }
        }

        let engine = &mut self.engine;
        let result = {
            block_on(SceneLoader::from_file(
                &scene_path,
                &FsResourceIo,
                engine.serialization_context.clone(),
                engine.resource_manager.clone(),
            ))
        };
        match result {
            Ok(loader) => {
                let scene = block_on(loader.0.finish(&engine.resource_manager));

                self.add_scene(scene, Some(scene_path));
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
        } else if let Some(first_unsaved) = self.scenes.first_unsaved_scene() {
            engine.user_interface.send_message(MessageBoxMessage::open(
                self.exit_message_box,
                MessageDirection::ToWidget,
                None,
                Some(format!(
                    "There are unsaved changes in the {} scene. \
                    Do you wish to save them before exit?",
                    first_unsaved.editor_scene.name()
                )),
            ));
        } else {
            self.exit = true;
        }
    }

    fn close_scene(&mut self, scene: Handle<Scene>) -> bool {
        self.try_leave_preview_mode();

        let engine = &mut self.engine;
        if let Some(mut editor_scene_entry) = self.scenes.take_scene(scene) {
            engine.scenes.remove(editor_scene_entry.editor_scene.scene);

            // Preview frame has scene frame texture assigned, it must be cleared explicitly,
            // otherwise it will show last rendered frame in preview which is not what we want.
            self.scene_viewer
                .set_render_target(&engine.user_interface, None);
            // Set default title scene
            self.scene_viewer
                .set_title(&engine.user_interface, "Scene Preview".to_string());

            editor_scene_entry.on_drop(engine);

            self.on_scene_changed();

            true
        } else {
            false
        }
    }

    fn set_current_scene(&mut self, scene: Handle<Scene>) {
        assert!(self.scenes.set_current_scene(scene));

        self.on_scene_changed();
    }

    fn on_scene_changed(&mut self) {
        let ui = &self.engine.user_interface;
        self.world_viewer.clear(ui);
        self.animation_editor.clear(ui);
        self.absm_editor.clear(ui);
        self.poll_ui_messages();

        self.world_viewer.sync_selection = true;

        self.sync_to_model();
        self.poll_ui_messages();
    }

    fn create_new_scene(&mut self) {
        let mut scene = Scene::new();

        scene.rendering_options.ambient_lighting_color = Color::opaque(200, 200, 200);

        self.add_scene(scene, None);
    }

    fn configure(&mut self, working_directory: PathBuf) {
        assert!(self.scenes.is_empty());

        self.asset_browser.clear_preview(&mut self.engine);

        std::env::set_current_dir(working_directory.clone()).unwrap();

        // We must re-read settings, because each project have its own unique settings.
        self.reload_settings();

        self.load_layout();

        let engine = &mut self.engine;

        let graphics_context = engine.graphics_context.as_initialized_mut();

        graphics_context
            .window
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

        graphics_context.renderer.flush();

        self.asset_browser
            .set_working_directory(engine, &working_directory);

        self.world_viewer
            .on_configure(&engine.user_interface, &self.settings);

        Log::info(format!(
            "New working directory was successfully set: {:?}",
            working_directory
        ));
    }

    fn select_object(&mut self, type_id: TypeId, handle: ErasedHandle) {
        if let Some(scene) = self.scenes.current_editor_scene_ref() {
            let new_selection = if type_id == TypeId::of::<Node>() {
                if self.engine.scenes[scene.scene]
                    .graph
                    .is_valid_handle(handle.into())
                {
                    Some(Selection::Graph(GraphSelection::single_or_empty(
                        handle.into(),
                    )))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(new_selection) = new_selection {
                self.message_sender
                    .do_scene_command(ChangeSelectionCommand::new(
                        new_selection,
                        scene.selection.clone(),
                    ))
            }
        }
    }

    fn open_material_editor(&mut self, material: MaterialResource) {
        let engine = &mut self.engine;

        self.material_editor.set_material(Some(material), engine);

        engine.user_interface.send_message(WindowMessage::open(
            self.material_editor.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    fn poll_ui_messages(&mut self) -> usize {
        scope_profile!();

        let mut processed = 0;

        while let Some(mut ui_message) = self.engine.user_interface.poll_message() {
            self.handle_ui_message(&mut ui_message);
            processed += 1;
        }

        if processed > 0 {
            // We need to ensure, that all the changes will be correctly rendered on screen. So
            // request update and render on next frame.
            self.update_loop_state.request_update_in_next_frame();
        }

        processed
    }

    fn update(&mut self, dt: f32) {
        scope_profile!();

        for_each_plugin!(self.plugins => on_update(self));

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

        self.log.update(&mut self.engine);
        self.material_editor.update(&mut self.engine);
        self.asset_browser.update(&mut self.engine);

        if let Some(scene) = self.scenes.current_editor_scene_ref() {
            self.light_panel.update(scene, &mut self.engine);
            self.animation_editor.update(scene, &self.engine);
            self.audio_preview_panel.update(scene, &self.engine);
            self.scene_viewer.update(scene, &mut self.engine);
        }

        self.overlay_pass.borrow_mut().pictogram_size = self.settings.debugging.pictogram_size;

        let mut iterations = 1;
        while iterations > 0 {
            iterations -= 1;

            let ui_messages_processed_count = self.poll_ui_messages();

            let mut needs_sync = false;

            let mut editor_messages_processed_count = 0;
            while let Ok(message) = self.message_receiver.try_recv() {
                for_each_plugin!(self.plugins => on_message(&message, self));

                editor_messages_processed_count += 1;
                self.path_fixer
                    .handle_message(&message, &self.engine.user_interface);

                self.save_scene_dialog
                    .handle_message(&message, &self.message_sender);

                if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                    self.inspector.handle_message(
                        &message,
                        editor_scene,
                        &mut self.engine,
                        &self.message_sender,
                    );
                }

                if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                    self.particle_system_control_panel.handle_message(
                        &message,
                        editor_scene,
                        &mut self.engine,
                    );
                    self.camera_control_panel.handle_message(
                        &message,
                        editor_scene,
                        &mut self.engine,
                    );
                    self.audio_preview_panel.handle_message(
                        &message,
                        editor_scene,
                        &mut self.engine,
                    );
                    self.animation_editor
                        .handle_message(&message, editor_scene, &mut self.engine);
                    self.absm_editor
                        .handle_message(&message, editor_scene, &mut self.engine);
                }

                self.scene_viewer.handle_message(&message, &mut self.engine);

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
                    Message::SelectionChanged { .. } => {
                        self.world_viewer.sync_selection = true;
                    }
                    Message::SaveScene { scene, path } => self.save_scene(scene, path),
                    Message::LoadScene(scene_path) => {
                        self.load_scene(scene_path);
                        needs_sync = true;
                    }
                    Message::SetInteractionMode(mode_kind) => {
                        if let Some(editor_scene_entry) = self.scenes.current_scene_entry_mut() {
                            editor_scene_entry
                                .set_interaction_mode(&mut self.engine, Some(mode_kind));
                        }
                    }
                    Message::Exit { force } => self.exit(force),
                    Message::CloseScene(scene) => {
                        needs_sync |= self.close_scene(scene);
                    }
                    Message::NewScene => {
                        self.create_new_scene();
                        needs_sync = true;
                    }
                    Message::SetCurrentScene(scene) => {
                        self.set_current_scene(scene);
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
                    Message::OpenNodeRemovalDialog => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                            self.node_removal_dialog.open(editor_scene, &self.engine)
                        }
                    }
                    Message::ShowInAssetBrowser(path) => {
                        self.asset_browser
                            .locate_path(&self.engine.user_interface, path);
                    }
                    Message::SetWorldViewerFilter(filter) => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                            self.world_viewer.set_filter(
                                filter,
                                editor_scene,
                                &self.engine.user_interface,
                            );
                        }
                    }
                    Message::LocateObject { type_id, handle } => self
                        .world_viewer
                        .try_locate_object(type_id, handle, &self.engine),
                    Message::SelectObject { type_id, handle } => {
                        self.select_object(type_id, handle);
                    }
                    Message::FocusObject(handle) => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_mut() {
                            let scene = &mut self.engine.scenes[editor_scene.scene];
                            editor_scene.camera_controller.fit_object(scene, handle);
                        }
                    }
                    Message::SetEditorCameraProjection(projection) => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
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
                    Message::SwitchToBuildMode => self.set_build_mode(),
                    Message::SwitchToEditMode => self.set_editor_mode(),
                    Message::OpenLoadSceneDialog => {
                        self.menu
                            .open_load_file_selector(&mut self.engine.user_interface);
                    }
                    Message::OpenSaveSceneDialog => {
                        self.menu
                            .open_save_file_selector(&mut self.engine.user_interface);
                    }
                    Message::OpenSaveSceneConfirmationDialog { scene, action } => {
                        self.save_scene_dialog.open(
                            &self.engine.user_interface,
                            scene,
                            &self.scenes,
                            action,
                        );
                    }
                    Message::SetBuildProfile(profile) => {
                        self.build_profile = profile;
                    }
                    Message::SaveSelectionAsPrefab(path) => {
                        self.try_save_selection_as_prefab(path);
                    }
                    Message::SyncNodeHandleName { view, handle } => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                            let scene = &self.engine.scenes[editor_scene.scene];
                            self.engine.user_interface.send_message(
                                HandlePropertyEditorMessage::name(
                                    view,
                                    MessageDirection::ToWidget,
                                    scene.graph.try_get(handle).map(|n| n.name_owned()),
                                ),
                            );
                        }
                    }
                    Message::ForceSync => {
                        needs_sync = true;
                    }
                    Message::OpenAnimationEditor => {
                        self.animation_editor.open(&self.engine.user_interface);
                    }
                    Message::OpenAbsmEditor => self.absm_editor.open(&self.engine.user_interface),
                    Message::ShowDocumentation(doc) => {
                        self.doc_window.open(doc, &self.engine.user_interface);
                    }
                    Message::SaveLayout => {
                        self.save_layout();
                    }
                    Message::LoadLayout => {
                        self.load_layout();
                    }
                    Message::ProvideSceneHierarchy { view } => {
                        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                            let scene = &self.engine.scenes[editor_scene.scene];
                            self.engine.user_interface.send_message(
                                HandlePropertyEditorMessage::hierarchy(
                                    view,
                                    MessageDirection::ToWidget,
                                    HierarchyNode::from_scene_node(
                                        editor_scene.scene_content_root,
                                        Handle::NONE,
                                        &scene.graph,
                                    ),
                                ),
                            );
                        }
                    }
                }
            }

            if needs_sync {
                self.sync_to_model();
            }

            if editor_messages_processed_count > 0 {
                self.update_loop_state.request_update_in_next_frame();
            }

            // Any processed UI message can produce editor messages and vice versa, in this case we
            // must do another pass.
            if ui_messages_processed_count > 0 || editor_messages_processed_count > 0 {
                iterations += 1;
            }
        }

        self.handle_resize();

        if let Some(editor_scene_entry) = self.scenes.current_scene_entry_mut() {
            let editor_scene = &mut editor_scene_entry.editor_scene;

            editor_scene.update(&mut self.engine, dt, &mut self.settings);

            self.absm_editor.update(editor_scene, &mut self.engine);

            if let Some(mode) = editor_scene_entry.current_interaction_mode {
                editor_scene_entry.interaction_modes[mode as usize].update(
                    editor_scene,
                    editor_scene.camera_controller.camera,
                    &mut self.engine,
                    &self.settings,
                );
            }
        }

        self.settings.update();
    }

    fn save_layout(&mut self) {
        let layout = self
            .engine
            .user_interface
            .node(self.docking_manager)
            .query_component::<DockingManager>()
            .unwrap()
            .layout(&self.engine.user_interface);
        self.settings.windows.layout = Some(layout);
    }

    fn load_layout(&mut self) {
        if let Some(layout) = self.settings.windows.layout.as_ref() {
            self.engine
                .user_interface
                .send_message(DockingManagerMessage::layout(
                    self.docking_manager,
                    MessageDirection::ToWidget,
                    layout.clone(),
                ));
        }
    }

    fn try_save_selection_as_prefab(&self, path: PathBuf) {
        if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
            let source_scene = &self.engine.scenes[editor_scene.scene];
            let mut dest_scene = Scene::new();
            if let Selection::Graph(ref graph_selection) = editor_scene.selection {
                for root_node in graph_selection.root_nodes(&source_scene.graph) {
                    source_scene.graph.copy_node(
                        root_node,
                        &mut dest_scene.graph,
                        &mut |_, _| true,
                        &mut |_, _, _| {},
                    );
                }

                let mut visitor = Visitor::new();
                match dest_scene.save("Scene", &mut visitor) {
                    Err(e) => Log::err(format!(
                        "Failed to save selection as prefab! Reason: {:?}",
                        e
                    )),
                    Ok(_) => {
                        if let Err(e) = visitor.save_binary(&path) {
                            Log::err(format!(
                                "Failed to save selection as prefab! Reason: {:?}",
                                e
                            ));
                        } else {
                            Log::info(format!(
                                "Selection was successfully saved as prefab to {:?}!",
                                path
                            ))
                        }
                    }
                }
            } else {
                Log::warn(
                    "Unable to selection to prefab, because selection is not scene selection!",
                );
            }
        } else {
            Log::warn("Unable to save selection to prefab, because there is no scene loaded!");
        }
    }

    pub fn add_game_plugin<P>(&mut self, plugin: P)
    where
        P: PluginConstructor + 'static,
    {
        self.engine.add_plugin_constructor(plugin)
    }

    pub fn add_editor_plugin<P>(&mut self, plugin: P)
    where
        P: EditorPlugin + 'static,
    {
        self.plugins.push(Some(Box::new(plugin)));
    }

    pub fn is_active(&self) -> bool {
        !self.update_loop_state.is_suspended()
            && (self.focused || !self.settings.general.suspend_unfocused_editor)
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        for_each_plugin!(self.plugins => on_start(&mut self));

        event_loop
            .run(move |event, window_target| match event {
                Event::AboutToWait => {
                    if self.is_active() {
                        update(&mut self, window_target);
                    }

                    if self.exit {
                        window_target.exit();

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
                Event::WindowEvent { ref event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            self.message_sender.send(Message::Exit { force: false });
                        }
                        WindowEvent::Resized(size) => {
                            if let Err(e) = self.engine.set_frame_size((*size).into()) {
                                fyrox::core::log::Log::writeln(
                                    MessageKind::Error,
                                    format!("Failed to set renderer size! Reason: {:?}", e),
                                );
                            }

                            let logical_size = size.to_logical(
                                self.engine
                                    .graphics_context
                                    .as_initialized_ref()
                                    .window
                                    .scale_factor(),
                            );
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

                            if size.width > 0 && size.height > 0 {
                                self.settings.windows.window_size.x = size.width as f32;
                                self.settings.windows.window_size.y = size.height as f32;
                            }
                        }
                        WindowEvent::Focused(focused) => {
                            self.focused = *focused;
                        }
                        WindowEvent::Moved(new_position) => {
                            if new_position.x > 0 && new_position.y > 0 {
                                self.settings.windows.window_position.x = new_position.x as f32;
                                self.settings.windows.window_position.y = new_position.y as f32;
                            }
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            set_ui_scaling(&self.engine.user_interface, *scale_factor as f32);
                        }
                        WindowEvent::RedrawRequested => {
                            if self.is_active() {
                                // Temporarily disable cameras in currently edited scene. This is needed to prevent any
                                // scene camera to interfere with the editor camera.
                                let mut camera_state = Vec::new();
                                if let Some(editor_scene) = self.scenes.current_editor_scene_ref() {
                                    let scene = &mut self.engine.scenes[editor_scene.scene];
                                    let has_preview_camera =
                                        scene.graph.is_valid_handle(editor_scene.preview_camera);
                                    for (handle, camera) in
                                        scene.graph.pair_iter_mut().filter_map(|(h, n)| {
                                            if has_preview_camera
                                                && h != editor_scene.preview_camera
                                                || !has_preview_camera
                                                    && h != editor_scene.camera_controller.camera
                                            {
                                                n.cast_mut::<Camera>().map(|c| (h, c))
                                            } else {
                                                None
                                            }
                                        })
                                    {
                                        camera_state.push((handle, camera.is_enabled()));
                                        camera.set_enabled(false);
                                    }
                                }

                                self.engine.render().unwrap();

                                // Revert state of the cameras.
                                if let Some(scene) = self.scenes.current_editor_scene_ref() {
                                    for (handle, enabled) in camera_state {
                                        self.engine.scenes[scene.scene].graph[handle]
                                            .as_camera_mut()
                                            .set_enabled(enabled);
                                    }
                                }
                            }
                        }
                        _ => (),
                    }

                    self.update_loop_state.request_update_in_current_frame();

                    if let Some(os_event) = translate_event(event) {
                        self.engine.user_interface.process_os_event(&os_event);
                    }
                }
                Event::LoopExiting => {
                    self.settings.force_save();

                    for_each_plugin!(self.plugins => on_exit(&mut self));
                }
                _ => {
                    if !self.update_loop_state.is_suspended() && self.is_active() {
                        if self.is_suspended {
                            for_each_plugin!(self.plugins => on_resumed(&mut self));
                            self.is_suspended = false;
                        }

                        window_target.set_control_flow(ControlFlow::Poll);
                    } else {
                        if !self.is_suspended {
                            for_each_plugin!(self.plugins => on_suspended(&mut self));
                            self.is_suspended = true;
                        }

                        window_target.set_control_flow(ControlFlow::Wait);
                    }
                }
            })
            .unwrap();
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

fn update(editor: &mut Editor, window_target: &EventLoopWindowTarget<()>) {
    scope_profile!();

    let elapsed = editor.game_loop_data.clock.elapsed().as_secs_f32();
    editor.game_loop_data.clock = Instant::now();
    editor.game_loop_data.lag += elapsed;

    while editor.game_loop_data.lag >= FIXED_TIMESTEP {
        editor.game_loop_data.lag -= FIXED_TIMESTEP;

        let mut switches = FxHashMap::default();

        for other_editor_scene_entry in editor.scenes.scenes.iter() {
            if let Some(current_editor_scene) = editor.scenes.current_editor_scene_ref() {
                switches.insert(
                    current_editor_scene.scene,
                    current_editor_scene.graph_switches.clone(),
                );

                if current_editor_scene.scene == other_editor_scene_entry.editor_scene.scene {
                    continue;
                }
            }

            // Other scenes will be paused.
            switches.insert(
                other_editor_scene_entry.editor_scene.scene,
                GraphUpdateSwitches {
                    paused: true,
                    ..Default::default()
                },
            );
        }

        editor.engine.pre_update(
            FIXED_TIMESTEP,
            window_target,
            &mut editor.game_loop_data.lag,
            switches,
        );

        editor.update(FIXED_TIMESTEP);

        editor.engine.post_update(FIXED_TIMESTEP);

        editor.post_update();

        if editor.game_loop_data.lag >= 1.5 * FIXED_TIMESTEP {
            break;
        }
    }

    let window = &editor.engine.graphics_context.as_initialized_ref().window;
    window.set_cursor_icon(translate_cursor_icon(editor.engine.user_interface.cursor()));
    window.request_redraw();

    if !editor.is_in_preview_mode() {
        editor.update_loop_state.decrease_counter();
    }
}
