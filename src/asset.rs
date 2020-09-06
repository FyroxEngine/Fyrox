use crate::{
    gui::{
        AssetItemMessage, BuildContext, CustomWidget, EditorUiMessage, EditorUiNode, Ui, UiMessage,
        UiNode, UiWidgetBuilder,
    },
    GameEngine,
};
use rg3d::gui::text::TextBuilder;
use rg3d::gui::HorizontalAlignment;
use rg3d::{
    core::{color::Color, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        brush::Brush,
        draw::{CommandKind, CommandTexture, DrawingContext},
        file_browser::FileBrowserBuilder,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{FileBrowserMessage, UiMessageData, WidgetMessage},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        Control, Orientation, Thickness,
    },
    resource::{texture::Texture, texture::TextureKind},
    scene::{base::BaseBuilder, camera::CameraBuilder, node::Node, Scene},
    utils::into_any_arc,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub struct AssetItem {
    widget: CustomWidget,
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
}

impl Deref for AssetItem {
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl DerefMut for AssetItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl Clone for AssetItem {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            path: self.path.clone(),
            kind: AssetKind::Unknown,
            preview: self.preview,
            selected: self.selected,
        }
    }
}

impl Control<EditorUiMessage, EditorUiNode> for AssetItem {
    fn raw_copy(&self) -> UiNode {
        UiNode::User(EditorUiNode::AssetItem(self.clone()))
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.screen_bounds();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            CommandKind::Geometry,
            self.background(),
            CommandTexture::None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::MouseDown { .. } = msg {
                    if !message.handled {
                        message.handled = true;
                        ui.send_message(AssetItemMessage::select(self.handle(), !self.selected));
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let EditorUiMessage::AssetItem(msg) = msg {
                    if let &AssetItemMessage::Select(select) = msg {
                        if self.selected != select && message.destination == self.handle() {
                            self.selected = select;
                            let brush = if select {
                                Brush::Solid(Color::TRANSPARENT)
                            } else {
                                Brush::Solid(Color::RED)
                            };
                            ui.send_message(WidgetMessage::background(self.handle(), brush));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct AssetItemBuilder {
    widget_builder: UiWidgetBuilder,
    path: Option<PathBuf>,
}

impl AssetItemBuilder {
    pub fn new(widget_builder: UiWidgetBuilder) -> Self {
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
        resource_manager: &mut ResourceManager,
    ) -> Handle<UiNode> {
        let path = self.path.unwrap_or_default();
        let mut kind = AssetKind::Unknown;
        let texture = path
            .extension()
            .map(|ext| match ext.to_string_lossy().to_lowercase().as_ref() {
                "jpg" | "tga" | "png" | "bmp" => {
                    kind = AssetKind::Texture;
                    Some(resource_manager.request_texture_async(&path, TextureKind::RGBA8))
                }
                "fbx" | "rgs" => {
                    kind = AssetKind::Model;
                    Some(
                        resource_manager
                            .request_texture_async("resources/model.png", TextureKind::RGBA8),
                    )
                }
                _ => None,
            })
            .flatten();

        let preview = ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_width(64.0)
                .with_height(64.0),
        )
        .with_opt_texture(into_any_arc(texture))
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
                                    path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string(),
                                )
                                .with_wrap(true)
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
        ctx.add_node(UiNode::User(EditorUiNode::AssetItem(item)))
    }
}

pub struct AssetBrowser {
    pub window: Handle<UiNode>,
    content_panel: Handle<UiNode>,
    folder_browser: Handle<UiNode>,
    scene: Handle<Scene>,
    preview: Handle<UiNode>,
}

impl AssetBrowser {
    pub fn new(engine: &mut GameEngine) -> Self {
        let mut scene = Scene::new();
        scene
            .graph
            .add_node(Node::Camera(CameraBuilder::new(BaseBuilder::new()).build()));

        // Test model
        if let Some(model) = engine
            .resource_manager
            .lock()
            .unwrap()
            .request_model("data/mutant.FBX")
        {
            model.lock().unwrap().instantiate_geometry(&mut scene);
        }

        let render_target = Arc::new(Mutex::new(Texture::default()));
        scene.render_target = Some(render_target.clone());

        let scene = engine.scenes.add(scene);

        let mut ctx = engine.user_interface.build_ctx();

        let path = PathBuf::from("./data");
        let content_panel;
        let folder_browser;
        let preview;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("Asset Browser"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            folder_browser =
                                FileBrowserBuilder::new(WidgetBuilder::new().on_column(0))
                                    .with_path(&path) // TODO: Bind to project when it will be available.
                                    .with_filter(Rc::new(RefCell::new(|p: &Path| p.is_dir())))
                                    .build(&mut ctx);
                            folder_browser
                        })
                        .with_child({
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_column(1))
                                .with_content({
                                    content_panel = WrapPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(&mut ctx);
                                    content_panel
                                })
                                .build(&mut ctx)
                        })
                        .with_child({
                            preview = ImageBuilder::new(WidgetBuilder::new().on_column(2))
                                .with_flip(true)
                                .with_texture(render_target)
                                .build(&mut ctx);
                            preview
                        }),
                )
                .add_column(Column::strict(250.0))
                .add_column(Column::stretch())
                .add_column(Column::strict(250.0))
                .add_row(Row::stretch())
                .build(&mut ctx),
            )
            .build(&mut ctx);

        engine
            .user_interface
            .send_message(FileBrowserMessage::path(folder_browser, path));

        Self {
            window,
            content_panel,
            folder_browser,
            preview,
            scene,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        let ui = &mut engine.user_interface;
        let resource_manager = &mut engine.resource_manager.lock().unwrap();
        if message.destination == self.folder_browser {
            if let UiMessageData::FileBrowser(msg) = &message.data {
                if let FileBrowserMessage::Path(path) = msg {
                    // Clean content panel first.
                    for &child in ui.node(self.content_panel).children() {
                        ui.send_message(WidgetMessage::remove(child));
                    }
                    // Get all supported assets from folder and generate previews for them.
                    if let Ok(dir_iter) = std::fs::read_dir(path) {
                        for p in dir_iter {
                            if let Ok(entry) = p {
                                let entry_path = entry.path();
                                if !entry_path.is_dir() {
                                    if entry_path.extension().map_or(false, |ext| {
                                        let ext = ext.to_string_lossy().to_lowercase();
                                        match ext.as_str() {
                                            "rgs" | "fbx" | "jpg" | "tga" | "png" | "bmp" => true,
                                            _ => false,
                                        }
                                    }) {
                                        let content = AssetItemBuilder::new(WidgetBuilder::new())
                                            .with_path(entry_path)
                                            .build(&mut ui.build_ctx(), resource_manager);
                                        ui.send_message(WidgetMessage::link(
                                            content,
                                            self.content_panel,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
