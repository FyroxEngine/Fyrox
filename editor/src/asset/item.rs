use crate::{
    asset::open_in_explorer,
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource, Resource, TypedResourceData},
        core::{
            algebra::Vector2, color::Color, futures::executor::block_on, make_relative_path,
            pool::Handle, reflect::prelude::*, type_traits::prelude::*, uuid_provider,
            visitor::prelude::*,
        },
        gui::{
            border::BorderBuilder,
            brush::Brush,
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            message::{MessageDirection, UiMessage},
            text::TextBuilder,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode,
            UserInterface, BRUSH_DARKER, BRUSH_DARKEST,
        },
        material::Material,
        scene::tilemap::tileset::TileSet,
    },
    message::MessageSender,
    Message,
};
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetItemMessage {
    Select(bool),
    Icon {
        texture: Option<UntypedResource>,
        flip_y: bool,
    },
}

impl AssetItemMessage {
    define_constructor!(AssetItemMessage:Select => fn select(bool), layout: false);
    define_constructor!(AssetItemMessage:Icon => fn icon(texture: Option<UntypedResource>, flip_y: bool), layout: false);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Visit, Reflect, ComponentProvider)]
pub struct AssetItem {
    widget: Widget,
    pub path: PathBuf,
    preview: Handle<UiNode>,
    selected: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: Option<ResourceManager>,
}

impl AssetItem {
    pub fn relative_path(&self) -> Result<PathBuf, std::io::Error> {
        let Some(resource_manager) = self.resource_manager.as_ref() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No resource manager".to_string(),
            ));
        };

        if resource_manager
            .state()
            .built_in_resources
            .contains_key(&self.path)
        {
            Ok(self.path.clone())
        } else {
            make_relative_path(&self.path)
        }
    }

    pub fn resource<T: TypedResourceData>(&self) -> Option<Resource<T>> {
        let resource_manager = self.resource_manager.as_ref()?;

        self.relative_path()
            .ok()
            .and_then(|path| resource_manager.try_request::<T>(path))
            .and_then(|resource| block_on(resource).ok())
    }

    pub fn open(&self) {
        let Some(sender) = self.sender.as_ref() else {
            return;
        };

        let Some(resource_manager) = self.resource_manager.as_ref() else {
            return;
        };

        if self
            .path
            .extension()
            .map_or(false, |ext| ext == "rgs" || ext == "ui")
        {
            sender.send(Message::LoadScene(self.path.clone()));
        } else if self.path.extension().map_or(false, |ext| ext == "material") {
            if let Ok(path) = make_relative_path(&self.path) {
                if let Ok(material) = block_on(resource_manager.request::<Material>(path)) {
                    sender.send(Message::OpenMaterialEditor(material));
                }
            }
        } else if self.path.extension().map_or(false, |ext| ext == "tileset") {
            if let Ok(path) = make_relative_path(&self.path) {
                if let Ok(tile_set) = block_on(resource_manager.request::<TileSet>(path)) {
                    sender.send(Message::OpenTileSetEditor(tile_set));
                }
            }
        } else {
            open_in_explorer(&self.path)
        }
    }
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

uuid_provider!(AssetItem = "54f7d9c1-e707-4c8c-a5c9-3fc5cc80b545");

impl Control for AssetItem {
    fn draw(&self, drawing_context: &mut DrawingContext) {
        let bounds = self.bounding_rect();
        drawing_context.push_rect_filled(&bounds, None);
        drawing_context.commit(
            self.clip_bounds(),
            self.background(),
            CommandTexture::None,
            None,
        );
        drawing_context.push_rect(&bounds, 1.0);
        drawing_context.commit(
            self.clip_bounds(),
            self.foreground(),
            CommandTexture::None,
            None,
        );
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
        } else if let Some(msg) = message.data::<AssetItemMessage>() {
            match msg {
                AssetItemMessage::Select(select) => {
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
                AssetItemMessage::Icon { texture, flip_y } => {
                    ui.send_message(ImageMessage::texture(
                        self.preview,
                        MessageDirection::ToWidget,
                        texture.clone(),
                    ));
                    ui.send_message(ImageMessage::flip(
                        self.preview,
                        MessageDirection::ToWidget,
                        *flip_y,
                    ))
                }
            }
        } else if let Some(WidgetMessage::DoubleClick { .. }) = message.data() {
            self.open();
        }
    }
}

pub struct AssetItemBuilder {
    widget_builder: WidgetBuilder,
    path: Option<PathBuf>,
    icon: Option<UntypedResource>,
}

fn make_tooltip(ctx: &mut BuildContext, text: &str) -> RcUiNodeHandle {
    let handle = BorderBuilder::new(
        WidgetBuilder::new()
            .with_visibility(false)
            .with_foreground(BRUSH_DARKEST)
            .with_background(Brush::Solid(Color::opaque(230, 230, 230)))
            .with_max_size(Vector2::new(300.0, f32::INFINITY))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(2.0))
                        .with_foreground(BRUSH_DARKER),
                )
                .with_wrap(WrapMode::Letter)
                .with_text(text)
                .build(ctx),
            ),
    )
    .build(ctx);
    RcUiNodeHandle::new(handle, ctx.sender())
}

impl AssetItemBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: None,
            icon: None,
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = Some(path.as_ref().to_owned());
        self
    }

    pub fn with_icon(mut self, icon: Option<UntypedResource>) -> Self {
        self.icon = icon;
        self
    }

    pub fn build(
        self,
        resource_manager: ResourceManager,
        message_sender: MessageSender,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let path = self.path.unwrap_or_default();

        let preview = ImageBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_width(60.0)
                .with_height(60.0),
        )
        .with_opt_texture(self.icon)
        .build(ctx);

        let item = AssetItem {
            widget: self
                .widget_builder
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drag(true)
                .with_foreground(Brush::Solid(Color::opaque(50, 50, 50)))
                .with_tooltip(make_tooltip(ctx, &format!("{:?}", path)))
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
                                .with_text(path.file_name().unwrap_or_default().to_string_lossy())
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
            preview,
            selected: false,
            sender: Some(message_sender),
            resource_manager: Some(resource_manager),
        };
        ctx.add_node(UiNode::new(item))
    }
}
