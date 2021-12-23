#![forbid(unsafe_code)]
#![allow(irrefutable_let_patterns)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::eval_order_dependence)]
// These are useless.
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::inconsistent_struct_constructor)]

extern crate rg3d;
#[macro_use]
extern crate lazy_static;
extern crate directories;

mod asset;
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
mod project_dirs;
mod scene;
mod scene_viewer;
mod settings;
mod utils;
mod world;

use crate::{
    asset::{AssetBrowser, AssetItem, AssetKind},
    command::{panel::CommandStackViewer, Command, CommandStack},
    configurator::Configurator,
    curve_editor::CurveEditorWindow,
    gui::make_dropdown_list_option,
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
    log::Log,
    material::MaterialEditor,
    menu::{Menu, MenuContext, Panels},
    overlay::OverlayRenderPass,
    scene::{
        commands::{
            graph::AddModelCommand, make_delete_selection_command, mesh::SetMeshTextureCommand,
            particle_system::SetParticleSystemTextureCommand, sound::DeleteSoundSourceCommand,
            sprite::SetSpriteTextureCommand, ChangeSelectionCommand, CommandGroup, PasteCommand,
            SceneCommand, SceneContext,
        },
        EditorScene, Selection,
    },
    scene_viewer::SceneViewer,
    settings::{Settings, SettingsSectionKind},
    utils::path_fixer::PathFixer,
    world::{graph::selection::GraphSelection, WorldViewer},
};
use rg3d::scene::camera::Projection;
use rg3d::{
    core::{
        algebra::{Point3, Vector2},
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        parking_lot::Mutex,
        pool::{ErasedHandle, Handle},
        scope_profile,
        sstorage::ImmutableString,
    },
    dpi::LogicalSize,
    engine::{
        resource_manager::{MaterialSearchOptions, TextureImportOptions},
        Engine,
    },
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    gui::{
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        dock::{DockingManagerBuilder, TileBuilder, TileContent},
        draw,
        dropdown_list::{DropdownListBuilder, DropdownListMessage},
        file_browser::{FileBrowserMode, FileSelectorBuilder, FileSelectorMessage, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        message::{KeyCode, MessageDirection, MouseButton, UiMessage},
        messagebox::{MessageBoxBuilder, MessageBoxButtons, MessageBoxMessage, MessageBoxResult},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::{TextBoxBuilder, TextBoxMessage},
        ttf::Font,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
    material::{Material, PropertyValue},
    resource::texture::{CompressionOptions, Texture, TextureKind, TextureState},
    scene::{
        debug::{Line, SceneDrawingContext},
        graph::Graph,
        mesh::{
            buffer::{VertexAttributeUsage, VertexReadTrait},
            Mesh,
        },
        node::Node,
        Scene,
    },
    utils::{into_gui_texture, log::MessageKind, translate_cursor_icon, translate_event},
};
use std::any::TypeId;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    str::from_utf8,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Instant,
};

pub const MSG_SYNC_FLAG: u64 = 1;

pub fn send_sync_message(ui: &UserInterface, mut msg: UiMessage) {
    msg.flags = MSG_SYNC_FLAG;
    ui.send_message(msg);
}

type GameEngine = rg3d::engine::Engine;

lazy_static! {
    // This checks release.toml debug handle and at
    // the same time checks if program is installed
    static ref DEBUG_HANDLE: bool = {
        let release_toml = project_dirs::resources_dir("release.toml");
        if release_toml.exists() {
            let file = fs::read(release_toml).unwrap();
            from_utf8(&file)
                .unwrap()
                .parse::<toml::Value>()
                .unwrap()["debug-mode"]
                .as_bool()
                .unwrap()
        } else {
            true
        }
    };

    // This constant gives DEBUG_HANDLE value to config_dir and data_dir
    // functions and checks if config and data dir are created.
    static ref TEST_EXISTENCE: bool = {
        if !(*DEBUG_HANDLE) {
            // We check if config and data dir exists
            if !project_dirs::data_dir("").exists() {
                // If there's aren't any, we create them.
                fs::create_dir(project_dirs::config_dir("")).unwrap();
                fs::create_dir(project_dirs::data_dir("")).unwrap();
            }

            true
        } else {
            false
        }
    };

    static ref CONFIG_DIR: Mutex<PathBuf> = Mutex::new(project_dirs::working_config_dir(""));
    static ref DATA_DIR: Mutex<PathBuf> = Mutex::new(project_dirs::working_data_dir(""));
}

pub fn load_image(data: &[u8]) -> Option<draw::SharedTexture> {
    Some(into_gui_texture(
        Texture::load_from_memory(data, CompressionOptions::NoCompression).ok()?,
    ))
}

pub fn make_color_material(color: Color) -> Arc<Mutex<Material>> {
    let mut material = Material::standard();
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

pub struct ModelImportDialog {
    // View
    pub window: Handle<UiNode>,
    options: Handle<UiNode>,
    path_field: Handle<UiNode>,
    path_selector: Handle<UiNode>,
    select_path: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    path_selection_section: Handle<UiNode>,

    // Data model
    model_path: PathBuf,
    material_search_options: MaterialSearchOptions,
}

impl ModelImportDialog {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let options;
        let select_path;
        let path_field;
        let ok;
        let cancel;
        let path_selection_section;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(135.0))
            .open(false)
            .with_title(WindowTitle::text("Import Model"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text("Please select the material search options.")
                            .build(ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(1)
                                    .with_child(
                                        TextBuilder::new(WidgetBuilder::new().on_column(0))
                                            .with_text("Options")
                                            .with_vertical_text_alignment(VerticalAlignment::Center)
                                            .build(ctx),
                                    )
                                    .with_child({
                                        options = DropdownListBuilder::new(
                                            WidgetBuilder::new().on_column(1),
                                        )
                                        .with_items(vec![
                                            make_dropdown_list_option(ctx, "Recursive Up"),
                                            make_dropdown_list_option(ctx, "Materials Directory"),
                                            make_dropdown_list_option(ctx, "Working Directory"),
                                        ])
                                        .with_selected(0)
                                        .with_close_on_selection(true)
                                        .build(ctx);
                                        options
                                    }),
                            )
                            .add_column(Column::strict(100.0))
                            .add_column(Column::stretch())
                            .add_row(Row::strict(26.0))
                            .build(ctx),
                        )
                        .with_child({
                            path_selection_section = GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(2)
                                    .with_visibility(false)
                                    .with_child({
                                        path_field = TextBoxBuilder::new(
                                            WidgetBuilder::new().with_enabled(false).on_column(0),
                                        )
                                        .with_vertical_text_alignment(VerticalAlignment::Center)
                                        .build(ctx);
                                        path_field
                                    })
                                    .with_child({
                                        select_path =
                                            ButtonBuilder::new(WidgetBuilder::new().on_column(1))
                                                .with_text("...")
                                                .build(ctx);
                                        select_path
                                    }),
                            )
                            .add_column(Column::stretch())
                            .add_column(Column::strict(26.0))
                            .add_row(Row::strict(26.0))
                            .build(ctx);
                            path_selection_section
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(4)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(100.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(100.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::strict(26.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        let path_selector = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(500.0))
                .open(false),
        )
        .with_filter(Filter::new(|p: &Path| p.is_dir()))
        .with_path(".")
        .build(ctx);

        Self {
            window,
            options,
            ok,
            cancel,
            select_path,
            path_selector,
            path_field,
            model_path: Default::default(),
            path_selection_section,
            material_search_options: MaterialSearchOptions::RecursiveUp,
        }
    }

    pub fn set_working_directory(&mut self, engine: &mut GameEngine, dir: &Path) {
        assert!(dir.is_dir());

        engine
            .user_interface
            .send_message(FileSelectorMessage::root(
                self.path_selector,
                MessageDirection::ToWidget,
                Some(dir.to_owned()),
            ));
    }

    pub fn open(&mut self, model_path: PathBuf, ui: &UserInterface) {
        self.model_path = model_path;

        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut Engine,
        sender: &Sender<Message>,
    ) {
        let ui = &engine.user_interface;

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));

                // No model was loaded yet, do it.
                if let Ok(model) =
                    rg3d::core::futures::executor::block_on(engine.resource_manager.request_model(
                        self.model_path.clone(),
                        self.material_search_options.clone(),
                    ))
                {
                    let scene = &mut engine.scenes[editor_scene.scene];

                    // Instantiate the model.
                    let instance = model.instantiate(scene);
                    // Enable instantiated animations.
                    for &animation in instance.animations.iter() {
                        scene.animations[animation].set_enabled(true);
                    }

                    // Immediately after extract if from the scene to subgraph. This is required to not violate
                    // the rule of one place of execution, only commands allowed to modify the scene.
                    let sub_graph = scene.graph.take_reserve_sub_graph(instance.root);
                    let animations_container = instance
                        .animations
                        .iter()
                        .map(|&anim| scene.animations.take_reserve(anim))
                        .collect();

                    let group = vec![
                        SceneCommand::new(AddModelCommand::new(sub_graph, animations_container)),
                        // We also want to select newly instantiated model.
                        SceneCommand::new(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(instance.root)),
                            editor_scene.selection.clone(),
                        )),
                    ];

                    sender
                        .send(Message::do_scene_command(CommandGroup::from(group)))
                        .unwrap();
                }
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.select_path {
                ui.send_message(WindowMessage::open_modal(
                    self.path_selector,
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(value))) =
            message.data::<DropdownListMessage>()
        {
            if message.destination() == self.options {
                let show_path_selection_options = match *value {
                    0 => {
                        self.material_search_options = MaterialSearchOptions::RecursiveUp;
                        false
                    }
                    1 => {
                        self.material_search_options =
                            MaterialSearchOptions::MaterialsDirectory(PathBuf::from("."));
                        true
                    }
                    2 => {
                        self.material_search_options = MaterialSearchOptions::WorkingDirectory;
                        false
                    }
                    _ => unreachable!(),
                };

                ui.send_message(WidgetMessage::visibility(
                    self.path_selection_section,
                    MessageDirection::ToWidget,
                    show_path_selection_options,
                ));
            }
        } else if let Some(FileSelectorMessage::Commit(path)) =
            message.data::<FileSelectorMessage>()
        {
            if message.destination() == self.path_selector {
                ui.send_message(TextBoxMessage::text(
                    self.path_field,
                    MessageDirection::ToWidget,
                    path.to_string_lossy().to_string(),
                ));

                self.material_search_options =
                    MaterialSearchOptions::MaterialsDirectory(path.clone());
            }
        }
    }
}

#[derive(Debug)]
pub enum Message {
    DoSceneCommand(SceneCommand),
    UndoSceneCommand,
    RedoSceneCommand,
    ClearSceneCommandStack,
    SelectionChanged,
    SyncToModel,
    SaveScene(PathBuf),
    LoadScene(PathBuf),
    CloseScene,
    SetInteractionMode(InteractionModeKind),
    Log(String),
    Configure {
        working_directory: PathBuf,
    },
    NewScene,
    Exit {
        force: bool,
    },
    OpenSettings(SettingsSectionKind),
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

struct Editor {
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
    light_panel: LightPanel,
    menu: Menu,
    exit: bool,
    configurator: Configurator,
    log: Log,
    command_stack_viewer: CommandStackViewer,
    validation_message_box: Handle<UiNode>,
    navmesh_panel: NavmeshPanel,
    settings: Settings,
    model_import_dialog: ModelImportDialog,
    path_fixer: PathFixer,
    material_editor: MaterialEditor,
    inspector: Inspector,
    curve_editor: CurveEditorWindow,
}

impl Editor {
    fn new(engine: &mut GameEngine) -> Self {
        let (message_sender, message_receiver) = mpsc::channel();

        *rg3d::gui::DEFAULT_FONT.0.lock().unwrap() = Font::from_memory(
            include_bytes!("../resources/embed/arial.ttf").to_vec(),
            14.0,
            Font::default_char_set(),
        )
        .unwrap();

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

        let scene_viewer = SceneViewer::new(engine, message_sender.clone());
        let asset_browser = AssetBrowser::new(engine);
        let menu = Menu::new(engine, message_sender.clone(), &settings);
        let light_panel = LightPanel::new(engine);

        let ctx = &mut engine.user_interface.build_ctx();
        let navmesh_panel = NavmeshPanel::new(ctx, message_sender.clone());
        let world_outliner = WorldViewer::new(ctx, message_sender.clone());
        let command_stack_viewer = CommandStackViewer::new(ctx, message_sender.clone());
        let log = Log::new(ctx);
        let model_import_dialog = ModelImportDialog::new(ctx);
        let inspector = Inspector::new(ctx, message_sender.clone());

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
                                                                .with_content(TileContent::Window(
                                                                    navmesh_panel.window,
                                                                ))
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

        let material_editor = MaterialEditor::new(engine);

        let mut editor = Self {
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
            model_import_dialog,
            path_fixer,
            material_editor,
            inspector,
            curve_editor,
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

        scene.render_target = Some(Texture::new_render_target(0, 0));
        self.scene_viewer
            .set_render_target(&engine.user_interface, scene.render_target.clone());

        let editor_scene = EditorScene::from_native_scene(scene, engine, path.clone());

        for mut interaction_mode in self.interaction_modes.drain(..) {
            interaction_mode.on_drop(engine);
        }

        self.interaction_modes = vec![
            Box::new(SelectInteractionMode::new(
                self.scene_viewer.frame(),
                self.scene_viewer.selection_frame(),
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
            Box::new(EditNavmeshMode::new(
                &editor_scene,
                engine,
                self.message_sender.clone(),
            )),
            Box::new(TerrainInteractionMode::new(
                &editor_scene,
                engine,
                self.message_sender.clone(),
            )),
        ];

        self.command_stack = CommandStack::new(false);
        self.scene = Some(editor_scene);

        self.set_interaction_mode(Some(InteractionModeKind::Move), engine);
        self.sync_to_model(engine);

        self.scene_viewer.set_title(
            &engine.user_interface,
            format!(
                "Scene Preview - {}",
                path.map_or("Unnamed Scene".to_string(), |p| p
                    .to_string_lossy()
                    .to_string())
            ),
        );

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

                // Activate new.
                if let Some(current_mode) = self.current_interaction_mode {
                    self.interaction_modes[current_mode as usize].activate(editor_scene, engine);
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        scope_profile!();

        // Prevent infinite message loops.
        if message.has_flags(MSG_SYNC_FLAG) {
            return;
        }

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
                },
                settings: &mut self.settings,
            },
        );

        self.log.handle_ui_message(message, engine);
        self.asset_browser.handle_ui_message(message, engine);
        self.command_stack_viewer.handle_ui_message(message);
        self.curve_editor.handle_ui_message(message, engine);
        self.path_fixer
            .handle_ui_message(message, &mut engine.user_interface);

        if let Some(editor_scene) = self.scene.as_mut() {
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

            self.scene_viewer
                .handle_ui_message(message, &engine.user_interface);

            self.material_editor
                .handle_ui_message(message, engine, &self.message_sender);

            self.model_import_dialog.handle_ui_message(
                message,
                editor_scene,
                engine,
                &self.message_sender,
            );

            let screen_bounds = self.scene_viewer.frame_bounds(&engine.user_interface);
            let frame_size = screen_bounds.size;

            if message.destination() == self.scene_viewer.frame() {
                if let Some(msg) = message.data::<WidgetMessage>() {
                    match *msg {
                        WidgetMessage::MouseDown { button, pos, .. } => {
                            engine
                                .user_interface
                                .capture_mouse(self.scene_viewer.frame());

                            if button == MouseButton::Left {
                                if let Some(current_im) = self.current_interaction_mode {
                                    let rel_pos = pos - screen_bounds.position;

                                    self.scene_viewer.click_mouse_pos = Some(rel_pos);

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
                                self.scene_viewer.click_mouse_pos = None;
                                if let Some(current_im) = self.current_interaction_mode {
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
                            let last_pos = *self.scene_viewer.last_mouse_pos.get_or_insert(pos);
                            let mouse_offset = pos - last_pos;
                            editor_scene.camera_controller.on_mouse_move(mouse_offset);
                            let rel_pos = pos - screen_bounds.position;

                            if let Some(current_im) = self.current_interaction_mode {
                                self.interaction_modes[current_im as usize].on_mouse_move(
                                    mouse_offset,
                                    rel_pos,
                                    editor_scene.camera_controller.camera,
                                    editor_scene,
                                    engine,
                                    frame_size,
                                    &self.settings,
                                );
                            }
                            self.scene_viewer.last_mouse_pos = Some(pos);
                        }
                        WidgetMessage::KeyUp(key) => {
                            editor_scene.camera_controller.on_key_up(key);

                            if let Some(current_im) = self.current_interaction_mode {
                                self.interaction_modes[current_im as usize].on_key_up(
                                    key,
                                    editor_scene,
                                    engine,
                                );
                            }
                        }
                        WidgetMessage::KeyDown(key) => {
                            editor_scene.camera_controller.on_key_down(key);

                            if let Some(current_im) = self.current_interaction_mode {
                                self.interaction_modes[current_im as usize].on_key_down(
                                    key,
                                    editor_scene,
                                    engine,
                                );
                            }

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
                                    if let Selection::Graph(graph_selection) =
                                        &editor_scene.selection
                                    {
                                        editor_scene.clipboard.fill_from_selection(
                                            graph_selection,
                                            editor_scene.scene,
                                            engine,
                                        );
                                    }
                                }
                                KeyCode::V
                                    if engine.user_interface.keyboard_modifiers().control =>
                                {
                                    if !editor_scene.clipboard.is_empty() {
                                        self.message_sender
                                            .send(Message::do_scene_command(PasteCommand::new()))
                                            .unwrap();
                                    }
                                }
                                KeyCode::Delete => {
                                    if !editor_scene.selection.is_empty() {
                                        match editor_scene.selection {
                                            Selection::Graph(_) => {
                                                self.message_sender
                                                    .send(Message::DoSceneCommand(
                                                        make_delete_selection_command(
                                                            editor_scene,
                                                            engine,
                                                        ),
                                                    ))
                                                    .unwrap();
                                            }
                                            Selection::Sound(ref selection) => {
                                                let mut commands = selection
                                                    .sources()
                                                    .iter()
                                                    .map(|&source| {
                                                        SceneCommand::new(
                                                            DeleteSoundSourceCommand::new(source),
                                                        )
                                                    })
                                                    .collect::<Vec<_>>();

                                                commands.insert(
                                                    0,
                                                    SceneCommand::new(ChangeSelectionCommand::new(
                                                        Selection::None,
                                                        editor_scene.selection.clone(),
                                                    )),
                                                );

                                                self.message_sender
                                                    .send(Message::do_scene_command(
                                                        CommandGroup::from(commands),
                                                    ))
                                                    .unwrap();
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                                _ => (),
                            }
                        }
                        WidgetMessage::Drop(handle) => {
                            if handle.is_some() {
                                if let Some(item) =
                                    engine.user_interface.node(handle).cast::<AssetItem>()
                                {
                                    // Make sure all resources loaded with relative paths only.
                                    // This will make scenes portable.
                                    let relative_path = make_relative_path(&item.path);

                                    match item.kind {
                                        AssetKind::Model => {
                                            self.model_import_dialog
                                                .open(relative_path, &engine.user_interface);
                                        }
                                        AssetKind::Texture => {
                                            let cursor_pos =
                                                engine.user_interface.cursor_position();
                                            let rel_pos = cursor_pos - screen_bounds.position;
                                            let graph = &engine.scenes[editor_scene.scene].graph;
                                            if let Some(result) =
                                                editor_scene.camera_controller.pick(
                                                    rel_pos,
                                                    graph,
                                                    editor_scene.root,
                                                    frame_size,
                                                    false,
                                                    |_, _| true,
                                                )
                                            {
                                                let tex = engine
                                                    .resource_manager
                                                    .request_texture(&relative_path, None);
                                                let texture = tex.clone();
                                                let texture = texture.state();
                                                if let TextureState::Ok(_) = *texture {
                                                    match &mut engine.scenes[editor_scene.scene]
                                                        .graph[result.node]
                                                    {
                                                        Node::Mesh(_) => {
                                                            self.message_sender
                                                                .send(Message::do_scene_command(
                                                                    SetMeshTextureCommand::new(
                                                                        result.node,
                                                                        tex,
                                                                    ),
                                                                ))
                                                                .unwrap();
                                                        }
                                                        Node::Sprite(_) => {
                                                            self.message_sender
                                                                .send(Message::do_scene_command(
                                                                    SetSpriteTextureCommand::new(
                                                                        result.node,
                                                                        Some(tex),
                                                                    ),
                                                                ))
                                                                .unwrap();
                                                        }
                                                        Node::ParticleSystem(_) => {
                                                            self.message_sender
                                                                .send(Message::do_scene_command(
                                                                    SetParticleSystemTextureCommand::new(
                                                                        result.node, Some(tex),
                                                                    ),
                                                                ),
                                                                )
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
                        _ => {}
                    }
                }
            }

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
    }

    fn sync_to_model(&mut self, engine: &mut GameEngine) {
        scope_profile!();

        self.menu
            .sync_to_model(self.scene.as_ref(), &mut engine.user_interface);

        if let Some(editor_scene) = self.scene.as_mut() {
            self.inspector.sync_to_model(editor_scene, engine);
            self.navmesh_panel.sync_to_model(editor_scene, engine);
            self.world_viewer.sync_to_model(editor_scene, engine);
            self.material_editor
                .sync_to_model(&mut engine.user_interface);
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
            self.world_viewer.clear(&mut engine.user_interface);
        }
    }

    fn post_update(&mut self, engine: &mut GameEngine) {
        if let Some(scene) = self.scene.as_mut() {
            self.world_viewer.post_update(scene, engine);
        }
    }

    fn update(&mut self, engine: &mut GameEngine, dt: f32) {
        scope_profile!();

        let mut needs_sync = false;

        while let Ok(message) = self.message_receiver.try_recv() {
            self.log.handle_message(&message, engine);
            self.path_fixer
                .handle_message(&message, &mut engine.user_interface);

            if let Some(editor_scene) = self.scene.as_ref() {
                self.inspector
                    .handle_message(&message, editor_scene, engine);
            }

            match message {
                Message::DoSceneCommand(command) => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        self.command_stack.do_command(
                            command.into_inner(),
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
                Message::ClearSceneCommandStack => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        self.command_stack.clear(SceneContext {
                            scene: &mut engine.scenes[editor_scene.scene],
                            message_sender: self.message_sender.clone(),
                            editor_scene,
                            resource_manager: engine.resource_manager.clone(),
                        });
                        needs_sync = true;
                    }
                }
                Message::SelectionChanged => {
                    self.world_viewer.sync_selection = true;
                }
                Message::SyncToModel => {
                    needs_sync = true;
                }
                Message::SaveScene(path) => {
                    if let Some(editor_scene) = self.scene.as_mut() {
                        match editor_scene.save(path.clone(), engine) {
                            Ok(message) => {
                                self.scene_viewer.set_title(
                                    &engine.user_interface,
                                    format!("Scene Preview - {}", path.display()),
                                );

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
                        rg3d::core::futures::executor::block_on(Scene::from_file(
                            &scene_path,
                            engine.resource_manager.clone(),
                            &MaterialSearchOptions::UsePathDirectly,
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
                        self.scene_viewer
                            .set_render_target(&engine.user_interface, None);
                    }
                }
                Message::NewScene => {
                    let mut scene = Scene::new();

                    scene.ambient_lighting_color = Color::opaque(200, 200, 200);

                    self.set_scene(engine, scene, None);
                }
                Message::Configure { working_directory } => {
                    assert!(self.scene.is_none());

                    self.asset_browser.clear_preview(engine);

                    std::env::set_current_dir(working_directory.clone()).unwrap();

                    engine.get_window().set_title(&format!(
                        "rusty-editor: {}",
                        working_directory.to_string_lossy()
                    ));

                    engine.resource_manager.state().destroy_unused_resources();

                    engine.renderer.flush();

                    self.asset_browser
                        .set_working_directory(engine, &working_directory);

                    self.model_import_dialog
                        .set_working_directory(engine, &working_directory);

                    self.message_sender
                        .send(Message::Log(format!(
                            "New working directory was successfully set: {:?}",
                            working_directory
                        )))
                        .unwrap();

                    needs_sync = true;
                }
                Message::OpenSettings(section) => {
                    self.menu.file_menu.settings.open(
                        &engine.user_interface,
                        &self.settings,
                        Some(section),
                    );
                }
                Message::OpenMaterialEditor(material) => {
                    self.material_editor.set_material(Some(material), engine);

                    engine.user_interface.send_message(WindowMessage::open(
                        self.material_editor.window,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
                Message::ShowInAssetBrowser(path) => {
                    self.asset_browser.locate_path(&engine.user_interface, path);
                }
                Message::SetWorldViewerFilter(filter) => {
                    self.world_viewer.set_filter(filter, &engine.user_interface);
                }
                Message::LocateObject { type_id, handle } => {
                    self.world_viewer.try_locate_object(type_id, handle, engine)
                }
                Message::SelectObject { type_id, handle } => {
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
                                    ChangeSelectionCommand::new(
                                        new_selection,
                                        scene.selection.clone(),
                                    ),
                                )))
                                .unwrap()
                        }
                    }
                }
                Message::SetEditorCameraProjection(projection) => {
                    if let Some(editor_scene) = self.scene.as_ref() {
                        editor_scene.camera_controller.set_projection(
                            &mut engine.scenes[editor_scene.scene].graph,
                            projection,
                        );
                    }
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

            let camera = scene.graph[editor_scene.camera_controller.camera].as_camera_mut();

            camera
                .projection_mut()
                .set_z_near(self.settings.graphics.z_near);
            camera
                .projection_mut()
                .set_z_far(self.settings.graphics.z_far);

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

            if let Selection::Graph(selection) = &editor_scene.selection {
                for &node in selection.nodes() {
                    let node = &scene.graph[node];
                    scene.drawing_context.draw_oob(
                        &node.local_bounding_box(),
                        node.global_transform(),
                        Color::GREEN,
                    );
                }
            }

            fn draw_recursively(
                node: Handle<Node>,
                graph: &Graph,
                ctx: &mut SceneDrawingContext,
                editor_scene: &EditorScene,
                show_tbn: bool,
                show_bounds: bool,
            ) {
                // Ignore editor nodes.
                if node == editor_scene.root {
                    return;
                }

                let node = &graph[node];
                match node {
                    Node::Base(_) => {
                        if show_bounds {
                            ctx.draw_oob(
                                &AxisAlignedBoundingBox::unit(),
                                node.global_transform(),
                                Color::opaque(255, 127, 39),
                            );
                        }
                    }
                    Node::Mesh(mesh) => {
                        if show_tbn {
                            // TODO: Add switch to settings to turn this on/off
                            let transform = node.global_transform();

                            for surface in mesh.surfaces() {
                                for vertex in surface.data().lock().vertex_buffer.iter() {
                                    let len = 0.025;
                                    let position = transform
                                        .transform_point(&Point3::from(
                                            vertex
                                                .read_3_f32(VertexAttributeUsage::Position)
                                                .unwrap(),
                                        ))
                                        .coords;
                                    let vertex_tangent =
                                        vertex.read_4_f32(VertexAttributeUsage::Tangent).unwrap();
                                    let tangent = transform
                                        .transform_vector(&vertex_tangent.xyz())
                                        .normalize()
                                        .scale(len);
                                    let normal = transform
                                        .transform_vector(
                                            &vertex
                                                .read_3_f32(VertexAttributeUsage::Normal)
                                                .unwrap()
                                                .xyz(),
                                        )
                                        .normalize()
                                        .scale(len);
                                    let binormal = tangent
                                        .xyz()
                                        .cross(&normal)
                                        .scale(vertex_tangent.w)
                                        .normalize()
                                        .scale(len);

                                    ctx.add_line(Line {
                                        begin: position,
                                        end: position + tangent,
                                        color: Color::RED,
                                    });

                                    ctx.add_line(Line {
                                        begin: position,
                                        end: position + normal,
                                        color: Color::BLUE,
                                    });

                                    ctx.add_line(Line {
                                        begin: position,
                                        end: position + binormal,
                                        color: Color::GREEN,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }

                for &child in node.children() {
                    draw_recursively(child, graph, ctx, editor_scene, show_tbn, show_bounds)
                }
            }

            // Draw pivots.
            draw_recursively(
                scene.graph.get_root(),
                &scene.graph,
                &mut scene.drawing_context,
                editor_scene,
                self.settings.debugging.show_tbn,
                self.settings.debugging.show_bounds,
            );

            let graph = &mut scene.graph;

            if self.settings.debugging.show_physics {
                graph.physics.draw(&mut scene.drawing_context);
            }
            editor_scene.camera_controller.update(graph, dt);

            if let Some(mode) = self.current_interaction_mode {
                self.interaction_modes[mode as usize].update(
                    editor_scene,
                    editor_scene.camera_controller.camera,
                    engine,
                );
            }

            self.asset_browser.update(engine);
            self.material_editor.update(engine);
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

        if dt >= 1.5 * fixed_timestep {
            break;
        }
    }

    let window = engine.get_window();
    window.set_cursor_icon(translate_cursor_icon(engine.user_interface.cursor()));
    window.request_redraw();
}

fn main() {
    let event_loop = EventLoop::new();

    let inner_size = if let Some(primary_monitor) = event_loop.primary_monitor() {
        let mut monitor_dimensions = primary_monitor.size();
        monitor_dimensions.height = (monitor_dimensions.height as f32 * 0.7) as u32;
        monitor_dimensions.width = (monitor_dimensions.width as f32 * 0.7) as u32;
        monitor_dimensions.to_logical::<f32>(primary_monitor.scale_factor())
    } else {
        LogicalSize::new(1024.0, 768.0)
    };

    let window_builder = rg3d::window::WindowBuilder::new()
        .with_inner_size(inner_size)
        .with_title("rusty editor")
        .with_resizable(true);

    let mut engine = GameEngine::new(window_builder, &event_loop, true).unwrap();

    engine.resource_manager.state().set_textures_import_options(
        TextureImportOptions::default().with_compression(CompressionOptions::NoCompression),
    );

    let overlay_pass = OverlayRenderPass::new(engine.renderer.pipeline_state());
    engine.renderer.add_render_pass(overlay_pass);

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
            engine.render().unwrap();
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
                    if let Err(e) = engine.set_frame_size(size.into()) {
                        rg3d::utils::log::Log::writeln(
                            MessageKind::Error,
                            format!("Failed to set renderer size! Reason: {:?}", e),
                        );
                    }
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
            if let Ok(profiling_results) = rg3d::core::profiler::print() {
                if let Ok(mut file) =
                    fs::File::create(project_dirs::working_data_dir("profiling.log"))
                {
                    let _ = writeln!(file, "{}", profiling_results);
                }
            }
        }
        _ => *control_flow = ControlFlow::Poll,
    });
}
