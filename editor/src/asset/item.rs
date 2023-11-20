use crate::{gui::AssetItemMessage, load_image};
use fyrox::{
    asset::manager::ResourceManager,
    core::{algebra::Vector2, color::Color, pool::Handle},
    core::{reflect::prelude::*, visitor::prelude::*},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        draw::{CommandTexture, Draw, DrawingContext},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, UiMessage},
        text::TextBuilder,
        widget::{Widget, WidgetBuilder, WidgetMessage},
        BuildContext, Control, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode,
        UserInterface, BRUSH_DARKER, BRUSH_DARKEST,
    },
    resource::texture::Texture,
    utils::into_gui_texture,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[allow(dead_code)]
#[derive(Debug, Clone, Visit, Reflect)]
pub struct AssetItem {
    widget: Widget,
    pub path: PathBuf,
    pub kind: AssetKind,
    preview: Handle<UiNode>,
    selected: bool,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Visit, Reflect)]
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
        let texture =
            path.extension()
                .and_then(|ext| match ext.to_string_lossy().to_lowercase().as_ref() {
                    "jpg" | "tga" | "png" | "bmp" | "tif" | "tiff" => {
                        kind = AssetKind::Texture;
                        Some(into_gui_texture(resource_manager.request::<Texture>(&path)))
                    }
                    "fbx" | "rgs" => {
                        kind = AssetKind::Model;
                        load_image(include_bytes!("../../resources/embed/model.png"))
                    }
                    "ogg" | "wav" => {
                        kind = AssetKind::Sound;
                        load_image(include_bytes!("../../resources/embed/sound.png"))
                    }
                    "shader" => {
                        kind = AssetKind::Shader;
                        load_image(include_bytes!("../../resources/embed/shader.png"))
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
            kind,
            preview,
            selected: false,
        };
        ctx.add_node(UiNode::new(item))
    }
}
