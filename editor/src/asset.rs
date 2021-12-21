use crate::{gui::AssetItemMessage, load_image, preview::PreviewPanel, GameEngine};
use rg3d::{
    core::{color::Color, pool::Handle, scope_profile},
    engine::resource_manager::ResourceManager,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        draw::{CommandTexture, Draw, DrawingContext},
        file_browser::{FileBrowserBuilder, FileBrowserMessage, Filter},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        text::{TextBuilder, TextMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_DARK,
    },
    utils::into_gui_texture,
};
use std::{
    any::{Any, TypeId},
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AssetItem {
    widget: Widget,
    pub path: PathBuf,
    pub kind: AssetKind,
    preview: Handle<UiNode>,
    selected: bool,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum AssetKind {
    Unknown,
    Model,
    Texture,
    Sound,
    Shader,
}

impl Deref for AssetItem {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for AssetItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Control for AssetItem {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(bounds, self.background(), CommandTexture::None, None);
        drawing_context.push_rect(&bounds, 1.0);
        drawing_context.commit(bounds, self.foreground(), CommandTexture::None, None);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(WidgetMessage::MouseDown { .. }) = message.data::<WidgetMessage>() {
            if !message.handled() {
                message.set_handled(true);
                ui.send_message(AssetItemMessage::select(
                    self.handle(),
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        } else if let Some(AssetItemMessage::Select(select)) = message.data::<AssetItemMessage>() {
            if self.selected != *select && message.destination() == self.handle() {
                self.selected = *select;
                ui.send_message(WidgetMessage::foreground(
                    self.handle(),
                    MessageDirection::ToWidget,
                    if *select {
                        Brush::Solid(Color::opaque(200, 220, 240))
                    } else {
                        Brush::Solid(Color::TRANSPARENT)
                    },
                ));
                ui.send_message(WidgetMessage::background(
                    self.handle(),
                    MessageDirection::ToWidget,
                    if *select {
                        Brush::Solid(Color::opaque(100, 100, 100))
                    } else {
                        Brush::Solid(Color::TRANSPARENT)
                    },
                ));
            }
        }
    }
}

pub struct AssetItemBuilder {
    widget_builder: WidgetBuilder,
    path: Option<PathBuf>,
}

impl AssetItemBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: None,
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_owned());
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let path = self.path.unwrap_or_default();
        let mut kind = AssetKind::Unknown;
        let texture = path
            .extension().and_then(|ext| match ext.to_string_lossy().to_lowercase().as_ref() {
                "jpg" | "tga" | "png" | "bmp" => {
                    kind = AssetKind::Texture;
                    Some(into_gui_texture(
                        resource_manager.request_texture(&path, None),
                    ))
                }
                "fbx" | "rgs" => {
                    kind = AssetKind::Model;
                    load_image(include_bytes!("../resources/embed/model.png"))
                }
                "ogg" | "wav" => {
                    kind = AssetKind::Sound;
                    load_image(include_bytes!("../resources/embed/sound.png"))
                }
                "shader" => {
                    kind = AssetKind::Shader;
                    load_image(include_bytes!("../resources/embed/shader.png"))
                }
                _ => None,
            });

        let preview = ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_width(60.0)
                .with_height(60.0),
        )
        .with_opt_texture(texture)
        .build(ctx);

        let item = AssetItem {
            widget: self
                .widget_builder
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drag(true)
                .with_foreground(Brush::Solid(Color::opaque(50, 50, 50)))
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_width(64.0)
                            .with_child(preview)
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_row(1),
                                )
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_text(
                                    &path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy(),
                                )
                                .build(ctx),
                            ),
                    )
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .build(ctx),
                )
                .build(),
            path,
            kind,
            preview,
            selected: false,
        };
        ctx.add_node(UiNode::new(item))
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
}

impl AssetBrowser {
    pub fn new(engine: &mut GameEngine) -> Self {
        let preview = PreviewPanel::new(engine, 250, 250);
        let mut ctx = engine.user_interface.build_ctx();

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
                                        .build(&mut ctx);
                                        folder_browser
                                    }),
                            )
                            .build(&mut ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(1)
                                    .with_child({
                                        selected_properties =
                                            TextBuilder::new(WidgetBuilder::new().on_row(0))
                                                .build(&mut ctx);
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
                                            .build(&mut ctx);
                                            content_panel
                                        })
                                        .build(&mut ctx);
                                        scroll_panel
                                    }),
                            )
                            .add_row(Row::strict(20.0))
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(&mut ctx),
                        )
                        .with_child(
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(2)
                                    .with_background(Brush::Solid(Color::opaque(80, 80, 80)))
                                    .with_child(preview.root),
                            )
                            .build(&mut ctx),
                        ),
                )
                .add_column(Column::strict(250.0))
                .add_column(Column::stretch())
                .add_column(Column::strict(250.0))
                .add_row(Row::stretch())
                .build(&mut ctx),
            )
            .build(&mut ctx);

        Self {
            window,
            content_panel,
            folder_browser,
            preview,
            scroll_panel,
            selected_properties,
            items: Default::default(),
            item_to_select: None,
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

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
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

            if item.kind == AssetKind::Model {
                let path = item.path.clone();
                rg3d::core::futures::executor::block_on(self.preview.load_model(&path, engine));
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
