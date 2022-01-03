use crate::{
    asset::{inspector::AssetInspector, item::AssetItemBuilder},
    gui::AssetItemMessage,
    preview::PreviewPanel,
    AssetItem, AssetKind, GameEngine, Message,
};
use rg3d::core::append_extension;
use rg3d::core::futures::executor::block_on;
use rg3d::engine::resource_manager::try_get_import_settings;
use rg3d::resource::texture::TextureImportOptions;
use rg3d::{
    core::{color::Color, pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        file_browser::{FileBrowserBuilder, FileBrowserMessage, Filter},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        text::{TextBuilder, TextMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        HorizontalAlignment, Orientation, UiNode, UserInterface, VerticalAlignment, BRUSH_DARK,
    },
};
use std::sync::mpsc::Sender;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

mod inspector;
pub mod item;

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
}

impl AssetBrowser {
    pub fn new(engine: &mut GameEngine) -> Self {
        let preview = PreviewPanel::new(engine, 250, 250);
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
                                    .with_background(Brush::Solid(Color::opaque(80, 80, 80)))
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

        self.preview.handle_message(message, engine);

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
                    block_on(self.preview.load_model(&path, engine));
                }
                AssetKind::Texture => {
                    let options = match block_on(try_get_import_settings(&item.path)) {
                        Some(options) => options,
                        None => {
                            // Create settings.
                            let options = TextureImportOptions::default();
                            options.save(&append_extension(&item.path, "options"));
                            options
                        }
                    };
                    self.inspector.inspect_resource_import_options(
                        &options,
                        &mut engine.user_interface,
                        sender,
                    )
                }
                AssetKind::Sound => {}
                AssetKind::Shader => {}
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
                            )
                        }

                        let entry_path = entry.path();
                        if !entry_path.is_dir() && entry_path.extension().map_or(false, check_ext) {
                            let asset_item = AssetItemBuilder::new(WidgetBuilder::new())
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
}
