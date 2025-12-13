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

use crate::{
    asset::{
        creator::ResourceCreator,
        dependency::DependencyViewer,
        item::{AssetItem, AssetItemBuilder, AssetItemMessage},
        preview::{
            cache::{AssetPreviewCache, IconRequest},
            AssetPreviewGeneratorsCollection,
        },
        selection::AssetSelection,
    },
    fyrox::{
        asset::{manager::ResourceManager, options::BaseImportOptions},
        core::{
            append_extension, err, futures::executor::block_on, log::Log, make_relative_path,
            ok_or_continue, pool::Handle, some_or_continue, SafeLock,
        },
        engine::Engine,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            border::BorderBuilder,
            button::{Button, ButtonBuilder, ButtonMessage},
            copypasta::ClipboardProvider,
            decorator::DecoratorBuilder,
            dock::{DockingManagerBuilder, TileBuilder, TileContent},
            file_browser::{FileBrowserBuilder, FileBrowserMessage, PathFilter},
            grid::{Column, GridBuilder, Row},
            menu::MenuItemMessage,
            message::{MouseButton, UiMessage},
            scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            style::{resource::StyleResourceExt, Style},
            utils::{make_image_button_with_tooltip, make_simple_tooltip},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
        },
    },
    load_image,
    message::MessageSender,
    plugin::EditorPluginsContainer,
    plugins::inspector::InspectorPlugin,
    preview::PreviewPanel,
    scene::{commands::ChangeSelectionCommand, container::EditorSceneEntry, Selection},
    utils::window_content,
    Message, Mode,
};
use fyrox::asset::event::ResourceEvent;
use menu::AssetItemContextMenu;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::Receiver;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc,
    },
};

mod creator;
mod dependency;
pub mod item;
pub mod menu;
pub mod preview;
mod selection;
pub mod selector;

fn show_in_explorer<P: AsRef<Path>>(path: P) {
    // opener crate is bugged on Windows, so using explorer's command directly.
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        fn execute_command(command: &mut Command) {
            match command.spawn() {
                Ok(mut process) => Log::verify(process.wait()),
                Err(err) => Log::err(format!(
                    "Failed to show asset item in explorer. Reason: {err:?}"
                )),
            }
        }

        execute_command(Command::new("explorer").arg("/select,").arg(path.as_ref()))
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Err(err) = opener::reveal(path) {
            Log::err(format!(
                "Failed to show asset item in explorer. Reason: {err:?}"
            ))
        }
    }
}

fn open_in_explorer<P: AsRef<Path>>(path: P) {
    if let Ok(path) = path.as_ref().canonicalize() {
        Log::verify(open::that(path))
    }
}

fn put_path_to_clipboard(engine: &mut Engine, path: &OsStr) {
    if let Some(mut clipboard) = engine.user_interfaces.first_mut().clipboard_mut() {
        Log::verify(clipboard.set_contents(path.to_string_lossy().to_string()));
    }
}

fn make_unique_path(parent: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut suffix = "_Copy".to_string();
    loop {
        let trial_copy_path = parent.join(format!("{stem}{suffix}.{ext}"));
        if trial_copy_path.exists() {
            suffix += "_Copy";
        } else {
            return trial_copy_path;
        }
    }
}

struct InspectorAddon {
    root: Handle<UiNode>,
    apply: Handle<UiNode>,
    revert: Handle<UiNode>,
    preview: PreviewPanel,
}

pub struct AssetBrowser {
    pub window: Handle<UiNode>,
    pub docking_manager: Handle<UiNode>,
    content_panel: Handle<UiNode>,
    folder_browser: Handle<UiNode>,
    scroll_panel: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    add_resource: Handle<UiNode>,
    refresh: Handle<UiNode>,
    items: Vec<Handle<UiNode>>,
    item_to_select: Option<PathBuf>,
    context_menu: AssetItemContextMenu,
    current_path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    dependency_viewer: DependencyViewer,
    resource_creator: Option<ResourceCreator>,
    preview_cache: AssetPreviewCache,
    pub preview_sender: Sender<IconRequest>,
    need_refresh: Arc<AtomicBool>,
    main_window: Handle<UiNode>,
    pub preview_generators: AssetPreviewGeneratorsCollection,
    inspector_addon: Option<InspectorAddon>,
    resource_event_receiver: Receiver<ResourceEvent>,
    resave_resources: Handle<UiNode>,
}

fn is_path_in_registry(path: &Path, resource_manager: &ResourceManager) -> bool {
    let rm_state = resource_manager.state();
    let registry = rm_state.resource_registry.safe_lock();
    if let Some(registry_directory) = registry.directory() {
        if let Ok(canonical_registry_path) = registry_directory.canonicalize() {
            if let Ok(canonical_path) = path.canonicalize() {
                if canonical_path.is_dir() {
                    return canonical_path.starts_with(canonical_registry_path);
                } else if let Some(parent) = canonical_registry_path.parent() {
                    if let Ok(relative) = canonical_path.strip_prefix(parent) {
                        return registry.is_registered(relative);
                    }
                }
            }
        }
    }
    false
}

fn is_supported_resource(ext: &OsStr, resource_manager: &ResourceManager) -> bool {
    let Some(ext) = ext.to_str() else {
        return false;
    };

    resource_manager
        .state()
        .loaders
        .safe_lock()
        .iter()
        .any(|loader| loader.supports_extension(ext))
}

fn create_file_system_watcher(
    resource_manager: ResourceManager,
    need_refresh_flag: Arc<AtomicBool>,
) -> Option<RecommendedWatcher> {
    notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else {
            return;
        };

        if !matches!(
            event.kind,
            EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
        ) {
            return;
        };

        for path in event.paths.iter() {
            if path.is_dir()
                || path
                    .extension()
                    .is_some_and(|ext| is_supported_resource(ext, &resource_manager))
            {
                need_refresh_flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    })
    .ok()
}

fn try_move_resource(
    src_path: &Path,
    dest_path: &Path,
    resource_manager: &ResourceManager,
) -> bool {
    if let Err(err) = block_on(resource_manager.move_resource_by_path(src_path, dest_path, true)) {
        err!(
            "An error occurred at the attempt to move a resource.\nReason: {}",
            err
        );

        false
    } else {
        true
    }
}

impl AssetBrowser {
    pub const ADD_ASSET_NORMAL_BRUSH: &'static str = "AssetBrowser.AddAssetNormalBrush";
    pub const ADD_ASSET_HOVER_BRUSH: &'static str = "AssetBrowser.AddAssetHoverBrush";
    pub const ADD_ASSET_PRESSED_BRUSH: &'static str = "AssetBrowser.AddAssetPressedBrush";

    pub fn new(engine: &mut Engine) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        engine
            .resource_manager
            .state()
            .event_broadcaster
            .add(sender);

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let add_resource = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(1))
                .with_height(24.0)
                .with_width(24.0)
                .with_margin(Thickness::uniform(1.0))
                .with_tooltip(make_simple_tooltip(ctx, "Add New Resource")),
        )
        .with_back(
            DecoratorBuilder::new(
                BorderBuilder::new(
                    WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
                )
                .with_pad_by_corner_radius(false)
                .with_corner_radius(ctx.style.property(Button::CORNER_RADIUS))
                .with_stroke_thickness(ctx.style.property(Button::BORDER_THICKNESS)),
            )
            .with_normal_brush(ctx.style.property(Self::ADD_ASSET_NORMAL_BRUSH))
            .with_hover_brush(ctx.style.property(Self::ADD_ASSET_HOVER_BRUSH))
            .with_pressed_brush(ctx.style.property(Self::ADD_ASSET_PRESSED_BRUSH))
            .build(ctx),
        )
        .with_text("+")
        .build(ctx);

        let resave_resources = make_image_button_with_tooltip(
            ctx,
            18.0,
            18.0,
            load_image!("../../resources/resave.png"),
            "Resave All Native Resources\n\
            Use for assets migration from the previous versions of the engine",
            None,
        );
        ctx[resave_resources].set_column(2);

        let refresh = make_image_button_with_tooltip(
            ctx,
            18.0,
            18.0,
            load_image!("../../resources/reimport.png"),
            "Refresh",
            Some(1),
        );
        ctx[refresh].set_column(1);

        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(2))
                .on_column(3)
                .with_height(22.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);

        let toolbar = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(add_resource)
                .with_child(refresh)
                .with_child(resave_resources)
                .with_child(search_bar),
        )
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);

        let content_panel;
        let folder_browser;
        let scroll_panel;
        let folder_browser_window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Folders"))
            .with_tab_label("Folders")
            .can_close(false)
            .can_minimize(false)
            .can_maximize(false)
            .with_content({
                folder_browser = FileBrowserBuilder::new(
                    WidgetBuilder::new().on_column(0).with_tab_index(Some(0)),
                )
                .with_no_items_text("There are no subfolders. Right-click to add one.")
                .with_show_path(false)
                .with_filter(PathFilter::folder())
                .build(ctx);
                folder_browser
            })
            .build(ctx);

        let main_window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Folder Content"))
            .with_tab_label("Content")
            .can_close(false)
            .can_minimize(false)
            .can_maximize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .on_column(1)
                        .with_child(toolbar)
                        .with_child({
                            scroll_panel = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    content_panel = WrapPanelBuilder::new(
                                        WidgetBuilder::new()
                                            .with_horizontal_alignment(HorizontalAlignment::Left)
                                            .with_vertical_alignment(VerticalAlignment::Top),
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
            .build(ctx);

        let docking_manager = DockingManagerBuilder::new(
            WidgetBuilder::new().with_child(
                TileBuilder::new(WidgetBuilder::new())
                    .with_content(TileContent::HorizontalTiles {
                        splitter: 0.25,
                        tiles: [
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(folder_browser_window))
                                .build(ctx),
                            TileBuilder::new(WidgetBuilder::new())
                                .with_content(TileContent::Window(main_window))
                                .build(ctx),
                        ],
                    })
                    .build(ctx),
            ),
        )
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_name("AssetBrowser"))
            .can_minimize(false)
            .with_title(WindowTitle::text("Asset Browser"))
            .with_tab_label("Asset Browser")
            .with_content(docking_manager)
            .build(ctx);

        let context_menu = AssetItemContextMenu::new(ctx);

        let dependency_viewer = DependencyViewer::new(ctx);

        let (preview_sender, preview_receiver) = mpsc::channel();

        let need_refresh = Arc::new(AtomicBool::new(false));
        let watcher =
            create_file_system_watcher(engine.resource_manager.clone(), need_refresh.clone());

        Self {
            dependency_viewer,
            window,
            docking_manager,
            content_panel,
            folder_browser,
            scroll_panel,
            search_bar,
            items: Default::default(),
            item_to_select: None,
            context_menu,
            current_path: Default::default(),
            add_resource,
            resource_creator: None,
            preview_cache: AssetPreviewCache::new(preview_receiver, 4),
            preview_sender,
            need_refresh,
            main_window,
            preview_generators: AssetPreviewGeneratorsCollection::new(),
            refresh,
            watcher,
            inspector_addon: None,
            resource_event_receiver: receiver,
            resave_resources,
        }
    }

    pub fn set_working_directory(&mut self, engine: &mut Engine) {
        let ui = engine.user_interfaces.first_mut();

        let registry_folder = engine.resource_manager.registry_folder();

        ui.send_many(
            self.folder_browser,
            [
                FileBrowserMessage::Root(Some(registry_folder.clone())),
                FileBrowserMessage::Path(registry_folder.clone()),
            ],
        );
        self.set_path(&registry_folder, ui, &engine.resource_manager);
    }

    fn add_asset(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        message_sender: &MessageSender,
    ) -> Handle<UiNode> {
        let is_dir = path.is_dir();

        let asset_item = AssetItemBuilder::new(
            WidgetBuilder::new().with_context_menu(self.context_menu.menu.clone()),
        )
        .with_icon(if is_dir {
            load_image!("../../resources/folder.png")
        } else {
            load_image!("../../resources/hourglass.png")
        })
        .with_path(path)
        .build(
            resource_manager.clone(),
            message_sender.clone(),
            &mut ui.build_ctx(),
        );

        if !is_dir {
            Log::verify(self.preview_sender.send(IconRequest {
                resource: resource_manager.request_untyped(path),
                widget_handle: asset_item,
                force_update: false,
            }));
        }

        self.items.push(asset_item);

        ui.send(asset_item, WidgetMessage::LinkWith(self.content_panel));

        asset_item
    }

    fn clear_assets(&mut self, ui: &UserInterface) {
        for child in self.items.drain(..) {
            ui.send(child, WidgetMessage::Remove);
        }
    }

    pub fn request_current_path(&self, path: PathBuf, ui: &UserInterface) {
        if !path.is_dir() {
            return;
        }
        ui.send(self.folder_browser, FileBrowserMessage::Path(path));
    }

    fn is_current_path_in_registry(&self, resource_manager: &ResourceManager) -> bool {
        is_path_in_registry(&self.current_path, resource_manager)
    }

    fn set_path(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) {
        if let Some(watcher) = self.watcher.as_mut() {
            // notify 6.1.1 crashes otherwise
            if self.current_path.exists() {
                Log::verify(watcher.unwatch(&self.current_path));
            }
            if path.exists() {
                Log::verify(watcher.watch(path, RecursiveMode::NonRecursive));
            } else {
                Log::err(format!("cannot watch non-existing path {path:?}"));
            }
        }

        self.current_path = path.to_path_buf();
        ui.send(
            self.main_window,
            WindowMessage::Title(WindowTitle::text(format!(
                "Folder Content - {}",
                self.current_path.display()
            ))),
        );
        ui.send(
            self.add_resource,
            WidgetMessage::Enabled(self.is_current_path_in_registry(resource_manager)),
        );
        self.schedule_refresh();
    }

    fn refresh(
        &mut self,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        message_sender: &MessageSender,
    ) {
        let item_to_select = self.item_to_select.take();
        let mut handle_to_select = Handle::NONE;

        // Clean content panel first.
        self.clear_assets(ui);

        // Add "return" item.
        if let Some(absolute_parent_path) = self
            .current_path
            .canonicalize()
            .ok()
            .and_then(|current_path| current_path.parent().map(|path| path.to_owned()))
        {
            let registry_folder = resource_manager.registry_folder();
            if absolute_parent_path.starts_with(&registry_folder) {
                if let Ok(relative_parent_path) = make_relative_path(absolute_parent_path) {
                    let asset_item = AssetItemBuilder::new(
                        WidgetBuilder::new().with_context_menu(self.context_menu.menu.clone()),
                    )
                    .with_icon(load_image!("../../resources/folder_return.png"))
                    .with_path(relative_parent_path)
                    .build(
                        resource_manager.clone(),
                        message_sender.clone(),
                        &mut ui.build_ctx(),
                    );

                    self.items.push(asset_item);
                    ui.send(asset_item, WidgetMessage::LinkWith(self.content_panel));
                }
            }
        }

        let mut folders = Vec::new();
        let mut resources = Vec::new();

        // Get all supported assets from a folder and generate previews for them.
        if let Ok(dir_iter) = std::fs::read_dir(&self.current_path) {
            for path in dir_iter
                .flatten()
                .map(|e| e.path())
                .filter(|p| is_path_in_registry(p, resource_manager))
            {
                if let Ok(entry_path) = make_relative_path(path) {
                    if entry_path.is_dir() {
                        folders.push(entry_path);
                    } else {
                        resources.push(entry_path);
                    }
                }
            }
        }

        // Collect built-in resource (only if the registry folder is selected).
        if let Ok(path) = self.current_path.canonicalize() {
            let working_dir = resource_manager.registry_folder();
            if path == working_dir {
                let built_in_resources = resource_manager
                    .state()
                    .built_in_resources
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();

                resources.extend_from_slice(&built_in_resources);
            }
        }

        folders.sort();
        resources.sort();

        // Generate items.
        for path in folders.into_iter().chain(resources.into_iter()) {
            let asset_item = self.add_asset(&path, ui, resource_manager, message_sender);

            if let Some(item_to_select) = item_to_select.as_ref() {
                if item_to_select == &path {
                    handle_to_select = asset_item;
                }
            }
        }

        if handle_to_select.is_some() {
            ui.send(handle_to_select, AssetItemMessage::Select(true));
            ui.send(
                self.scroll_panel,
                ScrollViewerMessage::BringIntoView(handle_to_select),
            );
        }
    }

    fn on_asset_selected(
        &mut self,
        selected_asset: Handle<UiNode>,
        sender: MessageSender,
        entry: &mut EditorSceneEntry,
        engine: &mut Engine,
    ) {
        let ui = &mut engine.user_interfaces.first_mut();

        let asset_path = ui
            .node(selected_asset)
            .cast::<AssetItem>()
            .expect("Must be AssetItem")
            .path
            .clone();

        if asset_path.as_os_str().is_empty() {
            err!("Selecting AssetItem with empty path: {selected_asset}");
            return;
        }

        if let Some(selection) = entry.selection.as_ref::<AssetSelection>() {
            if selection.contains(&asset_path) {
                return;
            }
        }
        sender.do_command(ChangeSelectionCommand::new(Selection::new(
            AssetSelection::new(asset_path.clone(), &engine.resource_manager),
        )));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut Engine,
        entry: &mut EditorSceneEntry,
        sender: MessageSender,
    ) {
        if let Some(inspector_addon) = self.inspector_addon.as_mut() {
            inspector_addon.preview.handle_message(message, engine);
        }
        if self
            .context_menu
            .handle_ui_message(message, &sender, engine)
        {
            self.schedule_refresh();
        }
        self.dependency_viewer
            .handle_ui_message(message, engine.user_interfaces.first_mut());
        if let Some(resource_creator) = self.resource_creator.as_mut() {
            let asset_added = resource_creator.handle_ui_message(
                message,
                engine,
                sender.clone(),
                &self.current_path,
            );
            if asset_added {
                self.schedule_refresh();
            }
        }

        let ui = engine.user_interfaces.first_mut();

        if let Some(msg) = message.data::<AssetItemMessage>() {
            let asset_item = message.destination();
            match msg {
                AssetItemMessage::Select(true) => {
                    if ui.has_descendant_or_equal(asset_item, self.content_panel) {
                        self.on_asset_selected(asset_item, sender.clone(), entry, engine);
                    }
                }
                AssetItemMessage::MoveTo {
                    src_item_path,
                    dest_dir,
                } => {
                    if let (Some(file_name), true) =
                        (src_item_path.file_name(), src_item_path.is_file())
                    {
                        try_move_resource(
                            src_item_path,
                            &dest_dir.join(file_name),
                            &engine.resource_manager,
                        );
                        self.schedule_refresh();
                    } else if src_item_path.is_dir() {
                        Log::verify(block_on(engine.resource_manager.try_move_folder(
                            src_item_path,
                            dest_dir,
                            true,
                        )));
                        self.schedule_refresh();
                    }
                }
                _ => {}
            }
        } else if let Some(msg) = message.data_from::<FileBrowserMessage>(self.folder_browser) {
            match msg {
                FileBrowserMessage::Path(path) => {
                    ui.send(self.search_bar, SearchBarMessage::Text(Default::default()));
                    self.set_path(path, ui, &engine.resource_manager);
                }
                FileBrowserMessage::Drop {
                    dropped,
                    path,
                    dropped_path,
                    ..
                } => {
                    self.on_file_browser_drop(
                        *dropped,
                        path,
                        dropped_path,
                        ui,
                        &engine.resource_manager,
                    );
                }
                _ => (),
            }
        } else if let Some(SearchBarMessage::Text(search_text)) = message.data_from(self.search_bar)
        {
            if search_text.is_empty() {
                if let Some(selected_item_path) = entry
                    .selection
                    .as_ref::<AssetSelection>()
                    .and_then(|s| s.selected_path())
                {
                    let path = if selected_item_path != PathBuf::default() {
                        selected_item_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("./"))
                    } else {
                        PathBuf::from("./")
                    };
                    self.item_to_select = Some(selected_item_path.to_path_buf());
                    self.set_path(&path, ui, &engine.resource_manager);
                }
            } else {
                self.clear_assets(ui);
                let search_text = search_text.to_lowercase();

                let registry = engine.resource_manager.state().resource_registry.clone();
                let registry = registry.safe_lock();
                let mut paths = Vec::new();
                for resource_path in registry.inner().values() {
                    let file_stem = some_or_continue!(resource_path
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_lowercase()));

                    if file_stem.contains(&search_text)
                        || rust_fuzzy_search::fuzzy_compare(&search_text, &file_stem) >= 0.33
                    {
                        paths.push(resource_path.clone());
                    }
                }
                drop(registry);

                for path in paths.into_iter().filter(|p| !p.as_os_str().is_empty()) {
                    self.add_asset(&path, ui, &engine.resource_manager, &sender);
                }
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.context_menu.dependencies {
                if let Some(item) = engine
                    .user_interfaces
                    .first_mut()
                    .try_get_node(self.context_menu.placement_target)
                    .and_then(|n| n.cast::<AssetItem>())
                {
                    if let Ok(resource) =
                        block_on(engine.resource_manager.request_untyped(&item.path))
                    {
                        self.dependency_viewer.open(
                            &resource,
                            &engine.resource_manager,
                            engine.user_interfaces.first_mut(),
                        );
                    }
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if let Some(resource_creator) = self.resource_creator.as_ref() {
                if message.destination() == resource_creator.window {
                    engine
                        .user_interfaces
                        .first()
                        .send(resource_creator.window, WidgetMessage::Remove);

                    self.resource_creator = None;
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_resource {
                let resource_creator = ResourceCreator::new(
                    &mut engine.user_interfaces.first_mut().build_ctx(),
                    &engine.resource_manager,
                );

                resource_creator.open(engine.user_interfaces.first());

                self.resource_creator = Some(resource_creator);
            } else if message.destination() == self.refresh {
                self.schedule_refresh();
            } else if message.destination() == self.resave_resources {
                block_on(engine.resource_manager.resave_native_resources());
            }

            if let Some(inspector_buttons) = self.inspector_addon.as_ref() {
                fn default_import_options(
                    extension: &OsStr,
                    resource_manager: &ResourceManager,
                ) -> Option<Box<dyn BaseImportOptions>> {
                    let rm_state = resource_manager.state();
                    let loaders = rm_state.loaders.safe_lock();
                    for loader in loaders.iter() {
                        if loader.supports_extension(&extension.to_string_lossy()) {
                            return loader.default_import_options();
                        }
                    }
                    None
                }

                if let Some(selection) = entry.selection.as_ref::<AssetSelection>() {
                    if let Some(path) = selection.selected_path() {
                        if let Some(extension) = path.extension() {
                            let default_import_options =
                                default_import_options(extension, &engine.resource_manager);
                            if let Some(mut import_options) = selection.selected_import_options() {
                                if message.destination() == inspector_buttons.revert {
                                    if let Some(default_import_options) = default_import_options {
                                        *import_options = default_import_options;
                                        sender.send(Message::ForceSync);
                                    }
                                } else if message.destination() == inspector_buttons.apply {
                                    import_options.save(&append_extension(path, "options"));

                                    if let Ok(resource) =
                                        block_on(engine.resource_manager.request_untyped(path))
                                    {
                                        engine.resource_manager.state().reload_resource(resource);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(WidgetMessage::MouseDown { button, .. }) = message.data() {
            if ui.has_descendant_or_equal(message.destination(), self.scroll_panel)
                && !message.handled()
            {
                if let MouseButton::Back = *button {
                    let mut parent_folder = self
                        .current_path
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("./"));
                    if parent_folder == Path::new("") {
                        parent_folder = PathBuf::from("./");
                    }

                    self.set_path(&parent_folder, ui, &engine.resource_manager);

                    message.set_handled(true);
                }
            }
        }
    }

    pub fn locate_path(&mut self, ui: &UserInterface, path: PathBuf) {
        let folder = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        ui.send(self.folder_browser, FileBrowserMessage::Path(folder));
        self.item_to_select = Some(path);
    }

    fn schedule_refresh(&self) {
        self.need_refresh.store(true, Ordering::Relaxed);
    }

    fn update_icon_queue(&mut self, engine: &mut Engine) {
        let ui = engine.user_interfaces.first();
        for event in self.resource_event_receiver.try_iter() {
            if let ResourceEvent::Reloaded(resource) = event {
                let resource_path =
                    some_or_continue!(engine.resource_manager.resource_path(&resource));
                let canonical_resource_path = ok_or_continue!(resource_path.canonicalize());
                for item in self.items.iter() {
                    if let Some(asset_item) = ui.try_get_of_type::<AssetItem>(*item) {
                        let asset_item_path = ok_or_continue!(asset_item.path.canonicalize());
                        if asset_item_path == canonical_resource_path {
                            self.preview_sender
                                .send(IconRequest {
                                    widget_handle: *item,
                                    resource,
                                    force_update: true,
                                })
                                .unwrap();
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, engine: &mut Engine, sender: &MessageSender) {
        self.update_icon_queue(engine);
        self.preview_cache
            .update(&mut self.preview_generators, engine);
        if let Some(inspector_addon) = self.inspector_addon.as_mut() {
            inspector_addon.preview.update(engine);
        }
        if self.need_refresh.load(Ordering::Relaxed) {
            self.refresh(
                engine.user_interfaces.first_mut(),
                &engine.resource_manager,
                sender,
            );
            self.need_refresh.store(false, Ordering::Relaxed);
        }
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send(
            window_content(self.window, ui),
            WidgetMessage::Enabled(mode.is_edit()),
        );
    }

    pub fn on_message(
        &mut self,
        engine: &mut Engine,
        entry: &EditorSceneEntry,
        message: &Message,
        editor_plugins_container: &EditorPluginsContainer,
    ) {
        let ui = engine.user_interfaces.first_mut();
        if let Message::SelectionChanged { .. } = message {
            if let Some(selection) = entry.selection.as_ref::<AssetSelection>() {
                // Deselect other items.
                for &item_handle in self.items.iter() {
                    let item = some_or_continue!(ui.try_get_of_type::<AssetItem>(item_handle));

                    ui.send(
                        item_handle,
                        AssetItemMessage::Select(selection.contains(&item.path)),
                    );
                }

                if let Some(inspector_plugin) =
                    editor_plugins_container.try_get::<InspectorPlugin>()
                {
                    if self.inspector_addon.is_none() {
                        let preview = PreviewPanel::new(engine, 250, 250);
                        let ui = engine.user_interfaces.first_mut();
                        let ctx = &mut ui.build_ctx();
                        let apply;
                        let revert;
                        let buttons = StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_child({
                                    apply = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Apply")
                                    .build(ctx);
                                    apply
                                })
                                .with_child({
                                    revert = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Revert")
                                    .build(ctx);
                                    revert
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx);

                        ctx[preview.root].set_row(1).set_height(300.0);

                        let root = GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child(buttons)
                                .with_child(preview.root),
                        )
                        .add_column(Column::stretch())
                        .add_row(Row::auto())
                        .add_row(Row::stretch())
                        .build(ctx);

                        ctx.inner()
                            .send(root, WidgetMessage::LinkWith(inspector_plugin.footer));

                        self.inspector_addon = Some(InspectorAddon {
                            root,
                            apply,
                            revert,
                            preview,
                        })
                    }
                    let inspector_addon = self.inspector_addon.as_mut().unwrap();
                    let mut has_preview = false;
                    if let Some(asset_path) = selection.selected_path() {
                        if asset_path.is_file() {
                            if let Ok(resource) =
                                block_on(engine.resource_manager.request_untyped(asset_path))
                            {
                                if let Some(preview_generator) =
                                    resource.type_uuid().and_then(|type_uuid| {
                                        self.preview_generators.map.get_mut(&type_uuid)
                                    })
                                {
                                    let preview_scene =
                                        &mut engine.scenes[inspector_addon.preview.scene()];
                                    let preview = preview_generator.generate_scene(
                                        &resource,
                                        &engine.resource_manager,
                                        preview_scene,
                                        inspector_addon.preview.camera,
                                    );
                                    has_preview = preview.is_some();
                                    inspector_addon.preview.set_model(preview, engine);
                                }
                            }
                        }
                    }
                    engine.user_interfaces.first_mut().send(
                        inspector_addon.preview.root,
                        WidgetMessage::Visibility(has_preview),
                    );
                }
            } else {
                for &item in self.items.iter() {
                    ui.send(item, AssetItemMessage::Select(false))
                }

                if let Some(inspector_addon) = self.inspector_addon.take() {
                    ui.send(inspector_addon.root, WidgetMessage::Remove);
                    inspector_addon.preview.destroy(engine);
                }
            }
        }
    }

    fn on_file_browser_drop(
        &mut self,
        dropped: Handle<UiNode>,
        dest_dir: &Path,
        src_dir: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
    ) {
        if let Some(item) = ui.try_get_of_type::<AssetItem>(dropped) {
            if let Some(file_name) = item.path.file_name() {
                try_move_resource(&item.path, &dest_dir.join(file_name), resource_manager);
            }
        } else if src_dir != Path::new("") {
            Log::verify(block_on(
                resource_manager.try_move_folder(src_dir, dest_dir, true),
            ));
            self.schedule_refresh();
        }
    }
}
