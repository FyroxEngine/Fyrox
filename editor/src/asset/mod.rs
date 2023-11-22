use crate::{
    asset::{dependency::DependencyViewer, inspector::AssetInspector, item::AssetItemBuilder},
    gui::{make_dropdown_list_option, AssetItemMessage},
    message::MessageSender,
    preview::PreviewPanel,
    utils::window_content,
    AssetItem, AssetKind, Message, Mode,
};
use fyrox::{
    asset::{manager::ResourceManager, state::ResourceState, untyped::UntypedResource},
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        futures::executor::block_on,
        log::Log,
        make_relative_path,
        parking_lot::lock_api::Mutex,
        pool::Handle,
        scope_profile,
        sstorage::ImmutableString,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        copypasta::ClipboardProvider,
        file_browser::{FileBrowserBuilder, FileBrowserMessage, Filter},
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextMessage,
        text_box::TextBoxBuilder,
        utils::make_simple_tooltip,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness, UiNode,
        UserInterface, VerticalAlignment, BRUSH_DARK,
    },
    material::{Material, MaterialResource, PropertyValue},
    resource::texture::Texture,
    scene::{
        base::BaseBuilder,
        mesh::{
            surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
            MeshBuilder,
        },
        sound::{SoundBuffer, SoundBuilder, Status},
    },
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

mod dependency;
mod inspector;
pub mod item;

struct ContextMenu {
    menu: RcUiNodeHandle,
    open: Handle<UiNode>,
    copy_path: Handle<UiNode>,
    copy_file_name: Handle<UiNode>,
    show_in_explorer: Handle<UiNode>,
    delete: Handle<UiNode>,
    placement_target: Handle<UiNode>,
    dependencies: Handle<UiNode>,
}

fn execute_command(command: &mut Command) {
    match command.spawn() {
        Ok(mut process) => Log::verify(process.wait()),
        Err(err) => Log::err(format!(
            "Failed to show asset item in explorer. Reason: {:?}",
            err
        )),
    }
}

fn show_in_explorer<P: AsRef<OsStr>>(path: P) {
    execute_command(Command::new("explorer").arg("/select,").arg(path))
}

fn open_in_explorer<P: AsRef<Path>>(path: P) {
    if let Ok(path) = path.as_ref().canonicalize() {
        Log::verify(open::that(path))
    } else {
        Log::err(format!("Failed to canonicalize path {:?}", path.as_ref()))
    }
}

fn put_path_to_clipboard(engine: &mut Engine, path: &OsStr) {
    if let Some(mut clipboard) = engine.user_interface.clipboard_mut() {
        Log::verify(clipboard.set_contents(path.to_string_lossy().to_string()));
    }
}

impl ContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let show_in_explorer;
        let open;
        let copy_path;
        let copy_file_name;
        let dependencies;
        let menu = PopupBuilder::new(WidgetBuilder::new())
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            open = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Open"))
                                .build(ctx);
                            open
                        })
                        .with_child({
                            copy_path = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Copy Full Path"))
                                .build(ctx);
                            copy_path
                        })
                        .with_child({
                            copy_file_name = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Copy File Name"))
                                .build(ctx);
                            copy_file_name
                        })
                        .with_child({
                            delete = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Delete"))
                                .build(ctx);
                            delete
                        })
                        .with_child({
                            show_in_explorer = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Show In Explorer"))
                                .build(ctx);
                            show_in_explorer
                        })
                        .with_child({
                            dependencies = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Dependencies"))
                                .build(ctx);
                            dependencies
                        }),
                )
                .build(ctx),
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            open,
            copy_path,
            delete,
            show_in_explorer,
            placement_target: Default::default(),
            copy_file_name,
            dependencies,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        sender: &MessageSender,
        engine: &mut Engine,
    ) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == *self.menu {
                self.placement_target = *target;
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(item) = engine
                .user_interface
                .try_get_node(self.placement_target)
                .and_then(|n| n.cast::<AssetItem>())
            {
                if message.destination() == self.delete {
                    Log::verify(std::fs::remove_file(&item.path))
                } else if message.destination() == self.show_in_explorer {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        show_in_explorer(canonical_path)
                    }
                } else if message.destination() == self.open {
                    if item.path.extension().map_or(false, |ext| ext == "rgs") {
                        sender.send(Message::LoadScene(item.path.clone()));
                    } else if item.path.extension().map_or(false, |ext| ext == "material") {
                        if let Ok(path) = make_relative_path(&item.path) {
                            if let Ok(material) =
                                block_on(engine.resource_manager.request::<Material>(path))
                            {
                                sender.send(Message::OpenMaterialEditor(material));
                            }
                        }
                    } else {
                        open_in_explorer(&item.path)
                    }
                } else if message.destination() == self.copy_path {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        put_path_to_clipboard(engine, canonical_path.as_os_str())
                    }
                } else if message.destination() == self.copy_file_name {
                    if let Some(file_name) = item.path.clone().file_name() {
                        put_path_to_clipboard(engine, file_name)
                    }
                }
            }
        }
    }
}

struct ResourceCreator {
    window: Handle<UiNode>,
    resource_constructors_list: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    name: Handle<UiNode>,
    selected: Option<usize>,
    name_str: String,
}

impl ResourceCreator {
    pub fn new(ctx: &mut BuildContext, resource_manager: &ResourceManager) -> Self {
        let items = resource_manager
            .state()
            .constructors_container
            .map
            .lock()
            .values()
            .map(|constructor| make_dropdown_list_option(ctx, &constructor.type_name))
            .collect();

        let name_str = String::from("unnamed_resource");
        let name;
        let ok;
        let cancel;
        let resource_constructors_list;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::text("Resource Creator"))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            name = TextBoxBuilder::new(
                                WidgetBuilder::new().on_row(0).with_height(22.0),
                            )
                            .with_text(&name_str)
                            .build(ctx);
                            name
                        })
                        .with_child({
                            resource_constructors_list =
                                ListViewBuilder::new(WidgetBuilder::new().on_row(1))
                                    .with_items(items)
                                    .build(ctx);
                            resource_constructors_list
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_enabled(false)
                                                .with_width(100.0)
                                                .with_height(22.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_height(22.0),
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
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            resource_constructors_list,
            ok,
            cancel,
            name,
            selected: None,
            name_str,
        }
    }

    fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    #[must_use]
    fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: MessageSender,
        base_path: &Path,
    ) -> bool {
        let mut asset_added = false;

        if let Some(ListViewMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.resource_constructors_list
                && message.direction() == MessageDirection::FromWidget
            {
                self.selected = Some(*index);
                engine.user_interface.send_message(WidgetMessage::enabled(
                    self.ok,
                    MessageDirection::ToWidget,
                    true,
                ));

                // Propose extension for the resource.
                let resource_manager_state = engine.resource_manager.state();
                let constructors = resource_manager_state.constructors_container.map.lock();
                if let Some(data_type_uuid) =
                    constructors.keys().nth(self.selected.unwrap_or_default())
                {
                    if let Some(loader) = resource_manager_state
                        .loaders
                        .iter()
                        .find(|loader| &loader.data_type_uuid() == data_type_uuid)
                    {
                        if let Some(first) = loader.extensions().first() {
                            let mut path = PathBuf::from(&self.name_str);
                            path.set_extension(first);

                            engine.user_interface.send_message(TextMessage::text(
                                self.name,
                                MessageDirection::ToWidget,
                                path.to_string_lossy().to_string(),
                            ));
                        }
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                let resource_manager_state = engine.resource_manager.state();
                let mut constructors = resource_manager_state.constructors_container.map.lock();
                if let Some(mut instance) = constructors
                    .values_mut()
                    .nth(self.selected.unwrap_or_default())
                    .map(|c| c.create_instance())
                {
                    let path = base_path.join(&self.name_str);
                    instance.set_path(path.clone());
                    match instance.save(&path) {
                        Ok(_) => {
                            let resource =
                                UntypedResource(Arc::new(Mutex::new(ResourceState::Ok(instance))));

                            drop(constructors);
                            drop(resource_manager_state);

                            Log::verify(engine.resource_manager.register(
                                resource,
                                path,
                                |_, _| true,
                            ));

                            sender.send(Message::ForceSync);

                            asset_added = true;
                        }
                        Err(e) => Log::err(format!("Unable to create a resource. Reason: {:?}", e)),
                    }
                }
            }

            if message.destination() == self.ok || message.destination() == self.cancel {
                engine.user_interface.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.name
                && message.direction() == MessageDirection::FromWidget
            {
                self.name_str = text.clone();
            }
        }

        asset_added
    }
}

pub struct AssetBrowser {
    pub window: Handle<UiNode>,
    content_panel: Handle<UiNode>,
    folder_browser: Handle<UiNode>,
    scroll_panel: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    add_resource: Handle<UiNode>,
    preview: PreviewPanel,
    items: Vec<Handle<UiNode>>,
    item_to_select: Option<PathBuf>,
    inspector: AssetInspector,
    context_menu: ContextMenu,
    selected_path: PathBuf,
    dependency_viewer: DependencyViewer,
    resource_creator: Option<ResourceCreator>,
}

fn is_supported_resource(ext: &OsStr, resource_manager: &ResourceManager) -> bool {
    resource_manager.state().loaders.iter().any(|loader| {
        loader
            .extensions()
            .iter()
            .any(|loader_ext| OsStr::new(loader_ext) == ext)
    })
}

impl AssetBrowser {
    pub fn new(engine: &mut Engine) -> Self {
        let preview = PreviewPanel::new(engine, 250, 250);
        let ctx = &mut engine.user_interface.build_ctx();

        let inspector = AssetInspector::new(ctx, 1, 0);

        let content_panel;
        let folder_browser;
        let search_bar;
        let scroll_panel;
        let add_resource;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("AssetBrowser"))
            .can_minimize(false)
            .with_title(WindowTitle::text("Asset Browser"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .with_background(BRUSH_DARK)
                                    .with_child({
                                        folder_browser = FileBrowserBuilder::new(
                                            WidgetBuilder::new().on_column(0),
                                        )
                                        .with_show_path(false)
                                        .with_filter(Filter::new(|p: &Path| p.is_dir()))
                                        .build(ctx);
                                        folder_browser
                                    }),
                            )
                            .build(ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(1)
                                    .with_child(
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child({
                                                    add_resource = ButtonBuilder::new(
                                                        WidgetBuilder::new()
                                                            .with_height(20.0)
                                                            .with_width(20.0)
                                                            .with_margin(Thickness::uniform(1.0))
                                                            .with_tooltip(make_simple_tooltip(
                                                                ctx,
                                                                "Add New Resource",
                                                            )),
                                                    )
                                                    .with_text("+")
                                                    .build(ctx);
                                                    add_resource
                                                })
                                                .with_child({
                                                    search_bar = SearchBarBuilder::new(
                                                        WidgetBuilder::new()
                                                            .on_column(1)
                                                            .with_height(22.0)
                                                            .with_margin(Thickness::uniform(1.0)),
                                                    )
                                                    .build(ctx);
                                                    search_bar
                                                }),
                                        )
                                        .add_column(Column::auto())
                                        .add_column(Column::stretch())
                                        .add_row(Row::auto())
                                        .build(ctx),
                                    )
                                    .with_child({
                                        scroll_panel = ScrollViewerBuilder::new(
                                            WidgetBuilder::new().on_row(1),
                                        )
                                        .with_content({
                                            content_panel = WrapPanelBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Left,
                                                    )
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Top,
                                                    ),
                                            )
                                            .with_orientation(Orientation::Horizontal)
                                            .build(ctx);
                                            content_panel
                                        })
                                        .build(ctx);
                                        scroll_panel
                                    }),
                            )
                            .add_row(Row::auto())
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        )
                        .with_child(
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(2)
                                    .with_foreground(Brush::Solid(Color::opaque(80, 80, 80)))
                                    .with_child(
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .with_child(preview.root)
                                                .with_child(inspector.container),
                                        )
                                        .add_column(Column::stretch())
                                        .add_row(Row::stretch())
                                        .add_row(Row::stretch())
                                        .build(ctx),
                                    ),
                            )
                            .build(ctx),
                        ),
                )
                .add_column(Column::strict(250.0))
                .add_column(Column::stretch())
                .add_column(Column::strict(250.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        let context_menu = ContextMenu::new(ctx);

        let dependency_viewer = DependencyViewer::new(ctx);

        Self {
            dependency_viewer,
            window,
            content_panel,
            folder_browser,
            preview,
            scroll_panel,
            search_bar,
            items: Default::default(),
            item_to_select: None,
            inspector,
            context_menu,
            selected_path: Default::default(),
            add_resource,
            resource_creator: None,
        }
    }

    pub fn clear_preview(&mut self, engine: &mut Engine) {
        self.preview.clear(engine);
    }

    pub fn set_working_directory(&mut self, engine: &mut Engine, dir: &Path) {
        assert!(dir.is_dir());

        engine.user_interface.send_message(FileBrowserMessage::root(
            self.folder_browser,
            MessageDirection::ToWidget,
            Some(dir.to_owned()),
        ));
    }

    fn add_asset(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) -> Handle<UiNode> {
        let asset_item = AssetItemBuilder::new(
            WidgetBuilder::new().with_context_menu(self.context_menu.menu.clone()),
        )
        .with_path(path)
        .build(&mut ui.build_ctx(), resource_manager.clone());

        self.items.push(asset_item);

        ui.send_message(WidgetMessage::link(
            asset_item,
            MessageDirection::ToWidget,
            self.content_panel,
        ));

        asset_item
    }

    fn clear_assets(&mut self, ui: &UserInterface) {
        for child in self.items.drain(..) {
            ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
    }

    fn set_path(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) {
        self.selected_path = path.to_path_buf();
        self.refresh(ui, resource_manager);
    }

    fn refresh(&mut self, ui: &mut UserInterface, resource_manager: &ResourceManager) {
        let item_to_select = self.item_to_select.take();
        let mut handle_to_select = Handle::NONE;

        // Clean content panel first.
        self.clear_assets(ui);

        // Get all supported assets from folder and generate previews for them.
        if let Ok(dir_iter) = std::fs::read_dir(&self.selected_path) {
            for entry in dir_iter.flatten() {
                if let Ok(entry_path) = make_relative_path(entry.path()) {
                    if !entry_path.is_dir()
                        && entry_path
                            .extension()
                            .map_or(false, |ext| is_supported_resource(ext, resource_manager))
                    {
                        let asset_item = self.add_asset(&entry_path, ui, resource_manager);

                        if let Some(item_to_select) = item_to_select.as_ref() {
                            if item_to_select == &entry_path {
                                handle_to_select = asset_item;
                            }
                        }
                    }
                }
            }
        }

        if let Ok(path) = self.selected_path.canonicalize() {
            if let Ok(working_dir) = std::env::current_dir().and_then(|dir| dir.canonicalize()) {
                if path == working_dir {
                    let state = resource_manager.state();
                    for (path, _) in state.built_in_resources.iter() {
                        self.add_asset(path, ui, resource_manager);
                    }
                }
            }
        }

        if handle_to_select.is_some() {
            ui.send_message(AssetItemMessage::select(
                handle_to_select,
                MessageDirection::ToWidget,
                true,
            ));

            ui.send_message(ScrollViewerMessage::bring_into_view(
                self.scroll_panel,
                MessageDirection::ToWidget,
                handle_to_select,
            ));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        sender: MessageSender,
    ) {
        scope_profile!();

        self.inspector.handle_ui_message(message, engine);
        self.preview.handle_message(message, engine);
        self.context_menu
            .handle_ui_message(message, &sender, engine);
        self.dependency_viewer
            .handle_ui_message(message, &mut engine.user_interface);
        if let Some(resource_creator) = self.resource_creator.as_mut() {
            let asset_added = resource_creator.handle_ui_message(
                message,
                engine,
                sender.clone(),
                &self.selected_path,
            );
            if asset_added {
                self.refresh(&mut engine.user_interface, &engine.resource_manager);
            }
        }

        let ui = &mut engine.user_interface;

        if let Some(AssetItemMessage::Select(true)) = message.data::<AssetItemMessage>() {
            // Deselect other items.
            for &item in self.items.iter().filter(|i| **i != message.destination()) {
                ui.send_message(AssetItemMessage::select(
                    item,
                    MessageDirection::ToWidget,
                    false,
                ))
            }

            self.inspector.inspect_resource_import_options(
                &ui.node(message.destination())
                    .cast::<AssetItem>()
                    .expect("Must be AssetItem")
                    .path
                    .clone(),
                ui,
                sender,
                &engine.resource_manager,
            );

            let item = ui
                .node(message.destination())
                .cast::<AssetItem>()
                .expect("Must be AssetItem");

            match item.kind {
                AssetKind::Unknown => {}
                AssetKind::Model => {
                    let path = item.path.clone();
                    block_on(self.preview.load_model(&path, engine));
                }
                AssetKind::Texture => {
                    let path = item.path.clone();
                    let mut material = Material::standard_two_sides();
                    Log::verify(material.set_property(
                        &ImmutableString::new("diffuseTexture"),
                        PropertyValue::Sampler {
                            value: Some(engine.resource_manager.request::<Texture>(&path)),
                            fallback: Default::default(),
                        },
                    ));
                    let material = MaterialResource::new_ok(material);

                    let graph = &mut engine.scenes[self.preview.scene()].graph;
                    let quad = MeshBuilder::new(BaseBuilder::new())
                        .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
                            SurfaceData::make_quad(
                                &UnitQuaternion::from_axis_angle(
                                    &Vector3::z_axis(),
                                    180.0f32.to_radians(),
                                )
                                .to_homogeneous(),
                            ),
                        ))
                        .with_material(material)
                        .build()])
                        .build(graph);
                    self.preview.set_model(quad, engine);
                }
                AssetKind::Sound => {
                    let path = item.path.clone();
                    if let Ok(buffer) =
                        block_on(engine.resource_manager.request::<SoundBuffer>(&path))
                    {
                        let graph = &mut engine.scenes[self.preview.scene()].graph;
                        let sound = SoundBuilder::new(BaseBuilder::new())
                            .with_buffer(Some(buffer))
                            .with_status(Status::Playing)
                            .build(graph);
                        self.preview.set_model(sound, engine);
                    }
                }
                AssetKind::Shader => {
                    Log::warn("Implement me!");
                }
            }
        } else if let Some(FileBrowserMessage::Path(path)) = message.data::<FileBrowserMessage>() {
            if message.destination() == self.folder_browser
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(SearchBarMessage::text(
                    self.search_bar,
                    MessageDirection::ToWidget,
                    Default::default(),
                ));
                self.set_path(path, ui, &engine.resource_manager);
            }
        } else if let Some(SearchBarMessage::Text(search_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                if search_text.is_empty() {
                    let path = self.selected_path.clone();
                    self.set_path(&path, ui, &engine.resource_manager);
                } else {
                    self.clear_assets(ui);
                    let search_text = search_text.to_lowercase();
                    for dir in fyrox::walkdir::WalkDir::new(".").into_iter().flatten() {
                        if let Some(extension) = dir.path().extension() {
                            if is_supported_resource(extension, &engine.resource_manager) {
                                let file_stem = dir
                                    .path()
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_lowercase())
                                    .unwrap_or_default();
                                if file_stem.contains(&search_text) {
                                    if let Ok(relative_path) = make_relative_path(dir.path()) {
                                        self.add_asset(
                                            &relative_path,
                                            ui,
                                            &engine.resource_manager,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.context_menu.dependencies {
                if let Some(item) = engine
                    .user_interface
                    .try_get_node(self.context_menu.placement_target)
                    .and_then(|n| n.cast::<AssetItem>())
                {
                    if let Ok(resource) =
                        block_on(engine.resource_manager.request_untyped(&item.path))
                    {
                        self.dependency_viewer
                            .open(&resource, &mut engine.user_interface);
                    }
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if let Some(resource_creator) = self.resource_creator.as_ref() {
                if message.destination() == resource_creator.window {
                    engine.user_interface.send_message(WidgetMessage::remove(
                        resource_creator.window,
                        MessageDirection::ToWidget,
                    ));

                    self.resource_creator = None;
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_resource {
                let resource_creator = ResourceCreator::new(
                    &mut engine.user_interface.build_ctx(),
                    &engine.resource_manager,
                );

                resource_creator.open(&engine.user_interface);

                self.resource_creator = Some(resource_creator);
            }
        }
    }

    pub fn locate_path(&mut self, ui: &UserInterface, path: PathBuf) {
        ui.send_message(FileBrowserMessage::path(
            self.folder_browser,
            MessageDirection::ToWidget,
            path.parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        ));

        self.item_to_select = Some(path);
    }

    pub fn update(&mut self, engine: &mut Engine) {
        self.preview.update(engine)
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
