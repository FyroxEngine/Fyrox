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

use fyrox::core::{log::Log, pool::NodeVariant};

use crate::{
    asset::open_in_explorer,
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource, Resource, TypedResourceData},
        core::{
            algebra::Vector2, color::Color, futures::executor::block_on, make_relative_path,
            parking_lot::lock_api::Mutex, pool::Handle, reflect::prelude::*, some_or_return,
            type_traits::prelude::*, uuid_provider, visitor::prelude::*,
        },
        graph::SceneGraph,
        gui::{
            border::BorderBuilder,
            brush::Brush,
            define_constructor,
            draw::{CommandTexture, Draw, DrawingContext},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            message::{MessageDirection, MouseButton, UiMessage},
            style::{resource::StyleResourceExt, Style},
            text::TextBuilder,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            BuildContext, Control, HorizontalAlignment, RcUiNodeHandle, Thickness, UiNode,
            UserInterface, VerticalAlignment,
        },
        material::Material,
        resource::texture::TextureResource,
        scene::tilemap::{brush::TileMapBrush, tileset::TileSet},
    },
    message::MessageSender,
    Message,
};
use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const DEFAULT_SIZE: f32 = 60.0;
pub const DEFAULT_VEC_SIZE: Vector2<f32> = Vector2::new(DEFAULT_SIZE, DEFAULT_SIZE);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetItemMessage {
    Select(bool),
    Icon {
        texture: Option<TextureResource>,
        flip_y: bool,
        color: Color,
    },
    MoveTo {
        src_item_path: PathBuf,
        dest_dir: PathBuf,
    },
}

impl AssetItemMessage {
    define_constructor!(AssetItemMessage:Select => fn select(bool), layout: false);
    define_constructor!(AssetItemMessage:Icon => fn icon(texture: Option<TextureResource>, flip_y: bool, color: Color), layout: false);
    define_constructor!(AssetItemMessage:MoveTo => fn move_to(src_item_path: PathBuf, dest_dir: PathBuf), layout: false);
}

#[allow(dead_code)]
#[derive(Debug, Clone, Visit, Reflect, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct AssetItem {
    widget: Widget,
    pub path: PathBuf,
    preview: Handle<UiNode>,
    selected: bool,
    text_border: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Option<MessageSender>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: Option<ResourceManager>,
}
impl NodeVariant<UiNode> for AssetItem {}

impl AssetItem {
    pub const SELECTED_PREVIEW: &'static str = "AssetItem.SelectedPreview";
    pub const SELECTED_TEXT_BORDER_BACKGROUND: &'static str =
        "AssetItem.SelectedTextBorderBackground";
    pub const TEXT_BORDER_DROP_BRUSH: &'static str = "AssetItem.TextBorderDropBrush";
    pub const DESELECTED_PREVIEW: &'static str = "AssetItem.DeselectedPreview";
    pub const DESELECTED_TEXT_BORDER_BACKGROUND: &'static str = "AssetItem.DeselectedTextBorder";
    pub const NORMAL_TEXT_BORDER_BRUSH: &'static str = "AssetItem.NormalTextBorderBrush";

    pub fn relative_path(&self) -> Result<PathBuf, std::io::Error> {
        let Some(resource_manager) = self.resource_manager.as_ref() else {
            return Err(std::io::Error::other("No resource manager".to_string()));
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

    pub fn untyped_resource(&self) -> Option<UntypedResource> {
        let resource_manager = self.resource_manager.as_ref()?;

        self.relative_path()
            .ok()
            .and_then(|path| block_on(resource_manager.request_untyped(path)).ok())
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
            .is_some_and(|ext| ext == "rgs" || ext == "ui")
        {
            sender.send(Message::LoadScene(self.path.clone()));
        } else if self.path.extension().is_some_and(|ext| ext == "material") {
            if let Ok(path) = make_relative_path(&self.path) {
                if let Ok(material) = block_on(resource_manager.request::<Material>(path)) {
                    sender.send(Message::OpenMaterialEditor(material));
                }
            }
        } else if self.path.extension().is_some_and(|ext| ext == "tileset") {
            if let Ok(path) = make_relative_path(&self.path) {
                match block_on(resource_manager.request::<TileSet>(path)) {
                    Ok(tile_set) => sender.send(Message::OpenTileSetEditor(tile_set)),
                    Err(err) => Log::err(format!("Open tileset error: {err:?}")),
                }
            }
        } else if self
            .path
            .extension()
            .is_some_and(|ext| ext == "tile_map_brush")
        {
            if let Ok(path) = make_relative_path(&self.path) {
                match block_on(resource_manager.request::<TileMapBrush>(path)) {
                    Ok(brush) => sender.send(Message::OpenTileMapBrushEditor(brush)),
                    Err(err) => Log::err(format!("Open tile_map_brush error: {err:?}")),
                }
            }
        } else if self.path.is_dir() {
            sender.send(Message::SetAssetBrowserCurrentDir(self.path.clone()));
        } else {
            open_in_explorer(&self.path)
        }
    }

    fn try_post_move_to_message(&self, ui: &UserInterface, dropped: Handle<UiNode>) {
        let dropped_item = some_or_return!(ui.try_get_of_type::<Self>(dropped).ok());

        if !self.path.is_dir() {
            return;
        }

        ui.send_message(AssetItemMessage::move_to(
            dropped,
            MessageDirection::FromWidget,
            dropped_item.path.clone(),
            self.path.clone(),
        ));
    }

    fn set_selected(&mut self, selected: bool, ui: &UserInterface) {
        if self.selected == selected {
            return;
        }
        self.selected = selected;

        let (preview_brush, text_border_brush) = if selected {
            (
                ui.style.property(Self::SELECTED_PREVIEW),
                ui.style.property(Self::SELECTED_TEXT_BORDER_BACKGROUND),
            )
        } else {
            (
                ui.style.property(Self::DESELECTED_PREVIEW),
                ui.style.property(Self::DESELECTED_TEXT_BORDER_BACKGROUND),
            )
        };

        ui.send_message(WidgetMessage::background(
            self.preview,
            MessageDirection::ToWidget,
            preview_brush,
        ));
        ui.send_message(WidgetMessage::background(
            self.text_border,
            MessageDirection::ToWidget,
            text_border_brush,
        ));
    }

    fn can_be_dropped_to(&self, dest: &AssetItem) -> bool {
        if self.path.is_file() && dest.path.is_dir() {
            self.resource_manager.as_ref().is_some_and(|rm| {
                block_on(rm.can_resource_be_moved(self.path.as_path(), dest.path.as_path(), true))
            })
        } else {
            self.path.is_dir() && dest.path.is_dir()
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
        drawing_context.push_rect_filled(&self.bounding_rect(), None);
        drawing_context.commit(
            self.clip_bounds(),
            Brush::Solid(Color::TRANSPARENT),
            CommandTexture::None,
            &self.material,
            None,
        );
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, .. } => {
                    if !message.handled() && !ui.keyboard_modifiers().alt {
                        if let MouseButton::Left | MouseButton::Right = *button {
                            message.set_handled(true);

                            ui.send_message(AssetItemMessage::select(
                                self.handle(),
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                    }
                }
                WidgetMessage::DragOver(dropped) => {
                    if ui
                        .try_get_of_type::<AssetItem>(*dropped)
                        .ok()
                        .is_some_and(|dropped| dropped.can_be_dropped_to(self))
                    {
                        ui.send_message(WidgetMessage::foreground(
                            self.text_border,
                            MessageDirection::ToWidget,
                            ui.style.property(Self::TEXT_BORDER_DROP_BRUSH),
                        ));
                    }
                }
                WidgetMessage::MouseLeave => {
                    ui.send_message(WidgetMessage::foreground(
                        self.text_border,
                        MessageDirection::ToWidget,
                        ui.style.property(Self::DESELECTED_TEXT_BORDER_BACKGROUND),
                    ));
                }
                WidgetMessage::Drop(dropped) => {
                    self.try_post_move_to_message(ui, *dropped);
                }
                WidgetMessage::DoubleClick { button, .. } => {
                    if *button == MouseButton::Left {
                        self.open();
                    }
                }
                _ => {}
            }
        } else if let Some(msg) = message.data::<AssetItemMessage>() {
            match msg {
                AssetItemMessage::Select(select) => {
                    if message.destination() == self.handle() {
                        self.set_selected(*select, ui);
                    }
                }
                AssetItemMessage::Icon {
                    texture,
                    flip_y,
                    color,
                } => {
                    ui.send_message(ImageMessage::texture(
                        self.preview,
                        MessageDirection::ToWidget,
                        texture.clone(),
                    ));
                    ui.send_message(ImageMessage::flip(
                        self.preview,
                        MessageDirection::ToWidget,
                        *flip_y,
                    ));
                    ui.send_message(WidgetMessage::background(
                        self.preview,
                        MessageDirection::ToWidget,
                        Brush::Solid(*color).into(),
                    ))
                }
                _ => (),
            }
        }
    }

    fn accepts_drop(&self, widget: Handle<UiNode>, ui: &UserInterface) -> bool {
        ui.try_get_of_type::<Self>(widget)
            .ok()
            .is_some_and(|asset_item| asset_item.can_be_dropped_to(self))
    }
}

pub struct AssetItemBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
    icon: Option<TextureResource>,
    title: Option<String>,
}

fn make_tooltip(ctx: &mut BuildContext, text: &str) -> RcUiNodeHandle {
    let handle = BorderBuilder::new(
        WidgetBuilder::new()
            .with_visibility(false)
            .with_foreground(ctx.style.property(Style::BRUSH_DARKEST))
            .with_background(ctx.style.property(Style::BRUSH_TEXT))
            .with_max_size(Vector2::new(300.0, f32::INFINITY))
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(2.0))
                        .with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
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
            path: Default::default(),
            icon: None,
            title: None,
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn with_icon(mut self, icon: Option<TextureResource>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn build(
        self,
        resource_manager: ResourceManager,
        message_sender: MessageSender,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let preview = ImageBuilder::new(
            WidgetBuilder::new()
                .with_background(ctx.style.property(AssetItem::DESELECTED_PREVIEW))
                .with_margin(Thickness::uniform(2.0))
                .with_width(DEFAULT_SIZE)
                .with_height(DEFAULT_SIZE),
        )
        .with_opt_texture(self.icon)
        .build(ctx);

        let text_border = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_tooltip(make_tooltip(ctx, &format!("{}", self.path.display())))
                .with_foreground(Brush::Solid(Color::TRANSPARENT).into())
                .with_background(
                    ctx.style
                        .property(AssetItem::DESELECTED_TEXT_BORDER_BACKGROUND),
                )
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                        .with_wrap(WrapMode::Letter)
                        .with_text(self.title.unwrap_or_else(|| {
                            self.path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        }))
                        .build(ctx),
                ),
        )
        .with_pad_by_corner_radius(false)
        .with_stroke_thickness(Thickness::uniform(2.0).into())
        .with_corner_radius(4.0.into())
        .build(ctx);

        let item = AssetItem {
            widget: self
                .widget_builder
                .with_user_data(Arc::new(Mutex::new(self.path.clone())))
                .with_margin(Thickness::uniform(1.0))
                .with_allow_drag(true)
                .with_allow_drop(true)
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_width(64.0)
                            .with_child(preview)
                            .with_child(text_border),
                    )
                    .add_column(Column::strict(64.0))
                    .add_row(Row::strict(64.0))
                    .add_row(Row::auto())
                    .build(ctx),
                )
                .build(ctx),
            path: self.path,
            preview,
            selected: false,
            text_border,
            sender: Some(message_sender),
            resource_manager: Some(resource_manager),
        };
        ctx.add_node(UiNode::new(item))
    }
}

#[cfg(test)]
mod test {
    use crate::asset::item::AssetItemBuilder;
    use fyrox::asset::io::FsResourceIo;
    use fyrox::asset::manager::ResourceManager;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};
    use std::sync::Arc;

    #[test]
    fn test_deletion() {
        let rm = ResourceManager::new(Arc::new(FsResourceIo), Default::default());
        test_widget_deletion(|ctx| {
            AssetItemBuilder::new(WidgetBuilder::new()).build(rm, Default::default(), ctx)
        });
    }
}
