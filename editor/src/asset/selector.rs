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

//! Asset selector is a small window widget that allows previewing and selecting assets of specified
//! types (more often - of a single type). It can be considered as a "tiny" asset browser, that has
//! no other functionality but previewing and selection.

use crate::fyrox::{
    asset::manager::ResourceManager,
    core::{pool::Handle, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    gui::{
        define_widget_deref,
        image::ImageBuilder,
        list_view::ListViewBuilder,
        message::UiMessage,
        widget::{Widget, WidgetBuilder},
        wrap_panel::WrapPanelBuilder,
        BuildContext, Control, Thickness, UiNode, UserInterface,
    },
};
use fyrox::core::algebra::Vector2;
use fyrox::gui::draw::DrawingContext;
use fyrox::gui::grid::{Column, GridBuilder, Row};
use fyrox::gui::message::OsEvent;
use fyrox::gui::text::TextBuilder;
use fyrox::gui::window::{Window, WindowBuilder};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "aa4f0726-8d25-4c90-add1-92ba392310c6")]
struct Item {
    pub widget: Widget,
    image: Handle<UiNode>,
    path: PathBuf,
}

define_widget_deref!(Item);

impl Control for Item {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

struct ItemBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
}

impl ItemBuilder {
    fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .with_height(40.0)
                .with_width(40.0)
                .on_row(0),
        )
        .build(ctx);

        let item = Item {
            widget: self
                .widget_builder
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new().with_child(image).with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_text(
                                    self.path
                                        .file_name()
                                        .map(|file_name| file_name.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                )
                                .build(ctx),
                        ),
                    )
                    .add_row(Row::stretch())
                    .add_row(Row::auto())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build(ctx),
            image,
            path: self.path,
        };
        ctx.add_node(UiNode::new(item))
    }
}

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "970bb83b-51e8-48e7-8050-f97bf0ac470b")]
pub struct AssetSelector {
    pub widget: Widget,
    list_view: Handle<UiNode>,
}

define_widget_deref!(AssetSelector);

impl Control for AssetSelector {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message)
    }
}

pub struct AssetSelectorBuilder {
    widget_builder: WidgetBuilder,
    asset_types: Vec<Uuid>,
}

impl AssetSelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            asset_types: Default::default(),
        }
    }

    pub fn with_asset_types(mut self, asset_types: Vec<Uuid>) -> Self {
        self.asset_types = asset_types;
        self
    }

    pub fn build(
        self,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let state = resource_manager.state();
        let loaders = state.loaders.lock();
        let registry = state.resource_registry.lock();

        let supported_resource_paths = loaders
            .iter()
            .filter_map(|loader| {
                if self.asset_types.contains(&loader.data_type_uuid()) {
                    Some(
                        registry
                            .inner()
                            .values()
                            .filter(|path| {
                                if let Some(ext) = path.extension().map(|ext| ext.to_string_lossy())
                                {
                                    loader.supports_extension(&ext)
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect::<Vec<_>>(),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        let items = supported_resource_paths
            .into_iter()
            .map(|path| {
                ItemBuilder::new(
                    WidgetBuilder::new()
                        .with_width(42.0)
                        .with_height(42.0)
                        .with_margin(Thickness::uniform(2.0)),
                )
                .with_path(path)
                .build(ctx)
            })
            .collect::<Vec<_>>();

        let list_view = ListViewBuilder::new(WidgetBuilder::new())
            .with_items_panel(WrapPanelBuilder::new(WidgetBuilder::new()).build(ctx))
            .with_items(items)
            .build(ctx);

        let selector = AssetSelector {
            widget: self.widget_builder.with_child(list_view).build(ctx),
            list_view,
        };
        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c348ad3d-52a6-40ad-a5e4-bf63fefe1906")]
pub struct AssetSelectorWindow {
    pub window: Window,
    selector: Handle<UiNode>,
}

impl Deref for AssetSelectorWindow {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for AssetSelectorWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl Control for AssetSelectorWindow {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.window.on_remove(sender);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.window.update(dt, ui);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event)
    }
}

pub struct AssetSelectorWindowBuilder {
    window_builder: WindowBuilder,
    asset_types: Vec<Uuid>,
}

impl AssetSelectorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            asset_types: Default::default(),
        }
    }

    pub fn with_asset_types(mut self, asset_types: Vec<Uuid>) -> Self {
        self.asset_types = asset_types;
        self
    }

    pub fn build(
        self,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let selector = AssetSelectorBuilder::new(WidgetBuilder::new()).build(resource_manager, ctx);

        let window = AssetSelectorWindow {
            window: self.window_builder.with_content(selector).build_window(ctx),
            selector,
        };

        ctx.add_node(UiNode::new(window))
    }
}
