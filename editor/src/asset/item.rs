use crate::gui::AssetItemMessage;
use crate::load_image;
use fyrox::core::color::Color;
use fyrox::core::pool::Handle;
use fyrox::engine::resource_manager::ResourceManager;
use fyrox::gui::brush::Brush;
use fyrox::gui::draw::{CommandTexture, Draw, DrawingContext};
use fyrox::gui::grid::{Column, GridBuilder, Row};
use fyrox::gui::image::ImageBuilder;
use fyrox::gui::message::{MessageDirection, UiMessage};
use fyrox::gui::text::TextBuilder;
use fyrox::gui::widget::{Widget, WidgetBuilder, WidgetMessage};
use fyrox::gui::{BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface};
use fyrox::utils::into_gui_texture;
use std::any::{Any, TypeId};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

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
        let texture =
            path.extension()
                .and_then(|ext| match ext.to_string_lossy().to_lowercase().as_ref() {
                    "jpg" | "tga" | "png" | "bmp" => {
                        kind = AssetKind::Texture;
                        Some(into_gui_texture(resource_manager.request_texture(&path)))
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
                                .with_text(&path.file_name().unwrap_or_default().to_string_lossy())
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
