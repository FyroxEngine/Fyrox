// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![allow(irrefutable_let_patterns)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::mixed_read_write_in_expression)]
// These are useless.
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::inconsistent_struct_constructor)]
#![allow(clippy::mutable_key_type)]

#[macro_use]
extern crate lazy_static;

pub mod asset;
pub mod audio;
pub mod camera;
pub mod command;
pub mod configurator;
pub mod export;
pub mod highlight;
pub mod interaction;
pub mod light;
pub mod menu;
pub mod mesh;
pub mod message;
pub mod overlay;
pub mod particle;
pub mod plugin;
pub mod plugins;
pub mod preview;
pub mod scene;
pub mod scene_viewer;
pub mod settings;
pub mod stats;
pub mod ui_scene;
pub mod utils;
pub mod world;

pub use fyrox;
use fyrox::core::make_relative_path;

use crate::plugins::probe::ReflectionProbePlugin;
use crate::{
    asset::{item::AssetItem, AssetBrowser},
    audio::{preview::AudioPreviewPanel, AudioPanel},
    camera::panel::CameraPreviewControlPanel,
    command::{panel::CommandStackViewer, Command, CommandTrait},
    configurator::Configurator,
    export::ExportWindow,
    fyrox::{
        asset::{io::FsResourceIo, manager::ResourceManager},
        core::{
            algebra::{Matrix3, Vector2},
            color::Color,
            futures::executor::block_on,
            log::{Log, MessageKind},
            parking_lot::Mutex,
            pool::Handle,
            task::TaskPool,
            uuid::Uuid,
            watcher::FileSystemWatcher,
            TypeUuidProvider,
        },
        dpi::{PhysicalPosition, PhysicalSize},
        engine::{Engine, EngineInitParams, GraphicsContextParams, SerializationContext},
        event::{Event, WindowEvent},
        event_loop::{EventLoop, EventLoopWindowTarget},
        fxhash::FxHashMap,
        graph::BaseSceneGraph,
        gui::{
            brush::Brush,
            button::ButtonBuilder,
            constructor::new_widget_constructor_container,
            dock::{
                DockingManager, DockingManagerBuilder, DockingManagerMessage, TileBuilder,
                TileContent,
            },
            dropdown_list::DropdownListBuilder,
            file_browser::{FileBrowserMode, FileSelectorBuilder, Filter},
            font::Font,
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            key::HotKey,
            log::LogPanel,
            message::{MessageDirection, UiMessage},
            messagebox::{
                MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult,
            },
            style::{resource::StyleResource, Style},
            text::TextBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, UiNode, UserInterface, VerticalAlignment,
        },
        material::{
            shader::{ShaderResource, ShaderResourceExtension},
            Material, MaterialResource,
        },
        plugin::{dylib::DyLibDynamicPlugin, DynamicPlugin, Plugin, PluginContainer},
        resource::texture::{
            CompressionOptions, TextureImportOptions, TextureMinificationFilter, TextureResource,
            TextureResourceExtension,
        },
        scene::{graph::GraphUpdateSwitches, mesh::Mesh, Scene, SceneLoader},
        utils::{translate_cursor_icon, translate_event},
        window::WindowAttributes,
    },
    highlight::HighlightRenderPass,
    interaction::{
        move_mode::MoveInteractionMode,
        navmesh::{EditNavmeshMode, NavmeshPanel},
        rotate_mode::RotateInteractionMode,
        scale_mode::ScaleInteractionMode,
        select_mode::SelectInteractionMode,
        terrain::TerrainInteractionMode,
    },
    light::LightPanel,
    menu::{Menu, MenuContext, Panels},
    mesh::{MeshControlPanel, SurfaceDataViewer},
    message::MessageSender,
    overlay::OverlayRenderPass,
    particle::ParticleSystemPreviewControlPanel,
    plugin::{EditorPlugin, EditorPluginsContainer},
    plugins::{
        absm::AbsmEditor, absm::AbsmEditorPlugin, animation::AnimationEditorPlugin,
        collider::ColliderPlugin, curve_editor::CurveEditorPlugin, material::MaterialPlugin,
        ragdoll::RagdollPlugin, settings::SettingsPlugin, stats::UiStatisticsPlugin,
        tilemap::TileMapEditorPlugin,
    },
    scene::{
        commands::{
            make_delete_selection_command, ChangeSelectionCommand, GameSceneContext, PasteCommand,
        },
        container::{EditorSceneEntry, SceneContainer},
        dialog::NodeRemovalDialog,
        settings::SceneSettingsWindow,
        GameScene, Selection,
    },
    scene_viewer::SceneViewer,
    settings::{general::EditorStyle, Settings},
    stats::{StatisticsWindow, StatisticsWindowAction},
    ui_scene::{
        commands::graph::PasteWidgetCommand, menu::WidgetContextMenu,
        utils::UiSceneWorldViewerDataProvider, UiScene,
    },
    utils::doc::DocWindow,
    world::{graph::EditorSceneWrapper, menu::SceneNodeContextMenu, WorldViewer},
};
use fyrox::asset::untyped::ResourceKind;
use fyrox::engine::ApplicationLoopController;
use fyrox_build_tools::{build::BuildWindow, CommandDescriptor};
pub use message::Message;
use plugins::inspector::InspectorPlugin;
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{BufRead, BufReader, Cursor, Read},
    path::{Path, PathBuf},
    process::Stdio,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver},
        Arc, LazyLock,
    },
    time::{Duration, Instant},
};
use toml_edit::DocumentMut;

pub const FIXED_TIMESTEP: f32 = 1.0 / 60.0;
pub const MSG_SYNC_FLAG: u64 = 1;

static EDITOR_VERSION: LazyLock<String> = LazyLock::new(|| {
    let manifest = include_bytes!("../Cargo.toml");
    let mut file = Cursor::new(&manifest);
    let mut toml = String::new();
    if file.read_to_string(&mut toml).is_ok() {
        if let Ok(document) = toml.parse::<DocumentMut>() {
            if let Some(package) = document.get("package").and_then(|i| i.as_table()) {
                if let Some(version) = package.get("version") {
                    return version.to_string().replace('\"', "");
                }
            }
        }
    }

    "<unknown>".to_string()
});

pub fn send_sync_message(ui: &UserInterface, mut msg: UiMessage) {
    msg.flags = MSG_SYNC_FLAG;
    ui.send_message(msg);
}

pub fn send_sync_messages<const N: usize>(ui: &UserInterface, mut messages: [UiMessage; N]) {
    for message in &mut messages {
        message.flags = MSG_SYNC_FLAG;
    }
    ui.send_messages(messages);
}

lazy_static! {
    static ref EDITOR_TEXTURE_CACHE: Mutex<FxHashMap<usize, TextureResource>> = Default::default();
}

pub fn load_texture_internal(data: &[u8]) -> Option<TextureResource> {
    let mut cache = EDITOR_TEXTURE_CACHE.lock();

    // Editor use data that is embedded in the binary, so each such piece of data will have fixed
    // location in memory. This fact allows us to cache the resources and skip redundant loading if
    // they're already loaded.
    let id = data.as_ptr() as usize;

    if let Some(existing) = cache.get(&id) {
        Some(existing.clone())
    } else {
        let texture = TextureResource::load_from_memory(
            Uuid::new_v4(),
            ResourceKind::Embedded,
            data,
            TextureImportOptions::default()
                .with_compression(CompressionOptions::NoCompression)
                .with_minification_filter(TextureMinificationFilter::LinearMipMapLinear)
                .with_lod_bias(-1.0),
        )
        .ok()?;

        cache.insert(id, texture.clone());

        Some(texture)
    }
}

pub fn load_image_internal(data: &[u8]) -> Option<TextureResource> {
    load_texture_internal(data)
}

#[macro_export]
macro_rules! load_texture {
    ($file:expr $(,)?) => {
        $crate::load_texture_internal(include_bytes!($file))
    };
}

#[macro_export]
macro_rules! load_image {
    ($file:expr $(,)?) => {
        $crate::load_image_internal(include_bytes!($file))
    };
}

lazy_static! {
    static ref GIZMO_SHADER: ShaderResource = {
        ShaderResource::from_str(
            Uuid::new_v4(),
            include_str!("../resources/shaders/gizmo.shader",),
            Default::default(),
        )
        .unwrap()
    };
}

pub fn make_color_material(color: Color) -> MaterialResource {
    let mut material = Material::from_shader(GIZMO_SHADER.clone());
    material.set_property("diffuseColor", color);
    MaterialResource::new_embedded(material)
}

pub fn set_mesh_diffuse_color(mesh: &mut Mesh, color: Color) {
    for surface in mesh.surfaces() {
        surface
            .material()
            .data_ref()
            .set_property("diffuseColor", color);
    }
}

pub fn create_terrain_layer_material() -> MaterialResource {
    let mut material = Material::standard_terrain();
    material.set_property("texCoordScale", Vector2::new(10.0, 10.0));
    MaterialResource::new_embedded(material)
}

pub fn make_scene_file_filter() -> Filter {
    Filter::new(|p: &Path| {
        p.is_dir()
            || p.extension()
                .is_some_and(|ext| matches!(ext.to_string_lossy().as_ref(), "rgs" | "ui"))
    })
}

pub fn make_save_file_selector(
    ctx: &mut BuildContext,
    default_file_name: PathBuf,
) -> Handle<UiNode> {
    FileSelectorBuilder::new(
        WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::text("Save Scene As"))
            .open(false)
            .with_remove_on_close(true),
    )
    .with_mode(FileBrowserMode::Save { default_file_name })
    .with_path("./")
    .with_filter(make_scene_file_filter())
    .build(ctx)
}

pub enum Mode {
    Edit,
    Build {
        queue: VecDeque<CommandDescriptor>,
        process: Option<std::process::Child>,
        play_after_build: bool,
    },
    Play {
        process: std::process::Child,
        active: Arc<AtomicBool>,
    },
}

impl Mode {
    pub fn is_edit(&self) -> bool {
        matches!(self, Mode::Edit)
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
    pub scenes: Vec<PathBuf>,
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
    CloseScene(Uuid),
}

pub struct SaveSceneConfirmationDialog {
    save_message_box: Handle<UiNode>,
    action: SaveSceneConfirmationDialogAction,
    id: Uuid,
}

impl SaveSceneConfirmationDialog {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let save_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(100.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::text("Unsaved changes")),
        )
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        Self {
            save_message_box,
            action: SaveSceneConfirmationDialogAction::None,
            id: Default::default(),
        }
    }

    pub fn open(
        &mut self,
        ui: &UserInterface,
        id: Uuid,
        scenes: &SceneContainer,
        action: SaveSceneConfirmationDialogAction,
    ) {
        self.id = id;
        self.action = action;

        if let Some(entry) = scenes.entry_by_scene_id(self.id) {
            ui.send_message(MessageBoxMessage::open(
                self.save_message_box,
                MessageDirection::ToWidget,
                None,
                Some(format!(
                    "There are unsaved changes in the {} scene. \
                Do you wish to save them before continue?",
                    entry.name(),
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
                        if let Some(entry) = scenes.entry_by_scene_id(self.id) {
                            if let Some(path) = entry.path.clone() {
                                // If the scene was already saved into some file - save it
                                // immediately and perform the requested action.
                                sender.send(Message::SaveScene { id: self.id, path });

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
                                sender.send(Message::OpenSaveSceneDialog {
                                    default_file_name: entry.default_file_name(),
                                })
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn handle_message(&mut self, message: &Message, sender: &MessageSender) {
        if let Message::SaveScene { id: scene, .. } = message {
            if *scene == self.id {
                self.id = Default::default();

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
    pub audio_panel: AudioPanel,
    pub mode: Mode,
    pub build_window: Option<BuildWindow>,
    pub scene_settings: SceneSettingsWindow,
    pub particle_system_control_panel: ParticleSystemPreviewControlPanel,
    pub camera_control_panel: CameraPreviewControlPanel,
    pub mesh_control_panel: MeshControlPanel,
    pub audio_preview_panel: AudioPreviewPanel,
    pub doc_window: DocWindow,
    pub docking_manager: Handle<UiNode>,
    pub node_removal_dialog: NodeRemovalDialog,
    pub engine: Engine,
    pub plugins: EditorPluginsContainer,
    pub focused: bool,
    pub update_loop_state: UpdateLoopState,
    pub is_suspended: bool,
    pub scene_node_context_menu: Rc<RefCell<SceneNodeContextMenu>>,
    pub widget_context_menu: Rc<RefCell<WidgetContextMenu>>,
    pub overlay_pass: Option<Rc<RefCell<OverlayRenderPass>>>,
    pub highlighter: Option<Rc<RefCell<HighlightRenderPass>>>,
    pub export_window: Option<ExportWindow>,
    pub statistics_window: Option<StatisticsWindow>,
    pub surface_data_viewer: Option<SurfaceDataViewer>,
    pub processed_ui_messages: usize,
    pub styles: FxHashMap<EditorStyle, StyleResource>,
    pub running_game_process: Option<(std::process::Child, Arc<AtomicBool>)>,
    pub user_project_icon: Option<Vec<u8>>,
    pub user_project_name: String,
    pub user_project_version: String,
}

impl Editor {
    pub fn new(startup_data: Option<StartupData>) -> Self {
        Self::new_with_settings(startup_data, Default::default())
    }

    pub fn new_with_settings(startup_data: Option<StartupData>, settings: Settings) -> Self {
        // Useful for debugging purposes when users don't bother to mention editor version
        // they're using.
        Log::info(format!("Editor version: {}", &*EDITOR_VERSION));

        let (log_message_sender, log_message_receiver) = channel();

        Log::add_listener(log_message_sender);

        let mut dark_style = Style::dark_style();
        dark_style
            .set(
                WorldViewer::INSTANCE_BRUSH,
                Brush::Solid(Color::opaque(160, 160, 200)),
            )
            .set(
                AssetItem::SELECTED_FOREGROUND,
                Brush::Solid(Color::opaque(200, 220, 240)),
            )
            .set(
                AssetItem::SELECTED_BACKGROUND,
                Brush::Solid(Color::opaque(100, 100, 100)),
            )
            .set(
                AssetItem::DESELECTED_BRUSH,
                Brush::Solid(Color::TRANSPARENT),
            )
            .set(ExportWindow::TITLE_BRUSH, Brush::Solid(Color::CORN_SILK))
            .set(
                AbsmEditor::NORMAL_ROOT_COLOR,
                Brush::Solid(Color::opaque(40, 80, 0)),
            )
            .set(
                AbsmEditor::SELECTED_ROOT_COLOR,
                Brush::Solid(Color::opaque(60, 100, 0)),
            );

        let dark_style = StyleResource::new_embedded(dark_style);
        let mut light_style = Style::light_style();
        light_style
            .set(
                WorldViewer::INSTANCE_BRUSH,
                Brush::Solid(Color::opaque(70, 70, 120)),
            )
            .set(
                AssetItem::SELECTED_FOREGROUND,
                Brush::Solid(Color::opaque(200, 220, 240)),
            )
            .set(
                AssetItem::SELECTED_BACKGROUND,
                Brush::Solid(Color::opaque(100, 100, 100)),
            )
            .set(
                AssetItem::DESELECTED_BRUSH,
                Brush::Solid(Color::TRANSPARENT),
            )
            .set(ExportWindow::TITLE_BRUSH, Brush::Solid(Color::CORN_SILK))
            .set(
                AbsmEditor::NORMAL_ROOT_COLOR,
                Brush::Solid(Color::opaque(40, 80, 0)),
            )
            .set(
                AbsmEditor::SELECTED_ROOT_COLOR,
                Brush::Solid(Color::opaque(60, 100, 0)),
            );

        let light_style = StyleResource::new_embedded(light_style);
        let styles = [
            (EditorStyle::Dark, dark_style),
            (EditorStyle::Light, light_style),
        ]
        .into_iter()
        .collect::<FxHashMap<_, _>>();

        let mut settings = settings;

        match Settings::load() {
            Ok(s) => {
                settings = s;

                Log::info("Editor settings were loaded successfully!");
            }
            Err(e) => Log::warn(format!(
                "Failed to load settings, fallback to default. Reason: {e:?}"
            )),
        }

        let inner_size = PhysicalSize::new(
            settings.windows.window_size.x,
            settings.windows.window_size.y,
        );

        let mut window_attributes = WindowAttributes::default();
        window_attributes.maximized = settings.windows.window_maximized;
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
            msaa_sample_count: Some(4),
            graphics_server_constructor: Default::default(),
        };

        let serialization_context = Arc::new(SerializationContext::new());
        let task_pool = Arc::new(TaskPool::new());
        let mut engine = Engine::new(EngineInitParams {
            graphics_context_params,
            resource_manager: ResourceManager::new(Arc::new(FsResourceIo), task_pool.clone()),
            serialization_context,
            task_pool,
            widget_constructors: Arc::new(new_widget_constructor_container()),
        })
        .unwrap();

        let (message_sender, message_receiver) = mpsc::channel();
        let message_sender = MessageSender(message_sender);

        {
            let mut font_state = engine.user_interfaces.first_mut().default_font.state();
            let font_state_data = font_state.data().unwrap();
            *font_state_data = Font::from_memory(
                include_bytes!("../resources/Roboto-Regular.ttf").as_slice(),
                1024,
            )
            .unwrap();
        }

        let ui = engine.user_interfaces.first_mut();
        if let Some(style) = styles.get(&settings.general.style) {
            ui.set_style(style.clone());
        }

        let configurator = Configurator::new(message_sender.clone(), &mut ui.build_ctx());

        let scene_viewer = SceneViewer::new(&mut engine, message_sender.clone(), &mut settings);
        let asset_browser = AssetBrowser::new(&mut engine);
        let menu = Menu::new(&mut engine, message_sender.clone(), &settings);
        let light_panel = LightPanel::new(&mut engine, message_sender.clone());
        let audio_panel = AudioPanel::new(&mut engine, message_sender.clone());
        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();
        let navmesh_panel = NavmeshPanel::new(scene_viewer.frame(), ctx, message_sender.clone());
        let scene_node_context_menu = Rc::new(RefCell::new(SceneNodeContextMenu::new(
            &engine.serialization_context,
            &engine.widget_constructors,
            ctx,
        )));
        let widget_context_menu = Rc::new(RefCell::new(WidgetContextMenu::new(
            &engine.widget_constructors,
            ctx,
        )));
        let world_outliner = WorldViewer::new(ctx, message_sender.clone(), &settings);
        let command_stack_viewer = CommandStackViewer::new(ctx, message_sender.clone());
        let log = LogPanel::new(
            ctx,
            log_message_receiver,
            load_image!("../resources/clear.png"),
            true,
        );
        let inspector_plugin =
            InspectorPlugin::new(ctx, message_sender.clone(), engine.resource_manager.clone());
        let particle_system_control_panel =
            ParticleSystemPreviewControlPanel::new(inspector_plugin.head, ctx);
        let camera_control_panel = CameraPreviewControlPanel::new(scene_viewer.frame(), ctx);
        let mesh_control_panel = MeshControlPanel::new(inspector_plugin.head, ctx);
        let audio_preview_panel = AudioPreviewPanel::new(inspector_plugin.head, ctx);
        let doc_window = DocWindow::new(ctx);
        let node_removal_dialog = NodeRemovalDialog::new(ctx);
        let scene_settings =
            SceneSettingsWindow::new(ctx, message_sender.clone(), engine.resource_manager.clone());

        let docking_manager;
        let root_grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(inner_size.width)
                .with_height(inner_size.height)
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
                                                                            inspector_plugin.window,
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
                            camera_control_panel.window,
                            navmesh_panel.window,
                            doc_window.window,
                            light_panel.window,
                            scene_settings.window,
                        ])
                        .build(ctx);
                    docking_manager
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let exit_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(100.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::text("Unsaved changes")),
        )
        .with_buttons(MessageBoxButtons::YesNoCancel)
        .build(ctx);

        let validation_message_box = MessageBoxBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
                .can_close(false)
                .can_minimize(false)
                .open(false)
                .with_title(WindowTitle::text("Validation failed!")),
        )
        .with_buttons(MessageBoxButtons::Ok)
        .build(ctx);

        let save_scene_dialog = SaveSceneConfirmationDialog::new(ctx);
        if let Some(layout) = settings.windows.layout.as_ref() {
            engine
                .user_interfaces
                .first_mut()
                .send_message(DockingManagerMessage::layout(
                    docking_manager,
                    MessageDirection::ToWidget,
                    layout.clone(),
                ));
        }

        let editor = Self {
            docking_manager,
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
            configurator,
            log,
            light_panel,
            command_stack_viewer,
            validation_message_box,
            settings,
            audio_panel,
            save_scene_dialog,
            mode: Mode::Edit,
            game_loop_data: GameLoopData {
                clock: Instant::now(),
                lag: 0.0,
            },
            build_window: None,
            scene_settings,
            particle_system_control_panel,
            camera_control_panel,
            mesh_control_panel,
            audio_preview_panel,
            node_removal_dialog,
            doc_window,
            plugins: EditorPluginsContainer::new()
                .with(ColliderPlugin::default())
                .with(TileMapEditorPlugin::default())
                .with(MaterialPlugin::default())
                .with(RagdollPlugin::default())
                .with(SettingsPlugin::default())
                .with(AnimationEditorPlugin::default())
                .with(AbsmEditorPlugin::default())
                .with(UiStatisticsPlugin::default())
                .with(CurveEditorPlugin::default())
                .with(ReflectionProbePlugin::default())
                .with(inspector_plugin),
            // Apparently, some window managers (like Wayland), does not send `Focused` event after the window
            // was created. So we must assume that the editor is focused by default, otherwise editor's thread
            // will sleep forever and the window won't come up.
            focused: true,
            update_loop_state: UpdateLoopState::default(),
            is_suspended: false,
            scene_node_context_menu,
            widget_context_menu,
            overlay_pass: None,
            highlighter: None,
            export_window: None,
            statistics_window: None,
            surface_data_viewer: None,
            processed_ui_messages: 0,
            styles,
            running_game_process: None,
            user_project_icon: None,
            user_project_name: Default::default(),
            user_project_version: Default::default(),
        };

        if let Some(data) = startup_data {
            editor.message_sender.send(Message::Configure {
                working_directory: if data.working_directory == PathBuf::default() {
                    std::env::current_dir().unwrap()
                } else {
                    data.working_directory
                },
            });

            for scene in data.scenes {
                if scene != PathBuf::default() {
                    editor.message_sender.send(Message::LoadScene(scene));
                }
            }
        } else {
            // Open configurator as usual.
            editor
                .engine
                .user_interfaces
                .first()
                .send_message(WindowMessage::open_modal(
                    editor.configurator.window,
                    MessageDirection::ToWidget,
                    true,
                    true,
                ));
        }

        editor
    }

    fn reload_settings(&mut self) {
        let old_subscribers = std::mem::take(&mut self.settings.subscribers);

        match Settings::load() {
            Ok(settings) => {
                self.settings = settings;
                self.settings.subscribers = old_subscribers;

                Log::info("Editor settings were reloaded successfully!");
            }
            Err(e) => {
                self.settings = Default::default();

                Log::warn(format!(
                    "Failed to load settings, fallback to default. Reason: {e:?}"
                ))
            }
        }

        self.menu
            .file_menu
            .update_recent_files_list(self.engine.user_interfaces.first_mut(), &self.settings);

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
            Err(e) => Log::info(format!("Failed to apply graphics settings! Reason: {e:?}")),
        }
    }

    fn add_scene(&mut self, entry: EditorSceneEntry) {
        self.try_leave_preview_mode();

        self.sync_to_model();
        self.poll_ui_messages();

        if let Some(path) = entry.path.as_ref() {
            if !self.settings.recent.scenes.contains(path) {
                self.settings.recent.scenes.push(path.clone());
                self.menu.file_menu.update_recent_files_list(
                    self.engine.user_interfaces.first_mut(),
                    &self.settings,
                );
            }
        }

        self.scenes.add_and_select(entry);

        self.scene_viewer
            .reset_camera_projection(self.engine.user_interfaces.first());

        self.on_scene_changed();
    }

    pub fn handle_hotkeys(&mut self, message: &UiMessage) {
        // A message could be handled already somewhere else (for example in a TextBox or any other
        // widget, that handles keyboard input), we must not respond to such messages.
        if message.handled() {
            return;
        }

        let modifiers = self.engine.user_interfaces.first_mut().keyboard_modifiers();
        let sender = self.message_sender.clone();
        let engine = &mut self.engine;

        if let Some(WidgetMessage::KeyDown(key)) = message.data() {
            let hot_key = HotKey::Some {
                code: *key,
                modifiers,
            };

            let mut processed = false;
            if let Some(scene) = self.scenes.current_scene_entry_mut() {
                if let Some(current_interaction_mode) = scene
                    .current_interaction_mode
                    .and_then(|current_mode| scene.interaction_modes.get_mut(&current_mode))
                {
                    processed |= current_interaction_mode.on_hot_key_pressed(
                        &hot_key,
                        &mut *scene.controller,
                        engine,
                        &self.settings,
                    );
                }
            }

            if !processed {
                let key_bindings = &self.settings.key_bindings;

                if hot_key == key_bindings.redo {
                    sender.send(Message::RedoCurrentSceneCommand);
                } else if hot_key == key_bindings.undo {
                    sender.send(Message::UndoCurrentSceneCommand);
                } else if hot_key == key_bindings.enable_select_mode {
                    sender.send(Message::SetInteractionMode(
                        SelectInteractionMode::type_uuid(),
                    ));
                } else if hot_key == key_bindings.enable_move_mode {
                    sender.send(Message::SetInteractionMode(MoveInteractionMode::type_uuid()));
                } else if hot_key == key_bindings.enable_rotate_mode {
                    sender.send(Message::SetInteractionMode(
                        RotateInteractionMode::type_uuid(),
                    ));
                } else if hot_key == key_bindings.enable_scale_mode {
                    sender.send(Message::SetInteractionMode(
                        ScaleInteractionMode::type_uuid(),
                    ));
                } else if hot_key == key_bindings.enable_navmesh_mode {
                    sender.send(Message::SetInteractionMode(EditNavmeshMode::type_uuid()));
                } else if hot_key == key_bindings.enable_terrain_mode {
                    sender.send(Message::SetInteractionMode(
                        TerrainInteractionMode::type_uuid(),
                    ));
                } else if hot_key == key_bindings.load_scene {
                    sender.send(Message::OpenLoadSceneDialog);
                } else if hot_key == key_bindings.run_game {
                    sender.send(Message::SwitchToBuildMode {
                        play_after_build: true,
                    });
                } else if hot_key == key_bindings.save_scene {
                    if let Some(entry) = self.scenes.current_scene_entry_ref() {
                        if let Some(path) = entry.path.as_ref() {
                            self.message_sender.send(Message::SaveScene {
                                id: entry.id,
                                path: path.clone(),
                            });
                        } else {
                            self.message_sender.send(Message::OpenSaveSceneDialog {
                                default_file_name: entry.default_file_name(),
                            });
                        }
                    }
                } else if hot_key == key_bindings.save_scene_as {
                    if let Some(entry) = self.scenes.current_scene_entry_ref() {
                        self.menu.file_menu.open_save_file_selector(
                            engine.user_interfaces.first_mut(),
                            entry.default_file_name(),
                        );
                    }
                } else if hot_key == key_bindings.save_all_scenes {
                    self.message_sender.send(Message::SaveAllScenes);
                } else if hot_key == key_bindings.copy_selection {
                    if let Some(entry) = self.scenes.current_scene_entry_mut() {
                        if let Some(graph_selection) = entry.selection.as_graph() {
                            if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
                                game_scene.clipboard.fill_from_selection(
                                    graph_selection,
                                    game_scene.scene,
                                    engine,
                                );
                            } else if let Some(ui_scene) =
                                entry.controller.downcast_mut::<UiScene>()
                            {
                                if let Some(selection) = entry.selection.as_ui() {
                                    ui_scene
                                        .clipboard
                                        .fill_from_selection(selection, &ui_scene.ui);
                                }
                            }
                        }
                    }
                } else if hot_key == key_bindings.paste {
                    if let Some(controller) = self.scenes.current_scene_controller_mut() {
                        if let Some(game_scene) = controller.downcast_mut::<GameScene>() {
                            if !game_scene.clipboard.is_empty() {
                                sender.do_command(PasteCommand::new(game_scene.scene_content_root));
                            }
                        } else if let Some(ui_scene) = controller.downcast_mut::<UiScene>() {
                            if !ui_scene.clipboard.is_empty() {
                                sender.do_command(PasteWidgetCommand::new(ui_scene.ui.root()));
                            }
                        }
                    }
                } else if hot_key == key_bindings.new_scene {
                    sender.send(Message::NewScene);
                } else if hot_key == key_bindings.close_scene {
                    if let Some(entry) = self.scenes.current_scene_entry_ref() {
                        sender.send(Message::CloseScene(entry.id));
                    }
                } else if hot_key == key_bindings.remove_selection {
                    if let Some(entry) = self.scenes.current_scene_entry_mut() {
                        if !entry.selection.is_empty() {
                            if entry.selection.is_graph() {
                                if let Some(game_scene) =
                                    entry.controller.downcast_mut::<GameScene>()
                                {
                                    if self.settings.general.show_node_removal_dialog
                                        && game_scene.is_current_selection_has_external_refs(
                                            &entry.selection,
                                            &engine.scenes[game_scene.scene].graph,
                                        )
                                    {
                                        sender.send(Message::OpenNodeRemovalDialog);
                                    } else {
                                        sender.send(Message::DoCommand(
                                            make_delete_selection_command(
                                                &entry.selection,
                                                game_scene,
                                                engine,
                                            ),
                                        ));
                                    }
                                }
                            } else if let Some(selection) = entry.selection.as_ui() {
                                if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
                                    sender.send(Message::DoCommand(
                                        selection.make_deletion_command(&ui_scene.ui),
                                    ));
                                }
                            }
                        }
                    }
                } else if hot_key == key_bindings.focus {
                    if let Some(entry) = self.scenes.current_scene_entry_mut() {
                        if let Some(selection) = entry.selection.as_graph() {
                            if let Some(first) = selection.nodes.first() {
                                sender.send(Message::FocusObject(*first));
                            }
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::KeyUp(key)) = message.data() {
            let hot_key = HotKey::Some {
                code: *key,
                modifiers,
            };

            if let Some(scene) = self.scenes.current_scene_entry_mut() {
                if let Some(current_interaction_mode) = scene
                    .current_interaction_mode
                    .and_then(|current_mode| scene.interaction_modes.get_mut(&current_mode))
                {
                    current_interaction_mode.on_hot_key_released(
                        &hot_key,
                        &mut *scene.controller,
                        engine,
                        &self.settings,
                    );
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &mut UiMessage) {
        // Prevent infinite message loops.
        if message.has_flags(MSG_SYNC_FLAG) {
            return;
        }

        for_each_plugin!(self.plugins => on_ui_message(message, self));

        let engine = &mut self.engine;

        self.save_scene_dialog
            .handle_ui_message(message, &self.message_sender, &self.scenes);

        let current_scene_entry = self.scenes.current_scene_entry_mut();

        self.configurator.handle_ui_message(message, engine);
        let inspector = self.plugins.get::<InspectorPlugin>();
        self.menu.handle_ui_message(
            message,
            MenuContext {
                engine,
                game_scene: current_scene_entry,
                panels: Panels {
                    scene_frame: self.scene_viewer.frame(),
                    inspector_window: inspector.window,
                    world_outliner_window: self.world_viewer.window,
                    asset_window: self.asset_browser.window,
                    light_panel: self.light_panel.window,
                    log_panel: self.log.window,
                    navmesh_panel: self.navmesh_panel.window,
                    audio_panel: self.audio_panel.window,
                    configurator_window: self.configurator.window,
                    command_stack_panel: self.command_stack_viewer.window,
                    scene_settings: &self.scene_settings,
                    export_window: &mut self.export_window,
                    statistics_window: &mut self.statistics_window,
                },
                settings: &mut self.settings,
            },
        );

        if let Some(surface_data_viewer) = self.surface_data_viewer.take() {
            self.surface_data_viewer = surface_data_viewer.handle_ui_message(message, engine);
        }

        let ui = engine.user_interfaces.first_mut();
        if let Some(build_window) = self.build_window.take() {
            self.build_window = build_window.handle_ui_message(message, ui, || {
                self.message_sender.send(Message::SwitchToEditMode)
            });
            if self.build_window.is_none() {
                if let Some((process, active)) = self.running_game_process.take() {
                    self.mode = Mode::Play { process, active };
                }
            }
        }
        if let Some(export_window) = self.export_window.as_mut() {
            export_window.handle_ui_message(
                message,
                ui,
                &self.message_sender,
                engine.resource_manager.clone(),
            );
        }
        if let Some(stats) = self.statistics_window.as_ref() {
            if let StatisticsWindowAction::Remove = stats.handle_ui_message(message, ui) {
                self.statistics_window.take();
            }
        }
        self.log.handle_ui_message(message, ui);
        self.asset_browser
            .handle_ui_message(message, engine, self.message_sender.clone());
        self.command_stack_viewer.handle_ui_message(message);
        self.scene_viewer.handle_ui_message(
            message,
            engine,
            &mut self.scenes,
            &mut self.settings,
            &self.mode,
        );

        let current_scene_entry = self.scenes.current_scene_entry_mut();

        if let Some(current_scene_entry) = current_scene_entry {
            if let Some(game_scene) = current_scene_entry.controller.downcast_mut::<GameScene>() {
                self.particle_system_control_panel.handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    game_scene,
                    engine,
                );
                self.camera_control_panel.handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    game_scene,
                    engine,
                );
                self.mesh_control_panel.handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    game_scene,
                    engine,
                    &self.message_sender,
                );
                self.audio_preview_panel.handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    game_scene,
                    engine,
                );

                self.audio_panel.handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    &self.message_sender,
                    engine,
                );
                self.node_removal_dialog.handle_ui_message(
                    &current_scene_entry.selection,
                    game_scene,
                    message,
                    engine,
                    &self.message_sender,
                );
                self.scene_settings
                    .handle_ui_message(message, &self.message_sender);

                self.navmesh_panel
                    .handle_message(message, &current_scene_entry.selection);

                if let Some(interaction_mode) = current_scene_entry
                    .current_interaction_mode
                    .and_then(|current_mode| {
                        current_scene_entry.interaction_modes.get_mut(&current_mode)
                    })
                {
                    interaction_mode.handle_ui_message(
                        message,
                        &current_scene_entry.selection,
                        game_scene,
                        engine,
                    );
                }

                self.scene_node_context_menu.borrow_mut().handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    game_scene,
                    engine,
                    &self.message_sender,
                    &self.settings,
                );
                self.world_viewer.handle_ui_message(
                    message,
                    &mut EditorSceneWrapper {
                        selection: &current_scene_entry.selection,
                        game_scene,
                        scene: &mut engine.scenes[game_scene.scene],
                        sender: &self.message_sender,
                        path: current_scene_entry.path.as_deref(),
                        resource_manager: &engine.resource_manager,
                        instantiation_scale: self.settings.model.instantiation_scale,
                    },
                    engine.user_interfaces.first(),
                    &mut self.settings,
                );

                self.light_panel
                    .handle_ui_message(message, game_scene, engine);
            } else if let Some(ui_scene) = current_scene_entry.controller.downcast_mut::<UiScene>()
            {
                self.world_viewer.handle_ui_message(
                    message,
                    &mut UiSceneWorldViewerDataProvider {
                        ui: &mut ui_scene.ui,
                        path: current_scene_entry.path.as_deref(),
                        selection: &current_scene_entry.selection,
                        sender: &self.message_sender,
                        resource_manager: &engine.resource_manager,
                    },
                    engine.user_interfaces.first(),
                    &mut self.settings,
                );

                self.widget_context_menu.borrow_mut().handle_ui_message(
                    message,
                    &current_scene_entry.selection,
                    ui_scene,
                    engine,
                    &self.message_sender,
                );
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
                            if first_unsaved.need_save() {
                                if let Some(path) = first_unsaved.path.as_ref() {
                                    self.message_sender.send(Message::SaveScene {
                                        id: first_unsaved.id,
                                        path: path.clone(),
                                    });

                                    self.message_sender
                                        .send(Message::CloseScene(first_unsaved.id));

                                    self.message_sender.send(Message::Exit {
                                        force: self.scenes.unsaved_scene_count() == 1,
                                    });
                                } else {
                                    self.message_sender.send(Message::OpenSaveSceneDialog {
                                        default_file_name: first_unsaved.default_file_name(),
                                    });
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
        if let Some(build_window) = self.build_window.take() {
            build_window.destroy(self.engine.user_interfaces.first());
        }

        let Some(entry) = self.scenes.current_scene_entry_ref() else {
            Log::err("Cannot enter build mode when there is no scene!");
            return;
        };

        let Some(path) = entry.path.as_ref().cloned() else {
            Log::err("Save you scene first!");
            return;
        };

        self.save_scene(entry.id, path.clone());

        let Some(build_profile) = self
            .settings
            .build
            .profiles
            .get(self.settings.build.selected_profile)
        else {
            Log::err("Selected build profile index is invalid.");
            return;
        };

        Log::info(format!(
            "Trying to run the game using command: {}",
            build_profile.run_command
        ));

        let mut command = build_profile.run_command.make_command();

        command
            .stdout(Stdio::piped())
            .arg("--")
            .arg("--override-scene")
            .arg(path);

        match command.spawn() {
            Ok(mut process) => {
                let active = Arc::new(AtomicBool::new(true));

                // Capture output from child process.
                let mut stdout = process.stdout.take().unwrap();
                let mut stderr = process.stderr.take().unwrap();
                let reader_active = active.clone();
                std::thread::spawn(move || {
                    while reader_active.load(Ordering::SeqCst) {
                        for line in BufReader::new(&mut stdout).lines().take(10).flatten() {
                            Log::info(line);
                        }
                    }
                });
                let reader_active = active.clone();
                std::thread::spawn(move || {
                    while reader_active.load(Ordering::SeqCst) {
                        for line in BufReader::new(&mut stderr).lines().take(10).flatten() {
                            Log::err(line);
                        }
                    }
                });

                self.mode = Mode::Play { active, process };

                self.on_mode_changed();
            }
            Err(e) => Log::err(format!("Failed to enter play mode: {e:?}")),
        }
    }

    fn set_build_mode(&mut self, play_after_build: bool) {
        if matches!(self.mode, Mode::Build { .. }) {
            Log::err("Cannot enter build mode when another build mode is active!");
            return;
        }

        if let Some(entry) = self.scenes.current_scene_entry_ref() {
            if entry.path.is_none() {
                Log::err("Save you scene first!");
                return;
            }
        }

        let Some(build_profile) = self
            .settings
            .build
            .profiles
            .get(self.settings.build.selected_profile)
        else {
            Log::err("Selected build profile index is invalid.");
            return;
        };

        let queue = build_profile
            .build_commands
            .iter()
            .cloned()
            .collect::<VecDeque<_>>();

        let old_mode = std::mem::replace(
            &mut self.mode,
            Mode::Build {
                queue,
                process: None,
                play_after_build,
            },
        );

        match old_mode {
            Mode::Edit => {}
            Mode::Build { .. } => {
                unreachable!();
            }
            Mode::Play { process, active } => {
                self.running_game_process = Some((process, active));
            }
        }

        let ui = self.engine.user_interfaces.first_mut();
        self.build_window = Some(BuildWindow::new("your game", &mut ui.build_ctx()));

        self.on_mode_changed();
    }

    fn set_editor_mode(&mut self) {
        match std::mem::replace(&mut self.mode, Mode::Edit) {
            Mode::Play { mut process, .. } => {
                Log::verify(process.kill());
                self.on_mode_changed();
            }
            Mode::Build { process, .. } => {
                if let Some(mut process) = process {
                    Log::verify(process.kill());
                }
                self.on_mode_changed();
            }
            _ => {}
        }
    }

    fn on_mode_changed(&mut self) {
        for_each_plugin!(self.plugins => on_mode_changed(self));

        let engine = &mut self.engine;
        let ui = engine.user_interfaces.first();
        self.scene_viewer.on_mode_changed(ui, &self.mode);
        self.world_viewer.on_mode_changed(ui, &self.mode);
        self.asset_browser.on_mode_changed(ui, &self.mode);
        self.command_stack_viewer.on_mode_changed(ui, &self.mode);
        self.audio_panel.on_mode_changed(ui, &self.mode);
        self.navmesh_panel.on_mode_changed(ui, &self.mode);
        self.menu.on_mode_changed(ui, &self.mode);
    }

    fn sync_to_model(&mut self) {
        for_each_plugin!(self.plugins => on_sync_to_model(self));

        let engine = &mut self.engine;

        self.menu.sync_to_model(
            self.scenes.current_scene_controller_ref().is_some(),
            engine.user_interfaces.first_mut(),
        );

        self.scene_viewer.sync_to_model(&self.scenes, engine);
        if let Some(exporter) = self.export_window.as_ref() {
            exporter.sync_to_model(engine.user_interfaces.first_mut());
        }

        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            self.command_stack_viewer.sync_to_model(
                current_scene_entry.command_stack.top,
                current_scene_entry.controller.command_names(
                    &mut current_scene_entry.command_stack,
                    &mut current_scene_entry.selection,
                    engine,
                ),
                engine.user_interfaces.first_mut(),
            );

            if let Some(game_scene) = current_scene_entry.controller.downcast_mut::<GameScene>() {
                self.scene_settings.sync_to_model(
                    false,
                    game_scene,
                    engine,
                    self.message_sender.clone(),
                );
                let sender = &self.message_sender;
                self.world_viewer.sync_to_model(
                    &EditorSceneWrapper {
                        selection: &current_scene_entry.selection,
                        game_scene,
                        scene: &mut engine.scenes[game_scene.scene],
                        sender,
                        path: current_scene_entry.path.as_deref(),
                        resource_manager: &engine.resource_manager,
                        instantiation_scale: self.settings.model.instantiation_scale,
                    },
                    engine.user_interfaces.first_mut(),
                    &self.settings,
                );

                self.audio_panel
                    .sync_to_model(&current_scene_entry.selection, game_scene, engine);
                self.navmesh_panel.sync_to_model(
                    engine,
                    &current_scene_entry.selection,
                    game_scene,
                );
            } else if let Some(ui_scene) = current_scene_entry.controller.downcast_mut::<UiScene>()
            {
                self.world_viewer.sync_to_model(
                    &UiSceneWorldViewerDataProvider {
                        ui: &mut ui_scene.ui,
                        path: current_scene_entry.path.as_deref(),
                        selection: &current_scene_entry.selection,
                        sender: &self.message_sender,
                        resource_manager: &engine.resource_manager,
                    },
                    engine.user_interfaces.first_mut(),
                    &self.settings,
                );
            }
        } else {
            self.world_viewer.clear(engine.user_interfaces.first());
        }
    }

    fn post_update(&mut self) {
        if let Some(entry) = self.scenes.current_scene_entry_mut() {
            if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                self.world_viewer.post_update(
                    &EditorSceneWrapper {
                        selection: &entry.selection,
                        game_scene,
                        scene: &mut self.engine.scenes[game_scene.scene],
                        sender: &self.message_sender,
                        path: entry.path.as_deref(),
                        resource_manager: &self.engine.resource_manager,
                        instantiation_scale: self.settings.model.instantiation_scale,
                    },
                    self.engine.user_interfaces.first_mut(),
                    &self.settings,
                );
            } else if let Some(ui_scene) = entry.controller.downcast_mut::<UiScene>() {
                self.world_viewer.post_update(
                    &UiSceneWorldViewerDataProvider {
                        ui: &mut ui_scene.ui,
                        path: entry.path.as_deref(),
                        selection: &entry.selection,
                        sender: &self.message_sender,
                        resource_manager: &self.engine.resource_manager,
                    },
                    self.engine.user_interfaces.first_mut(),
                    &self.settings,
                );
            }
        }

        for_each_plugin!(self.plugins => on_post_update(self));
    }

    fn do_current_scene_command(&mut self, command: Command) -> bool {
        let engine = &mut self.engine;
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            current_scene_entry.has_unsaved_changes |= command.is_significant();

            current_scene_entry.controller.do_command(
                &mut current_scene_entry.command_stack,
                command,
                &mut current_scene_entry.selection,
                engine,
            );

            true
        } else {
            false
        }
    }

    fn undo_current_scene_command(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            if let Some(command) = current_scene_entry.command_stack.top_command() {
                current_scene_entry.has_unsaved_changes |= command.is_significant();
            }

            current_scene_entry.controller.undo(
                &mut current_scene_entry.command_stack,
                &mut current_scene_entry.selection,
                engine,
            );

            true
        } else {
            false
        }
    }

    fn redo_current_scene_command(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            current_scene_entry.controller.redo(
                &mut current_scene_entry.command_stack,
                &mut current_scene_entry.selection,
                engine,
            );

            if let Some(command) = current_scene_entry.command_stack.top_command() {
                current_scene_entry.has_unsaved_changes |= command.is_significant();
            }

            true
        } else {
            false
        }
    }

    fn clear_current_scene_command_stack(&mut self) -> bool {
        let engine = &mut self.engine;
        if let Some(current_scene_entry) = self.scenes.current_scene_entry_mut() {
            current_scene_entry.controller.clear_command_stack(
                &mut current_scene_entry.command_stack,
                &mut current_scene_entry.selection,
                &mut engine.scenes,
            );
            true
        } else {
            false
        }
    }

    fn try_leave_preview_mode(&mut self) {
        if let Some(entry) = self.scenes.current_scene_entry_mut() {
            if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
                let engine = &mut self.engine;
                self.particle_system_control_panel
                    .leave_preview_mode(game_scene, engine);
                self.camera_control_panel
                    .leave_preview_mode(game_scene, engine);
                self.audio_preview_panel
                    .leave_preview_mode(game_scene, engine);
            }
        }

        for_each_plugin!(self.plugins => on_leave_preview_mode(self));
    }

    pub fn is_in_preview_mode(&mut self) -> bool {
        let mut is_any_plugin_in_preview_mode = false;
        let mut i = 0;
        while i < self.plugins.0.len() {
            if let Some(plugin) = self.plugins.0.get_mut(i).and_then(|p| p.take()) {
                is_any_plugin_in_preview_mode |= plugin.is_in_preview_mode(self);

                if let Some(entry) = self.plugins.0.get_mut(i) {
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
            || self.light_panel.is_in_preview_mode()
            || self.export_window.is_some()
            || is_any_plugin_in_preview_mode
            || self
                .scenes
                .current_scene_controller_ref()
                .is_some_and(|s| s.is_interacting())
            || stays_active
    }

    fn save_scene(&mut self, id: Uuid, path: PathBuf) {
        let path = match make_relative_path(path.clone()) {
            Ok(path) => path,
            Err(err) => {
                Log::err(format!(
                    "Failed to create relative path for {}. Reason: {err}",
                    path.display()
                ));
                return;
            }
        };

        // If there is some other open scene with the same name, then close it.
        for entry in self.scenes.entries.iter() {
            if entry.id != id && entry.path.as_ref() == Some(&path) {
                self.close_scene(entry.id);
                break;
            }
        }

        self.try_leave_preview_mode();

        let engine = &mut self.engine;
        if let Some(entry) = self.scenes.entry_by_scene_id_mut(id) {
            if !self.settings.recent.scenes.contains(&path) {
                self.settings.recent.scenes.push(path.clone());
                self.menu
                    .file_menu
                    .update_recent_files_list(engine.user_interfaces.first_mut(), &self.settings);
            }

            match entry.save(path.clone(), &self.settings, engine) {
                Ok(message) => {
                    self.scene_viewer.set_title(
                        engine.user_interfaces.first(),
                        format!("Scene Preview - {}", path.display()),
                    );
                    Log::info(message);

                    entry.has_unsaved_changes = false;
                }
                Err(message) => {
                    Log::err(message.clone());
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(MessageBoxMessage::open(
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
        let scene_path = match make_relative_path(scene_path) {
            Ok(path) => path,
            Err(err) => {
                Log::err(err.to_string());
                return;
            }
        };

        for entry in self.scenes.entries.iter() {
            if entry.path.as_ref() == Some(&scene_path) {
                self.set_current_scene(entry.id);
                return;
            }
        }

        if let Some(ext) = scene_path.extension() {
            if ext == "rgs" {
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
                        let scene = block_on(loader.0.finish());
                        let entry = EditorSceneEntry::new_game_scene(
                            scene,
                            Some(scene_path),
                            engine,
                            &mut self.settings,
                            self.message_sender.clone(),
                            &self.scene_viewer,
                            self.highlighter.clone(),
                        );
                        self.add_scene(entry);
                    }
                    Err(e) => {
                        Log::err(e.to_string());
                    }
                }
            } else if ext == "ui" {
                match block_on(UserInterface::load_from_file_ex(
                    &scene_path,
                    self.engine.widget_constructors.clone(),
                    self.engine.resource_manager.clone(),
                    &FsResourceIo,
                )) {
                    Ok(ui) => {
                        let entry = EditorSceneEntry::new_ui_scene(
                            ui,
                            Some(scene_path),
                            self.message_sender.clone(),
                            &self.scene_viewer,
                            &mut self.engine,
                            &self.settings,
                        );
                        self.add_scene(entry);
                    }
                    Err(e) => {
                        Log::err(e.to_string());
                    }
                }
            } else {
                Log::err(format!(
                    "{} is not a game scene or UI scene!",
                    scene_path.display()
                ));
            }
        }
    }

    fn exit(&mut self, force: bool) {
        let engine = &mut self.engine;
        if force {
            self.exit = true;
        } else if let Some(first_unsaved) = self.scenes.first_unsaved_scene() {
            engine
                .user_interfaces
                .first_mut()
                .send_message(MessageBoxMessage::open(
                    self.exit_message_box,
                    MessageDirection::ToWidget,
                    None,
                    Some(format!(
                        "There are unsaved changes in the {} scene. \
                    Do you wish to save them before exit?",
                        first_unsaved.name()
                    )),
                ));
        } else {
            self.exit = true;
        }
    }

    fn close_scene(&mut self, id: Uuid) -> bool {
        let closing_current_scene = self
            .scenes
            .current_scene_entry_ref()
            .map(|s| s.id == id)
            .unwrap_or_default();

        if closing_current_scene {
            self.try_leave_preview_mode();
        }

        let engine = &mut self.engine;
        if let Some(mut entry) = self.scenes.take_scene(id) {
            entry
                .controller
                .on_destroy(&mut entry.command_stack, engine, &mut entry.selection);

            if closing_current_scene {
                // Preview frame has scene frame texture assigned, it must be cleared explicitly,
                // otherwise it will show last rendered frame in preview which is not what we want.
                self.scene_viewer
                    .set_render_target(engine.user_interfaces.first(), None);
                // Set default title scene
                self.scene_viewer
                    .set_title(engine.user_interfaces.first(), "Scene Preview".to_string());
            }

            entry.before_drop(engine);

            if closing_current_scene {
                self.on_scene_changed();
            }

            true
        } else {
            false
        }
    }

    fn set_current_scene(&mut self, id: Uuid) {
        if self.scenes.set_current_scene(id) {
            self.on_scene_changed();
        }
    }

    fn on_scene_changed(&mut self) {
        let ui = &self.engine.user_interfaces.first();
        if let Some(entry) = self.scenes.current_scene_entry_ref() {
            if entry.controller.downcast_ref::<GameScene>().is_some() {
                self.world_viewer.item_context_menu = Some(self.scene_node_context_menu.clone());
            } else if entry.controller.downcast_ref::<UiScene>().is_some() {
                self.world_viewer.item_context_menu = Some(self.widget_context_menu.clone());
            } else {
                self.world_viewer.item_context_menu = None;
            }

            self.menu.on_scene_changed(&*entry.controller, ui);
        }

        self.world_viewer.clear(ui);

        self.poll_ui_messages();

        self.world_viewer.sync_selection = true;

        self.scene_viewer
            .on_current_scene_changed(self.scenes.current_scene_entry_mut(), &mut self.engine);

        for_each_plugin!(self.plugins => on_scene_changed(self));

        self.sync_to_model();
        self.poll_ui_messages();
    }

    fn create_new_scene(&mut self) {
        let entry = EditorSceneEntry::new_game_scene(
            Scene::new(),
            None,
            &mut self.engine,
            &mut self.settings,
            self.message_sender.clone(),
            &self.scene_viewer,
            self.highlighter.clone(),
        );
        self.add_scene(entry);
    }

    fn create_new_ui_scene(&mut self) {
        let mut ui = UserInterface::new(Vector2::new(200.0, 200.0));

        // Create test content.
        ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(160.0)
                .with_height(32.0)
                .with_desired_position(Vector2::new(20.0, 20.0)),
        )
        .with_text("Click Me!")
        .build(&mut ui.build_ctx());

        TextBuilder::new(WidgetBuilder::new().with_desired_position(Vector2::new(300.0, 300.0)))
            .with_text("This is some text.")
            .build(&mut ui.build_ctx());

        let entry = EditorSceneEntry::new_ui_scene(
            ui,
            None,
            self.message_sender.clone(),
            &self.scene_viewer,
            &mut self.engine,
            &self.settings,
        );
        self.add_scene(entry);
    }

    fn configure(&mut self, working_directory: PathBuf) {
        assert!(self.scenes.is_empty());

        self.asset_browser.clear_preview(&mut self.engine);

        let current_working_directory = std::env::current_dir().unwrap();
        if current_working_directory != working_directory {
            std::env::set_current_dir(working_directory.clone()).unwrap();
            self.engine.resource_manager.update_or_load_registry();
            self.reload_settings();
            self.load_layout();
        }

        let engine = &mut self.engine;

        let graphics_context = engine.graphics_context.as_initialized_mut();

        graphics_context.window.set_title(&format!(
            "FyroxEd{} {}{}: {}",
            self.user_project_name,
            *EDITOR_VERSION,
            self.user_project_version,
            working_directory.to_string_lossy()
        ));

        match FileSystemWatcher::new(&working_directory, Duration::from_secs(1)) {
            Ok(watcher) => {
                engine.resource_manager.state().set_watcher(Some(watcher));
            }
            Err(e) => {
                Log::err(format!("Unable to create resource watcher. Reason {e:?}"));
            }
        }

        engine.resource_manager.state().destroy_unused_resources();

        self.asset_browser
            .set_working_directory(engine, &working_directory, &self.message_sender);

        self.world_viewer
            .on_configure(engine.user_interfaces.first(), &self.settings);

        Log::info(format!(
            "New working directory was successfully set: {working_directory:?}"
        ));
    }

    fn poll_ui_messages(&mut self) -> usize {
        let mut processed = 0;

        while let Some(mut ui_message) = self.engine.user_interfaces.first_mut().poll_message() {
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

    fn handle_modes(&mut self, dt: f32) {
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

                            Log::warn(format!("Game was closed: {status:?}"))
                        }
                    }
                    Err(err) => Log::err(format!("Failed to wait for game process: {err:?}")),
                }
            }
            Mode::Build {
                ref mut process,
                ref mut queue,
                play_after_build,
            } => {
                if process.is_none() {
                    if let Some(build_command) = queue.pop_front() {
                        Log::info(format!("Trying to run build command: {build_command}"));

                        match build_command
                            .make_command()
                            .stderr(Stdio::piped())
                            .stdout(Stdio::piped())
                            .spawn()
                        {
                            Ok(mut new_process) => {
                                if let Some(build_window) = self.build_window.as_mut() {
                                    build_window.listen(
                                        (
                                            new_process.stderr.take().unwrap(),
                                            new_process.stdout.take().unwrap(),
                                        ),
                                        self.engine.user_interfaces.first(),
                                    );
                                }

                                *process = Some(new_process);
                            }
                            Err(e) => Log::err(format!("Failed to enter build mode: {e:?}")),
                        }
                    } else {
                        Log::warn("Empty build command queue!");
                        self.mode = Mode::Edit;
                        self.on_mode_changed();
                        return;
                    }
                }

                if let Some(process_ref) = process {
                    if let Some(build_window) = self.build_window.as_mut() {
                        build_window.update(self.engine.user_interfaces.first(), dt);
                    }

                    match process_ref.try_wait() {
                        Ok(status) => {
                            if let Some(status) = status {
                                let success_code = 0;
                                let wtf_code = 12345;
                                let code = status.code().unwrap_or(wtf_code);
                                if code != success_code {
                                    Log::err("Failed to build the game!");
                                    self.mode = Mode::Edit;
                                    self.on_mode_changed();
                                } else if queue.is_empty() && play_after_build {
                                    self.set_play_mode();
                                } else {
                                    let ui = self.engine.user_interfaces.first();
                                    if queue.is_empty() {
                                        if let Some(build_window) = self.build_window.take() {
                                            build_window.destroy(ui);
                                        }
                                        if let Some((process, active)) =
                                            self.running_game_process.take()
                                        {
                                            self.mode = Mode::Play { process, active };
                                            self.on_mode_changed();
                                        } else {
                                            self.mode = Mode::Edit;
                                            self.on_mode_changed();
                                        }
                                    } else {
                                        if let Some(build_window) = self.build_window.as_mut() {
                                            build_window.reset(ui);
                                        }
                                        // Continue on next command.
                                        *process = None;
                                    }
                                }
                            }
                        }
                        Err(err) => Log::err(format!("Failed to wait for game process: {err:?}")),
                    }
                }
            }
            _ => {}
        }
    }

    fn update(&mut self, dt: f32) {
        for_each_plugin!(self.plugins => on_update(self));

        self.handle_modes(dt);

        let ui = self.engine.user_interfaces.first_mut();

        if let Some(active_tooltip) = ui.active_tooltip() {
            if !active_tooltip.shown {
                // Keep the editor running until the current tooltip is not shown.
                self.update_loop_state.request_update_in_next_frame();
            }
        }

        self.log.update(self.settings.general.max_log_entries, ui);
        if let Some(export_window) = self.export_window.as_mut() {
            export_window.update(ui);
        }

        self.asset_browser
            .update(&mut self.engine, &self.message_sender);
        if let Some(surface_data_viewer) = self.surface_data_viewer.as_mut() {
            surface_data_viewer.update(&mut self.engine);
        }

        self.scene_viewer
            .pre_update(&self.settings, &mut self.engine);
        if let Some(entry) = self.scenes.current_scene_entry_ref() {
            if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                if let Some(stats) = self.statistics_window.as_ref() {
                    stats.update(game_scene.scene, &self.engine);
                }

                self.light_panel.update(game_scene, &mut self.engine);
                self.audio_preview_panel
                    .update(&entry.selection, game_scene, &self.engine);
                self.scene_viewer.update(game_scene, &mut self.engine);
            }
        }

        if let Some(overlay_pass) = self.overlay_pass.as_ref() {
            overlay_pass.borrow_mut().pictogram_size = self.settings.debugging.pictogram_size;
        }

        self.processed_ui_messages = 0;
        let mut iterations = 1;
        while iterations > 0 {
            iterations -= 1;

            let ui_messages_processed_count = self.poll_ui_messages();
            self.processed_ui_messages += ui_messages_processed_count;

            let mut needs_sync = false;

            let mut editor_messages_processed_count = 0;
            while let Ok(message) = self.message_receiver.try_recv() {
                for_each_plugin!(self.plugins => on_message(&message, self));

                editor_messages_processed_count += 1;

                self.save_scene_dialog
                    .handle_message(&message, &self.message_sender);

                if let Some(entry) = self.scenes.current_scene_entry_mut() {
                    if let Some(game_scene) = entry.controller.downcast_mut::<GameScene>() {
                        self.particle_system_control_panel.handle_message(
                            &message,
                            &entry.selection,
                            game_scene,
                            &mut self.engine,
                        );
                        self.camera_control_panel.handle_message(
                            &message,
                            &entry.selection,
                            game_scene,
                            &mut self.engine,
                        );
                        self.mesh_control_panel.handle_message(
                            &message,
                            &entry.selection,
                            game_scene,
                            &mut self.engine,
                        );
                        self.audio_preview_panel.handle_message(
                            &message,
                            &entry.selection,
                            game_scene,
                            &mut self.engine,
                        );
                    }
                    needs_sync |=
                        entry
                            .controller
                            .on_message(&message, &entry.selection, &mut self.engine);
                }
                self.scene_viewer.handle_message(&message, &mut self.engine);

                match message {
                    Message::DoCommand(command) => {
                        needs_sync |= self.do_current_scene_command(command);
                    }
                    Message::UndoCurrentSceneCommand => {
                        needs_sync |= self.undo_current_scene_command();
                    }
                    Message::RedoCurrentSceneCommand => {
                        needs_sync |= self.redo_current_scene_command();
                    }
                    Message::ClearCurrentSceneCommandStack => {
                        needs_sync |= self.clear_current_scene_command_stack();
                    }
                    Message::SelectionChanged { .. } => {
                        self.world_viewer.sync_selection = true;
                    }
                    Message::SaveScene { id: scene, path } => self.save_scene(scene, path),
                    Message::LoadScene(scene_path) => {
                        self.load_scene(scene_path);
                        needs_sync = true;
                    }
                    Message::SetInteractionMode(mode_kind) => {
                        if let Some(game_scene_entry) = self.scenes.current_scene_entry_mut() {
                            game_scene_entry
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
                    Message::NewUiScene => {
                        self.create_new_ui_scene();
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
                    Message::OpenNodeRemovalDialog => {
                        if let Some(entry) = self.scenes.current_scene_entry_ref() {
                            // TODO
                            if let Some(game_scene) = entry.controller.downcast_ref::<GameScene>() {
                                self.node_removal_dialog.open(
                                    &entry.selection,
                                    game_scene,
                                    &self.engine,
                                )
                            }
                        }
                    }
                    Message::SetAssetBrowserCurrentDir(path) => {
                        self.asset_browser
                            .request_current_path(path, self.engine.user_interfaces.first());
                    }
                    Message::ShowInAssetBrowser(path) => {
                        self.asset_browser
                            .locate_path(self.engine.user_interfaces.first(), path);
                    }
                    Message::LocateObject { handle } => self
                        .world_viewer
                        .try_locate_object(handle, self.engine.user_interfaces.first()),
                    Message::SwitchMode => match self.mode {
                        Mode::Edit => self.set_build_mode(true),
                        _ => self.set_editor_mode(),
                    },
                    Message::SwitchToBuildMode { play_after_build } => {
                        self.set_build_mode(play_after_build)
                    }
                    Message::SwitchToEditMode => self.set_editor_mode(),
                    Message::OpenLoadSceneDialog => {
                        self.menu
                            .open_load_file_selector(self.engine.user_interfaces.first_mut());
                    }
                    Message::OpenSaveSceneDialog { default_file_name } => {
                        self.menu.open_save_file_selector(
                            self.engine.user_interfaces.first_mut(),
                            default_file_name,
                        );
                    }
                    Message::OpenSaveSceneConfirmationDialog { id, action } => {
                        self.save_scene_dialog.open(
                            self.engine.user_interfaces.first(),
                            id,
                            &self.scenes,
                            action,
                        );
                    }
                    Message::ForceSync => {
                        needs_sync = true;
                    }
                    Message::SaveAllScenes => {
                        for scene in self.scenes.iter() {
                            if let Some(path) = scene.path.clone() {
                                self.message_sender
                                    .send(Message::SaveScene { id: scene.id, path })
                            }
                        }
                    }
                    Message::ShowDocumentation(doc) => {
                        self.doc_window
                            .open(doc, self.engine.user_interfaces.first());
                    }
                    Message::SaveLayout => {
                        self.save_layout();
                    }
                    Message::LoadLayout => {
                        self.load_layout();
                    }
                    Message::ViewSurfaceData(data) => {
                        let mut viewer = SurfaceDataViewer::new(&mut self.engine);
                        viewer.open(data, &mut self.engine);
                        self.surface_data_viewer = Some(viewer);
                    }
                    Message::SyncInteractionModes => {
                        self.scene_viewer.sync_interaction_modes(
                            self.scenes.current_scene_entry_mut(),
                            self.engine.user_interfaces.first_mut(),
                        );
                    }
                    _ => (),
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

        if let Some(entry) = self.scenes.current_scene_entry_mut() {
            let controller = &mut entry.controller;

            let screen_bounds = self
                .scene_viewer
                .frame_bounds(self.engine.user_interfaces.first());
            if let Some(new_render_target) = controller.update(
                &entry.selection,
                &mut self.engine,
                dt,
                entry.path.as_deref(),
                &mut self.settings,
                screen_bounds,
            ) {
                self.scene_viewer.set_render_target(
                    self.engine.user_interfaces.first(),
                    Some(new_render_target),
                );
            }

            if let Some(interaction_mode) = entry
                .current_interaction_mode
                .and_then(|current_mode| entry.interaction_modes.get_mut(&current_mode))
            {
                interaction_mode.update(
                    &entry.selection,
                    &mut **controller,
                    &mut self.engine,
                    &self.settings,
                );
            }
        }

        if self.settings.try_save() {
            let ui = self.engine.user_interfaces.first_mut();
            if let Some(style) = self.styles.get(&self.settings.general.style) {
                if style != ui.style() {
                    ui.set_style(style.clone());
                }
            }
        }
    }

    fn save_layout(&mut self) {
        let ui = self.engine.user_interfaces.first();
        let layout = ui
            .node(self.docking_manager)
            .query_component::<DockingManager>()
            .unwrap()
            .layout(ui);
        self.settings.windows.layout = Some(layout);
    }

    fn load_layout(&mut self) {
        if let Some(layout) = self.settings.windows.layout.as_ref() {
            self.engine
                .user_interfaces
                .first_mut()
                .send_message(DockingManagerMessage::layout(
                    self.docking_manager,
                    MessageDirection::ToWidget,
                    layout.clone(),
                ));
        }
    }

    pub fn add_game_plugin<P>(&mut self, plugin: P)
    where
        P: Plugin + 'static,
    {
        let inspector = self.plugins.get::<InspectorPlugin>();
        *inspector.property_editors.context_type_id.lock() = plugin.type_id();
        inspector
            .property_editors
            .merge(plugin.register_property_editors());
        self.engine.add_plugin(plugin)
    }

    /// Tries to add a new dynamic plugin. This method attempts to load a dynamic library by the
    /// given path and searches for `fyrox_plugin` function. This function is called to create a
    /// plugin instance. This method will fail if there's no dynamic library at the given path or
    /// the `fyrox_plugin` function is not found.
    ///
    /// # Hot reloading
    ///
    /// This method can enable hot reloading for the plugin, by setting `reload_when_changed` parameter
    /// to `true`. When enabled, the engine will clone the library to implementation-defined path
    /// and load it. It will setup file system watcher to receive changes from the OS and reload
    /// the plugin.
    pub fn add_dynamic_plugin<P>(
        &mut self,
        path: P,
        reload_when_changed: bool,
        use_relative_paths: bool,
    ) -> Result<(), String>
    where
        P: AsRef<Path> + 'static,
    {
        self.add_dynamic_plugin_custom(DyLibDynamicPlugin::new(
            path,
            reload_when_changed,
            use_relative_paths,
        )?)
    }

    pub fn add_dynamic_plugin_custom<P>(&mut self, plugin: P) -> Result<(), String>
    where
        P: DynamicPlugin + 'static,
    {
        let plugin = self.engine.add_dynamic_plugin_custom(plugin);
        let inspector = self.plugins.get::<InspectorPlugin>();
        *inspector.property_editors.context_type_id.lock() = plugin.type_id();
        inspector
            .property_editors
            .merge(plugin.register_property_editors());
        Ok(())
    }

    pub fn add_editor_plugin<P>(&mut self, plugin: P)
    where
        P: EditorPlugin + 'static,
    {
        self.plugins.add(plugin);
    }

    pub fn is_active(&self) -> bool {
        !self.update_loop_state.is_suspended()
            && (self.focused || !self.settings.general.suspend_unfocused_editor)
            // Keep the editor active if user holds any mouse button.
            || self.engine.user_interfaces.first().captured_node().is_some()
    }

    fn on_resumed(&mut self, evt: &EventLoopWindowTarget<()>) {
        let engine = &mut self.engine;

        engine.initialize_graphics_context(evt).unwrap();

        let graphics_context = engine.graphics_context.as_initialized_mut();

        graphics_context.set_window_icon_from_memory(
            self.user_project_icon
                .as_deref()
                .unwrap_or(include_bytes!("../resources/icon.png")),
        );

        // High-DPI screen support
        Log::info(format!(
            "UI scaling of your OS is: {}",
            graphics_context.window.scale_factor()
        ));

        set_ui_scaling(
            engine.user_interfaces.first(),
            graphics_context.window.scale_factor() as f32,
        );

        let overlay_pass = OverlayRenderPass::new(graphics_context.renderer.graphics_server());
        graphics_context
            .renderer
            .add_render_pass(overlay_pass.clone());
        self.overlay_pass = Some(overlay_pass);

        let highlighter = HighlightRenderPass::new(
            &*graphics_context.renderer.server,
            self.settings.windows.window_size.x as usize,
            self.settings.windows.window_size.y as usize,
        );
        graphics_context
            .renderer
            .add_render_pass(highlighter.clone());
        self.highlighter = Some(highlighter);

        match graphics_context
            .renderer
            .set_quality_settings(&self.settings.graphics.quality)
        {
            Ok(_) => {
                Log::info("Graphics settings were applied successfully!");
            }
            Err(e) => Log::err(format!("Failed to apply graphics settings! Reason: {e:?}")),
        }
    }

    fn on_suspended(&mut self) {
        self.overlay_pass.take();
        self.highlighter.take();

        self.engine.destroy_graphics_context().unwrap();
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
                            Mode::Build {
                                ref mut process, ..
                            } => {
                                if let Some(process) = process {
                                    let _ = process.kill();
                                }
                            }
                            Mode::Play {
                                ref mut process, ..
                            } => {
                                let _ = process.kill();
                            }
                        }
                    }
                }
                Event::Resumed => {
                    self.on_resumed(window_target);
                }
                Event::Suspended => {
                    self.on_suspended();
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
                                    format!("Failed to set renderer size! Reason: {e:?}"),
                                );
                            }

                            let window = &self.engine.graphics_context.as_initialized_ref().window;

                            let logical_size = size.to_logical(window.scale_factor());
                            self.engine.user_interfaces.first_mut().send_message(
                                WidgetMessage::width(
                                    self.root_grid,
                                    MessageDirection::ToWidget,
                                    logical_size.width,
                                ),
                            );
                            self.engine.user_interfaces.first_mut().send_message(
                                WidgetMessage::height(
                                    self.root_grid,
                                    MessageDirection::ToWidget,
                                    logical_size.height,
                                ),
                            );

                            if size.width > 0 && size.height > 0 {
                                self.settings.windows.window_size.x = size.width as f32;
                                self.settings.windows.window_size.y = size.height as f32;
                            }

                            self.settings.windows.window_maximized = window.is_maximized();
                        }
                        WindowEvent::Focused(focused) => {
                            self.focused = *focused;
                        }
                        WindowEvent::Moved(new_position) => {
                            // Allow the window to go outside the screen bounds by a little. This
                            // happens when the window is maximized.
                            if new_position.x > -50 && new_position.y > -50 {
                                self.settings.windows.window_position.x = new_position.x as f32;
                                self.settings.windows.window_position.y = new_position.y as f32;
                            }
                        }
                        WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                            set_ui_scaling(
                                self.engine.user_interfaces.first(),
                                *scale_factor as f32,
                            );
                        }
                        WindowEvent::RedrawRequested => {
                            if self.is_active() {
                                if let Some(entry) = self.scenes.current_scene_entry_mut() {
                                    entry
                                        .controller
                                        .on_before_render(&entry.selection, &mut self.engine);
                                }

                                self.engine.render().unwrap();

                                if let Some(scene) = self.scenes.current_scene_controller_mut() {
                                    scene.on_after_render(&mut self.engine);
                                }
                            }
                        }
                        _ => (),
                    }

                    // Any action in the window, other than a redraw request forces the editor to
                    // do another update pass which then pushes a redraw request to the event
                    // queue. This check prevents infinite loop of this kind.
                    if !matches!(event, WindowEvent::RedrawRequested) {
                        self.update_loop_state.request_update_in_current_frame();
                    }

                    if let Some(os_event) = translate_event(event) {
                        self.engine
                            .user_interfaces
                            .first_mut()
                            .process_os_event(&os_event);
                    }
                }
                Event::LoopExiting => {
                    let ids = self.scenes.entries.iter().map(|e| e.id).collect::<Vec<_>>();
                    for id in ids {
                        self.close_scene(id);
                    }

                    self.settings.force_save();

                    for_each_plugin!(self.plugins => on_exit(&mut self));
                }
                _ => {
                    if self.is_active() {
                        if self.is_suspended {
                            for_each_plugin!(self.plugins => on_resumed(&mut self));
                            self.is_suspended = false;
                        }
                    } else if !self.is_suspended {
                        for_each_plugin!(self.plugins => on_suspended(&mut self));
                        self.is_suspended = true;
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
    let elapsed = editor.game_loop_data.clock.elapsed().as_secs_f32();
    editor.game_loop_data.clock = Instant::now();
    editor.game_loop_data.lag += elapsed;

    while editor.game_loop_data.lag >= FIXED_TIMESTEP {
        editor.game_loop_data.lag -= FIXED_TIMESTEP;

        let mut switches = FxHashMap::default();

        for other_game_scene_entry in editor.scenes.entries.iter() {
            if let Some(other_game_scene) = other_game_scene_entry
                .controller
                .downcast_ref::<GameScene>()
            {
                if let Some(current_game_scene) = editor
                    .scenes
                    .current_scene_controller_ref()
                    .and_then(|e| e.downcast_ref::<GameScene>())
                {
                    switches.insert(
                        current_game_scene.scene,
                        current_game_scene.graph_switches.clone(),
                    );

                    if current_game_scene.scene == other_game_scene.scene {
                        continue;
                    }
                }

                // Other scenes will be paused.
                switches.insert(
                    other_game_scene.scene,
                    GraphUpdateSwitches {
                        paused: true,
                        ..Default::default()
                    },
                );
            }
        }

        editor.engine.pre_update(
            FIXED_TIMESTEP,
            ApplicationLoopController::WindowTarget(window_target),
            &mut editor.game_loop_data.lag,
            switches,
        );

        let mut need_reload_plugins = false;
        for plugin_index in 0..editor.engine.plugins().len() {
            let plugin = &editor.engine.plugins()[plugin_index];

            if let PluginContainer::Dynamic(plugin) = plugin {
                let plugin_type_id = plugin.as_loaded_ref().type_id();

                if plugin.is_reload_needed_now() {
                    // Clear command stacks for scenes. This is mandatory step, because command stack
                    // could contain objects from plugins and any attempt to use them after the plugin is
                    // unloaded will cause crash.
                    for i in 0..editor.scenes.entries.len() {
                        let entry = &mut editor.scenes.entries[i];
                        entry.controller.clear_command_stack(
                            &mut entry.command_stack,
                            &mut entry.selection,
                            &mut editor.engine.scenes,
                        );
                        entry.selection = Default::default();

                        Log::warn(format!("Command stack flushed for scene {i}"));
                    }

                    editor.message_sender.send(Message::SelectionChanged {
                        old_selection: Default::default(),
                    });
                    editor.message_sender.send(Message::ForceSync);

                    // Remove property editors that were created from the plugin.
                    let inspector = editor.plugins.get_mut::<InspectorPlugin>();
                    let mut definitions = inspector.property_editors.definitions_mut();

                    let mut to_be_removed = Vec::new();
                    for (type_id, entry) in &mut *definitions {
                        if entry.source_type_id == plugin_type_id {
                            to_be_removed.push(*type_id);
                        }
                    }

                    for type_id in to_be_removed {
                        definitions.remove(&type_id);
                    }

                    need_reload_plugins = true;
                }
            }
        }

        editor.update(FIXED_TIMESTEP);

        editor.engine.post_update(
            FIXED_TIMESTEP,
            &Default::default(),
            &mut editor.game_loop_data.lag,
            ApplicationLoopController::WindowTarget(window_target),
        );

        if need_reload_plugins {
            let on_plugin_reloaded = |plugin: &dyn Plugin| {
                let inspector = editor.plugins.get_mut::<InspectorPlugin>();
                *inspector.property_editors.context_type_id.lock() = plugin.type_id();
                inspector
                    .property_editors
                    .merge(plugin.register_property_editors());
            };

            editor.engine.handle_plugins_hot_reloading(
                FIXED_TIMESTEP,
                ApplicationLoopController::WindowTarget(window_target),
                &mut editor.game_loop_data.lag,
                on_plugin_reloaded,
            );
        }

        editor.post_update();

        if editor.game_loop_data.lag >= 1.5 * FIXED_TIMESTEP {
            break;
        }
    }

    let window = &editor.engine.graphics_context.as_initialized_ref().window;
    window.set_cursor_icon(translate_cursor_icon(
        editor.engine.user_interfaces.first_mut().cursor(),
    ));
    window.request_redraw();

    if !editor.is_in_preview_mode() {
        editor.update_loop_state.decrease_counter();
    }
}
