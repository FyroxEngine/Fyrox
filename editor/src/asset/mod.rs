use crate::{
    asset::{
        inspector::{
            handlers::{
                model::ModelImportOptionsHandler, sound::SoundBufferImportOptionsHandler,
                texture::TextureImportOptionsHandler,
            },
            AssetInspector,
        },
        item::AssetItemBuilder,
    },
    gui::AssetItemMessage,
    preview::PreviewPanel,
    utils::window_content,
    AssetItem, AssetKind, GameEngine, Message, Mode,
};
use fyrox::{
    core::{
        color::Color, futures::executor::block_on, make_relative_path, pool::Handle, scope_profile,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        copypasta::ClipboardProvider,
        file_browser::{FileBrowserBuilder, FileBrowserMessage, Filter},
        grid::{Column, GridBuilder, Row},
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, HorizontalAlignment, Orientation, UiNode, UserInterface, VerticalAlignment,
        BRUSH_DARK,
    },
    utils::log::Log,
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};

mod inspector;
pub mod item;

struct ContextMenu {
    menu: Handle<UiNode>,
    open: Handle<UiNode>,
    copy_path: Handle<UiNode>,
    copy_file_name: Handle<UiNode>,
    show_in_explorer: Handle<UiNode>,
    delete: Handle<UiNode>,
    placement_target: Handle<UiNode>,
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

fn open_in_explorer<P: AsRef<OsStr>>(path: P) {
    execute_command(Command::new("explorer").arg(path))
}

fn put_path_to_clipboard(engine: &mut Engine, path: &OsStr) {
    if let Some(clipboard) = engine.user_interface.clipboard_mut() {
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
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            open,
            copy_path,
            delete,
            show_in_explorer,
            placement_target: Default::default(),
            copy_file_name,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu {
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
                    show_in_explorer(&item.path)
                } else if message.destination() == self.open {
                    open_in_explorer(&item.path)
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

pub struct AssetBrowser {
    pub window: Handle<UiNode>,
    content_panel: Handle<UiNode>,
    folder_browser: Handle<UiNode>,
    scroll_panel: Handle<UiNode>,
    selected_properties: Handle<UiNode>,
    preview: PreviewPanel,
    items: Vec<Handle<UiNode>>,
    item_to_select: Option<PathBuf>,
    inspector: AssetInspector,
    context_menu: ContextMenu,
}

impl AssetBrowser {
    pub fn new(engine: &mut GameEngine) -> Self {
        let preview = PreviewPanel::new(engine, 250, 250, true);
        let ctx = &mut engine.user_interface.build_ctx();

        let inspector = AssetInspector::new(ctx, 1, 0);

        let content_panel;
        let folder_browser;
        let selected_properties;
        let scroll_panel;
        let window = WindowBuilder::new(WidgetBuilder::new())
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
                                    .with_child({
                                        selected_properties =
                                            TextBuilder::new(WidgetBuilder::new().on_row(0))
                                                .build(ctx);
                                        selected_properties
                                    })
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
                            .add_row(Row::strict(20.0))
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

        Self {
            window,
            content_panel,
            folder_browser,
            preview,
            scroll_panel,
            selected_properties,
            items: Default::default(),
            item_to_select: None,
            inspector,
            context_menu,
        }
    }

    pub fn clear_preview(&mut self, engine: &mut GameEngine) {
        self.preview.clear(engine);
    }

    pub fn set_working_directory(&mut self, engine: &mut GameEngine, dir: &Path) {
        assert!(dir.is_dir());

        engine.user_interface.send_message(FileBrowserMessage::root(
            self.folder_browser,
            MessageDirection::ToWidget,
            Some(dir.to_owned()),
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &mut GameEngine,
        sender: Sender<Message>,
    ) {
        scope_profile!();

        self.inspector.handle_ui_message(message, engine);
        self.preview.handle_message(message, engine);
        self.context_menu.handle_ui_message(message, engine);

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

            let item = ui
                .node(message.destination())
                .cast::<AssetItem>()
                .expect("Must be AssetItem");
            ui.send_message(TextMessage::text(
                self.selected_properties,
                MessageDirection::ToWidget,
                format!("Path: {:?}", item.path),
            ));

            match item.kind {
                AssetKind::Unknown => {}
                AssetKind::Model => {
                    let path = item.path.clone();
                    block_on(self.preview.load_model(&path, true, engine));

                    self.inspector.inspect_resource_import_options(
                        ModelImportOptionsHandler::new(&path),
                        &mut engine.user_interface,
                        sender,
                    )
                }
                AssetKind::Texture => self.inspector.inspect_resource_import_options(
                    TextureImportOptionsHandler::new(&item.path),
                    &mut engine.user_interface,
                    sender,
                ),
                AssetKind::Sound => self.inspector.inspect_resource_import_options(
                    SoundBufferImportOptionsHandler::new(&item.path),
                    &mut engine.user_interface,
                    sender,
                ),
                AssetKind::Shader => {
                    Log::warn("Implement me!");
                }
                AssetKind::Absm => {
                    Log::warn("Implement me!");
                }
            }
        } else if let Some(FileBrowserMessage::Path(path)) = message.data::<FileBrowserMessage>() {
            if message.destination() == self.folder_browser
                && message.direction() == MessageDirection::FromWidget
            {
                let item_to_select = self.item_to_select.take();
                let mut handle_to_select = Handle::NONE;

                // Clean content panel first.
                for child in self.items.drain(..) {
                    ui.send_message(WidgetMessage::remove(child, MessageDirection::ToWidget));
                }

                // Get all supported assets from folder and generate previews for them.
                if let Ok(dir_iter) = std::fs::read_dir(path) {
                    for entry in dir_iter.flatten() {
                        fn check_ext(ext: &OsStr) -> bool {
                            let ext = ext.to_string_lossy().to_lowercase();
                            matches!(
                                ext.as_str(),
                                "rgs"
                                    | "fbx"
                                    | "jpg"
                                    | "tga"
                                    | "png"
                                    | "bmp"
                                    | "ogg"
                                    | "wav"
                                    | "shader"
                                    | "absm"
                            )
                        }

                        let entry_path = make_relative_path(entry.path());
                        if !entry_path.is_dir() && entry_path.extension().map_or(false, check_ext) {
                            let asset_item = AssetItemBuilder::new(
                                WidgetBuilder::new().with_context_menu(self.context_menu.menu),
                            )
                            .with_path(entry_path.clone())
                            .build(&mut ui.build_ctx(), engine.resource_manager.clone());

                            self.items.push(asset_item);

                            ui.send_message(WidgetMessage::link(
                                asset_item,
                                MessageDirection::ToWidget,
                                self.content_panel,
                            ));

                            if let Some(item_to_select) = item_to_select.as_ref() {
                                if item_to_select == &entry_path {
                                    handle_to_select = asset_item;
                                }
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

    pub fn update(&mut self, engine: &mut GameEngine) {
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
