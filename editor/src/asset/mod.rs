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
        inspector::AssetInspector,
        item::{AssetItem, AssetItemBuilder, AssetItemMessage},
        preview::{
            cache::{AssetPreviewCache, IconRequest},
            AssetPreviewGeneratorsCollection,
        },
    },
    fyrox::{
        asset::manager::ResourceManager,
        core::{futures::executor::block_on, log::Log, make_relative_path, pool::Handle},
        engine::Engine,
        graph::BaseSceneGraph,
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            copypasta::ClipboardProvider,
            dock::{DockingManagerBuilder, TileBuilder, TileContent},
            file_browser::{FileBrowserBuilder, FileBrowserMessage, Filter},
            grid::{Column, GridBuilder, Row},
            menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
            message::{MessageDirection, UiMessage},
            popup::{Placement, PopupBuilder, PopupMessage},
            scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            utils::{make_image_button_with_tooltip, make_simple_tooltip},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness, UiNode,
            UserInterface, VerticalAlignment,
        },
        walkdir,
    },
    load_image,
    message::MessageSender,
    preview::PreviewPanel,
    utils::window_content,
    Mode,
};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    ffi::OsStr,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc,
    },
};

mod creator;
mod dependency;
mod inspector;
pub mod item;
pub mod preview;

struct ContextMenu {
    menu: RcUiNodeHandle,
    open: Handle<UiNode>,
    duplicate: Handle<UiNode>,
    copy_path: Handle<UiNode>,
    copy_file_name: Handle<UiNode>,
    show_in_explorer: Handle<UiNode>,
    delete: Handle<UiNode>,
    placement_target: Handle<UiNode>,
    dependencies: Handle<UiNode>,
}

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
    } else {
        Log::err(format!("Failed to canonicalize path {:?}", path.as_ref()))
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

impl ContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete;
        let show_in_explorer;
        let open;
        let duplicate;
        let copy_path;
        let copy_file_name;
        let dependencies;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new()).with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            open = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Open"))
                                .build(ctx);
                            open
                        })
                        .with_child({
                            duplicate = MenuItemBuilder::new(WidgetBuilder::new())
                                .with_content(MenuItemContent::text("Duplicate"))
                                .build(ctx);
                            duplicate
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
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            open,
            duplicate,
            copy_path,
            delete,
            show_in_explorer,
            placement_target: Default::default(),
            copy_file_name,
            dependencies,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) -> bool {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if let Some(item) = engine
                .user_interfaces
                .first_mut()
                .try_get(self.placement_target)
                .and_then(|n| n.cast::<AssetItem>())
            {
                if message.destination() == self.delete {
                    Log::verify(std::fs::remove_file(&item.path));
                    return true;
                } else if message.destination() == self.show_in_explorer {
                    if let Ok(canonical_path) = item.path.canonicalize() {
                        show_in_explorer(canonical_path)
                    }
                } else if message.destination() == self.open {
                    item.open();
                } else if message.destination() == self.duplicate {
                    if let Some(resource) = item.untyped_resource() {
                        if let Some(path) = engine.resource_manager.resource_path(&resource) {
                            if let Some(built_in) = engine
                                .resource_manager
                                .state()
                                .built_in_resources
                                .get(&path)
                            {
                                if let Some(data_source) = built_in.data_source.as_ref() {
                                    let final_copy_path = make_unique_path(
                                        Path::new("."),
                                        path.to_str().unwrap(),
                                        &data_source.extension,
                                    );

                                    match File::create(&final_copy_path) {
                                        Ok(mut file) => {
                                            Log::verify(file.write_all(&data_source.bytes));
                                        }
                                        Err(err) => Log::err(format!(
                                            "Failed to create a file for resource at path {}. \
                                                Reason: {:?}",
                                            final_copy_path.display(),
                                            err
                                        )),
                                    }
                                }
                            } else if let Ok(canonical_path) = path.canonicalize() {
                                if let (Some(parent), Some(stem), Some(ext)) = (
                                    canonical_path.parent(),
                                    canonical_path.file_stem(),
                                    canonical_path.extension(),
                                ) {
                                    let stem = stem.to_string_lossy().to_string();
                                    let ext = ext.to_string_lossy().to_string();
                                    let final_copy_path = make_unique_path(parent, &stem, &ext);
                                    Log::verify(std::fs::copy(canonical_path, final_copy_path));
                                }
                            }
                        }
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

        false
    }
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
    preview: PreviewPanel,
    items: Vec<Handle<UiNode>>,
    item_to_select: Option<PathBuf>,
    inspector: AssetInspector,
    context_menu: ContextMenu,
    current_path: PathBuf,
    selected_item_path: PathBuf,
    watcher: Option<RecommendedWatcher>,
    dependency_viewer: DependencyViewer,
    resource_creator: Option<ResourceCreator>,
    preview_cache: AssetPreviewCache,
    preview_sender: Sender<IconRequest>,
    need_refresh: Arc<AtomicBool>,
    main_window: Handle<UiNode>,
    pub preview_generators: AssetPreviewGeneratorsCollection,
}

fn is_supported_resource(ext: &OsStr, resource_manager: &ResourceManager) -> bool {
    let Some(ext) = ext.to_str() else {
        return false;
    };

    resource_manager
        .state()
        .loaders
        .lock()
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
            let Some(extension) = path.extension() else {
                continue;
            };

            if is_supported_resource(extension, &resource_manager) {
                need_refresh_flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    })
    .ok()
}

impl AssetBrowser {
    pub fn new(engine: &mut Engine) -> Self {
        let preview = PreviewPanel::new(engine, 250, 250);
        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let inspector = AssetInspector::new(ctx, 1, 0);

        let add_resource = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(1))
                .with_height(24.0)
                .with_width(24.0)
                .with_margin(Thickness::uniform(1.0))
                .with_tooltip(make_simple_tooltip(ctx, "Add New Resource")),
        )
        .with_text("+")
        .build(ctx);

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
                .on_column(2)
                .with_height(22.0)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);

        let toolbar = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(add_resource)
                .with_child(refresh)
                .with_child(search_bar),
        )
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
                .with_show_path(false)
                .with_filter(Filter::new(|p: &Path| p.is_dir()))
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

        let preview_window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Asset Preview"))
            .with_tab_label("Preview")
            .can_close(false)
            .can_minimize(false)
            .can_maximize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(preview.root)
                        .with_child(inspector.container),
                )
                .add_column(Column::stretch())
                .add_row(Row::stretch())
                .add_row(Row::stretch())
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
                                .with_content(TileContent::HorizontalTiles {
                                    splitter: 0.75,
                                    tiles: [
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(main_window))
                                            .build(ctx),
                                        TileBuilder::new(WidgetBuilder::new())
                                            .with_content(TileContent::Window(preview_window))
                                            .build(ctx),
                                    ],
                                })
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

        let context_menu = ContextMenu::new(ctx);

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
            preview,
            scroll_panel,
            search_bar,
            items: Default::default(),
            item_to_select: None,
            inspector,
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
            selected_item_path: Default::default(),
        }
    }

    pub fn clear_preview(&mut self, engine: &mut Engine) {
        self.preview.clear(engine);
    }

    pub fn set_working_directory(
        &mut self,
        engine: &mut Engine,
        dir: &Path,
        message_sender: &MessageSender,
    ) {
        assert!(dir.is_dir());

        engine
            .user_interfaces
            .first_mut()
            .send_message(FileBrowserMessage::root(
                self.folder_browser,
                MessageDirection::ToWidget,
                Some(dir.to_owned()),
            ));

        engine
            .user_interfaces
            .first_mut()
            .send_message(FileBrowserMessage::path(
                self.folder_browser,
                MessageDirection::ToWidget,
                "./".into(),
            ));

        self.set_path(
            Path::new("./"),
            engine.user_interfaces.first_mut(),
            &engine.resource_manager,
            message_sender,
        );
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
            None
        })
        .with_path(path)
        .build(
            resource_manager.clone(),
            message_sender.clone(),
            &mut ui.build_ctx(),
        );

        if !is_dir {
            // Spawn async task, that will load the respective resource and generate preview for it in
            // a separate thread. This prevents blocking the main thread and thus keeps the editor
            // responsive.
            let rm = resource_manager.clone();
            let resource_path = path.to_path_buf();
            let preview_sender = self.preview_sender.clone();
            let task_pool = resource_manager.task_pool();
            task_pool.spawn_task(async move {
                if let Ok(resource) = rm.request_untyped(resource_path).await {
                    Log::verify(preview_sender.send(IconRequest {
                        resource,
                        asset_item,
                    }));
                }
            });
        }

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

    pub fn request_current_path(&self, path: PathBuf, ui: &UserInterface) {
        if !path.is_dir() {
            return;
        }

        ui.send_message(FileBrowserMessage::path(
            self.folder_browser,
            MessageDirection::ToWidget,
            path,
        ));
    }

    fn set_path(
        &mut self,
        path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        message_sender: &MessageSender,
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
        ui.send_message(WindowMessage::title(
            self.main_window,
            MessageDirection::ToWidget,
            WindowTitle::text(format!("Folder Content - {}", self.current_path.display())),
        ));
        self.refresh(ui, resource_manager, message_sender);
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
        if let Some(mut parent_path) = make_relative_path(&self.current_path)
            .ok()
            .and_then(|path| path.parent().map(|path| path.to_owned()))
        {
            if parent_path == PathBuf::default() {
                parent_path = "./".into();
            }

            let asset_item = AssetItemBuilder::new(
                WidgetBuilder::new().with_context_menu(self.context_menu.menu.clone()),
            )
            .with_icon(load_image!("../../resources/folder_return.png"))
            .with_path(parent_path)
            .build(
                resource_manager.clone(),
                message_sender.clone(),
                &mut ui.build_ctx(),
            );

            self.items.push(asset_item);

            ui.send_message(WidgetMessage::link(
                asset_item,
                MessageDirection::ToWidget,
                self.content_panel,
            ));
        }

        let mut folders = Vec::new();
        let mut resources = Vec::new();

        // Get all supported assets from folder and generate previews for them.
        if let Ok(dir_iter) = std::fs::read_dir(&self.current_path) {
            for entry in dir_iter.flatten() {
                if let Ok(entry_path) = make_relative_path(entry.path()) {
                    if entry_path.is_dir() {
                        folders.push(entry_path);
                    } else if entry_path
                        .extension()
                        .is_some_and(|ext| is_supported_resource(ext, resource_manager))
                    {
                        resources.push(entry_path);
                    }
                }
            }
        }

        // Collect built-in resource (only if the root folder is selected).
        if let Ok(path) = self.current_path.canonicalize() {
            if let Ok(working_dir) = std::env::current_dir().and_then(|dir| dir.canonicalize()) {
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
        self.inspector.handle_ui_message(message, engine);
        self.preview.handle_message(message, engine);
        if self.context_menu.handle_ui_message(message, engine) {
            self.refresh(
                engine.user_interfaces.first_mut(),
                &engine.resource_manager,
                &sender,
            );
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
                self.refresh(
                    engine.user_interfaces.first_mut(),
                    &engine.resource_manager,
                    &sender,
                );
            }
        }

        let ui = &mut engine.user_interfaces.first_mut();

        if let Some(AssetItemMessage::Select(true)) = message.data::<AssetItemMessage>() {
            // Deselect other items.
            for &item in self.items.iter().filter(|i| **i != message.destination()) {
                ui.send_message(AssetItemMessage::select(
                    item,
                    MessageDirection::ToWidget,
                    false,
                ))
            }

            let asset_path = ui
                .node(message.destination())
                .cast::<AssetItem>()
                .expect("Must be AssetItem")
                .path
                .clone();

            self.selected_item_path = asset_path.clone();

            self.inspector.inspect_resource_import_options(
                &asset_path,
                ui,
                sender,
                &engine.resource_manager,
            );

            if let Ok(resource) = block_on(engine.resource_manager.request_untyped(asset_path)) {
                if let Some(preview_generator) = resource
                    .type_uuid()
                    .and_then(|type_uuid| self.preview_generators.map.get_mut(&type_uuid))
                {
                    let preview_scene = &mut engine.scenes[self.preview.scene()];
                    let preview = preview_generator.generate_scene(
                        &resource,
                        &engine.resource_manager,
                        preview_scene,
                    );
                    ui.send_message(WidgetMessage::visibility(
                        self.preview.root,
                        MessageDirection::ToWidget,
                        preview.is_some(),
                    ));
                    self.preview.set_model(preview, engine);
                }
            }
        } else if let Some(msg) = message.data::<FileBrowserMessage>() {
            if message.destination() == self.folder_browser
                && message.direction() == MessageDirection::FromWidget
            {
                match msg {
                    FileBrowserMessage::Path(path) => {
                        ui.send_message(SearchBarMessage::text(
                            self.search_bar,
                            MessageDirection::ToWidget,
                            Default::default(),
                        ));
                        self.set_path(path, ui, &engine.resource_manager, &sender);
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
                            &sender,
                        );
                    }
                    _ => (),
                }
            }
        } else if let Some(SearchBarMessage::Text(search_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                if search_text.is_empty() {
                    let path = if self.selected_item_path != PathBuf::default() {
                        self.selected_item_path
                            .parent()
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("./"))
                    } else {
                        PathBuf::from("./")
                    };
                    self.item_to_select = Some(self.selected_item_path.clone());
                    self.set_path(&path, ui, &engine.resource_manager, &sender);
                } else {
                    self.clear_assets(ui);
                    let search_text = search_text.to_lowercase();

                    // TODO. This should be extracted from the project manifest.
                    let target_dir_path = Path::new("target").canonicalize();

                    for dir in std::fs::read_dir(".").into_iter().flatten().flatten() {
                        let path = dir.path();

                        // Ignore content of the `/target` folder, it contains build artifacts and
                        // they're useless anyway.
                        if let Ok(target_dir_path) = target_dir_path.as_ref() {
                            if let Ok(canonical_path) = path.canonicalize() {
                                if &canonical_path == target_dir_path {
                                    continue;
                                }
                            }
                        }

                        for dir in fyrox::walkdir::WalkDir::new(path).into_iter().flatten() {
                            if let Some(extension) = dir.path().extension() {
                                if is_supported_resource(extension, &engine.resource_manager) {
                                    let file_stem = dir
                                        .path()
                                        .file_stem()
                                        .map(|s| s.to_string_lossy().to_lowercase())
                                        .unwrap_or_default();
                                    if file_stem.contains(&search_text)
                                        || rust_fuzzy_search::fuzzy_compare(
                                            &search_text,
                                            file_stem.as_str(),
                                        ) >= 0.33
                                    {
                                        if let Ok(relative_path) = make_relative_path(dir.path()) {
                                            self.add_asset(
                                                &relative_path,
                                                ui,
                                                &engine.resource_manager,
                                                &sender,
                                            );
                                        }
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
                    .user_interfaces
                    .first_mut()
                    .try_get(self.context_menu.placement_target)
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
                        .first_mut()
                        .send_message(WidgetMessage::remove(
                            resource_creator.window,
                            MessageDirection::ToWidget,
                        ));

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
                self.refresh(
                    engine.user_interfaces.first_mut(),
                    &engine.resource_manager,
                    &sender,
                );
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

    pub fn update(&mut self, engine: &mut Engine, sender: &MessageSender) {
        self.preview_cache
            .update(&mut self.preview_generators, engine);
        self.preview.update(engine);
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
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }

    fn on_file_browser_drop(
        &mut self,
        dropped: Handle<UiNode>,
        target_dir: &Path,
        dropped_path: &Path,
        ui: &mut UserInterface,
        resource_manager: &ResourceManager,
        message_sender: &MessageSender,
    ) {
        if let Some(item) = ui.try_get(dropped).and_then(|n| n.cast::<AssetItem>()) {
            if let Ok(relative_path) = make_relative_path(target_dir) {
                if let Ok(resource) = block_on(resource_manager.request_untyped(&item.path)) {
                    if let Some(path) = resource_manager.resource_path(&resource) {
                        if let Some(file_name) = path.file_name() {
                            let new_full_path = relative_path.join(file_name);
                            Log::verify(block_on(
                                resource_manager.move_resource(&resource, new_full_path),
                            ));

                            self.refresh(ui, resource_manager, message_sender);
                        }
                    }
                }
            }
        } else if dropped_path != Path::new("") {
            if target_dir.starts_with(dropped_path) {
                // Trying to drop a folder into it's own subfolder
                return;
            }
            // At this point we have a folder dropped on some other folder. In this case
            // we need to move all the assets from the dropped folder to a new subfolder (with the same
            // name as the dropped folder) of the other folder first. After that we can move the rest
            // of the files and finally delete the dropped folder.
            let mut what_where_stack = vec![(dropped_path.to_path_buf(), target_dir.to_path_buf())];
            while let Some((src_dir, target_dir)) = what_where_stack.pop() {
                if let Some(src_dir_name) = src_dir.file_name() {
                    let target_sub_dir = target_dir.join(src_dir_name);
                    if !target_sub_dir.exists() {
                        Log::verify(std::fs::create_dir(&target_sub_dir));
                    }

                    if target_sub_dir.exists() {
                        for entry in walkdir::WalkDir::new(&src_dir)
                            .max_depth(1)
                            .into_iter()
                            .filter_map(|e| e.ok())
                        {
                            if entry.path().is_file() {
                                if let Ok(target_sub_dir_normalized) =
                                    make_relative_path(&target_sub_dir)
                                {
                                    if let Ok(resource) =
                                        block_on(resource_manager.request_untyped(entry.path()))
                                    {
                                        if let Some(path) =
                                            resource_manager.resource_path(&resource)
                                        {
                                            if let Some(file_name) = path.file_name() {
                                                let new_full_path =
                                                    target_sub_dir_normalized.join(file_name);
                                                Log::verify(block_on(
                                                    resource_manager
                                                        .move_resource(&resource, new_full_path),
                                                ));
                                            }
                                        }
                                    }
                                }
                            } else if entry.path().is_dir() && entry.path() != src_dir {
                                // Sub-folders will be processed after all assets from current dir
                                // were moved.
                                what_where_stack
                                    .push((entry.path().to_path_buf(), target_sub_dir.clone()));
                            }
                        }
                    }
                }
            }

            self.refresh(ui, resource_manager, message_sender);
        }
    }
}
