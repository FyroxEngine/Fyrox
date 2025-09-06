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

use crate::{
    asset::{item::AssetItemMessage, preview::cache::IconRequest},
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource, TypedResourceData},
        core::{
            algebra::Vector2, log::Log, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
            visitor::prelude::*,
        },
        graph::SceneGraph,
        gui::{
            border::BorderBuilder,
            button::{ButtonBuilder, ButtonMessage},
            decorator::DecoratorBuilder,
            define_constructor, define_widget_deref,
            draw::DrawingContext,
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            image::{ImageBuilder, ImageMessage},
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            message::{MessageDirection, OsEvent, UiMessage},
            searchbar::{SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            text::{Text, TextBuilder},
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{Window, WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode,
            UserInterface, VerticalAlignment,
        },
    },
};
use fyrox::asset::Resource;
use fyrox::core::PhantomDataSendSync;
use fyrox::gui::brush::Brush;
use rust_fuzzy_search::fuzzy_compare;
use std::borrow::Cow;
use std::path::Path;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetSelectorMessage {
    Select(UntypedResource),
}

impl AssetSelectorMessage {
    define_constructor!(AssetSelectorMessage:Select => fn select(UntypedResource), layout: false);
}

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "aa4f0726-8d25-4c90-add1-92ba392310c6")]
struct Item {
    pub widget: Widget,
    image: Handle<UiNode>,
    path: PathBuf,
    #[visit(skip)]
    #[reflect(hidden)]
    sender: Sender<IconRequest>,
    #[visit(skip)]
    #[reflect(hidden)]
    need_request_preview: Cell<bool>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
}

define_widget_deref!(Item);

impl Control for Item {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle {
            if let Some(AssetItemMessage::Icon {
                texture,
                flip_y,
                color,
            }) = message.data()
            {
                ui.send_message(ImageMessage::texture(
                    self.image,
                    MessageDirection::ToWidget,
                    texture.clone(),
                ));
                ui.send_message(ImageMessage::flip(
                    self.image,
                    MessageDirection::ToWidget,
                    *flip_y,
                ));
                ui.send_message(WidgetMessage::background(
                    self.image,
                    MessageDirection::ToWidget,
                    Brush::Solid(*color).into(),
                ))
            }
        }
    }

    fn update(&mut self, _dt: f32, ui: &mut UserInterface) {
        if self.need_request_preview.get() {
            let parent_container_bounds = ui
                .find_up(self.parent, &mut |n| n.has_component::<ListView>())
                .map(|(_, node)| node.screen_bounds())
                .unwrap_or_default();

            let screen_bounds = self.screen_bounds();
            for corner in [
                screen_bounds.left_top_corner(),
                screen_bounds.right_top_corner(),
                screen_bounds.right_bottom_corner(),
                screen_bounds.left_bottom_corner(),
            ] {
                if parent_container_bounds.contains(corner) {
                    self.need_request_preview.set(false);

                    Log::verify(self.sender.send(IconRequest {
                        widget_handle: self.handle,
                        resource: self.resource_manager.request_untyped(self.path.as_path()),
                        force_update: false,
                    }));

                    break;
                }
            }
        }
    }
}

struct ItemBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
    sender: Sender<IconRequest>,
}

impl ItemBuilder {
    fn new(sender: Sender<IconRequest>, widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            sender,
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = path;
        self
    }

    fn build(self, resource_manager: ResourceManager, ctx: &mut BuildContext) -> Handle<UiNode> {
        let image = ImageBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Top)
                .with_height(64.0)
                .with_width(64.0)
                .with_margin(Thickness::uniform(1.0))
                .on_row(0),
        )
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new().with_child(image).with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .on_row(1)
                        .with_width(64.0)
                        .with_margin(Thickness::uniform(1.0)),
                )
                .with_vertical_text_alignment(VerticalAlignment::Top)
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_wrap(WrapMode::Letter)
                .with_text(
                    self.path
                        .file_name()
                        .map(|file_name| file_name.to_string_lossy().to_string())
                        .unwrap_or_default(),
                )
                .build(ctx),
            ),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_column(Column::auto())
        .build(ctx);

        let item = Item {
            widget: self
                .widget_builder
                .with_need_update(true)
                .with_child(
                    DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_child(content),
                    ))
                    .build(ctx),
                )
                .build(ctx),
            image,
            path: self.path,
            sender: self.sender,
            need_request_preview: Cell::new(true),
            resource_manager,
        };
        ctx.add_node(UiNode::new(item))
    }
}

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "970bb83b-51e8-48e7-8050-f97bf0ac470b")]
pub struct AssetSelector {
    pub widget: Widget,
    list_view: Handle<UiNode>,
    resources: Vec<PathBuf>,
    #[visit(skip)]
    #[reflect(hidden)]
    resource_manager: ResourceManager,
}

define_widget_deref!(AssetSelector);

impl Control for AssetSelector {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ListViewMessage::SelectionChanged(selected)) = message.data() {
            if message.destination() == self.list_view
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(first) = selected.first().cloned() {
                    if let Some(resource) = self.resources.get(first) {
                        ui.send_message(AssetSelectorMessage::select(
                            self.handle(),
                            MessageDirection::FromWidget,
                            self.resource_manager.request_untyped(resource),
                        ));
                    }
                }
            }
        } else if let Some(WidgetMessage::Focus) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                ui.send_message(WidgetMessage::focus(
                    self.list_view,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}

pub struct AssetSelectorBuilder<'a> {
    widget_builder: WidgetBuilder,
    asset_types: Vec<Uuid>,
    selected_asset_path: Cow<'a, Path>,
}

impl<'a> AssetSelectorBuilder<'a> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            asset_types: Default::default(),
            selected_asset_path: Default::default(),
        }
    }

    pub fn with_asset_types(mut self, asset_types: Vec<Uuid>) -> Self {
        self.asset_types = asset_types;
        self
    }

    pub fn with_selected_asset_path(mut self, selected_asset_path: Cow<'a, Path>) -> Self {
        self.selected_asset_path = selected_asset_path;
        self
    }

    pub fn build(
        self,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let state = resource_manager.state();
        let loaders = state.loaders.lock();
        let registry = state.resource_registry.lock();

        let mut supported_resource_paths = loaders
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

        supported_resource_paths.extend(state.built_in_resources.values().filter_map(|res| {
            let resource_state = res.resource.0.lock();
            if self
                .asset_types
                .contains(&resource_state.state.data_ref()?.type_uuid())
            {
                Some(res.id.clone())
            } else {
                None
            }
        }));

        supported_resource_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        let selection_index = supported_resource_paths
            .iter()
            .enumerate()
            .find(|(_, path)| path.as_path() == self.selected_asset_path)
            .map(|(i, _)| i);

        let items = supported_resource_paths
            .iter()
            .map(|path| {
                ItemBuilder::new(
                    icon_request_sender.clone(),
                    WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                )
                .with_path(path.clone())
                .build(resource_manager.clone(), ctx)
            })
            .collect::<Vec<_>>();

        let selected_item = selection_index.and_then(|i| items.get(i).cloned());

        let list_view = ListViewBuilder::new(WidgetBuilder::new())
            .with_items_panel(
                WrapPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_vertical_alignment(VerticalAlignment::Top)
                        .with_horizontal_alignment(HorizontalAlignment::Stretch),
                )
                .with_orientation(Orientation::Horizontal)
                .build(ctx),
            )
            .with_selection(selection_index.map(|i| vec![i]).unwrap_or_default())
            .with_items(items)
            .build(ctx);

        if let Some(selected_item) = selected_item {
            ctx.send_message(ListViewMessage::bring_item_into_view(
                list_view,
                MessageDirection::ToWidget,
                selected_item,
            ));
        }

        let selector = AssetSelector {
            widget: self.widget_builder.with_child(list_view).build(ctx),
            list_view,
            resources: supported_resource_paths,
            resource_manager: resource_manager.clone(),
        };
        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone, Debug, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "c348ad3d-52a6-40ad-a5e4-bf63fefe1906")]
pub struct AssetSelectorWindow {
    pub window: Window,
    selector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    selected_resource: Option<UntypedResource>,
    search_bar: Handle<UiNode>,
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

        if let Some(AssetSelectorMessage::Select(resource)) = message.data() {
            if message.destination() == self.selector
                && message.direction() == MessageDirection::FromWidget
            {
                self.selected_resource = Some(resource.clone());
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                if let Some(resource) = self.selected_resource.as_ref().cloned() {
                    ui.send_message(AssetSelectorMessage::select(
                        self.handle,
                        MessageDirection::FromWidget,
                        resource,
                    ));
                }
            }

            if message.destination() == self.cancel || message.destination() == self.ok {
                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(WindowMessage::Open { .. } | WindowMessage::OpenModal { .. }) =
            message.data()
        {
            if message.destination() == self.handle {
                ui.send_message(WidgetMessage::focus(
                    self.search_bar,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(SearchBarMessage::Text(text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                let selector = ui.try_get_of_type::<AssetSelector>(self.selector).unwrap();
                let items = &*ui
                    .try_get_of_type::<ListView>(selector.list_view)
                    .unwrap()
                    .items;

                let filter_text = text.to_lowercase();

                for item in items {
                    let mut matches = false;
                    for (_, widget) in ui.traverse_iter(*item) {
                        if let Some(text) = widget.cast::<Text>() {
                            let text = text.text().to_lowercase();
                            matches |= text.contains(&filter_text)
                                || fuzzy_compare(&filter_text, text.as_str()) >= 0.5;
                        }
                    }
                    ui.send_message(WidgetMessage::visibility(
                        *item,
                        MessageDirection::ToWidget,
                        matches || filter_text.is_empty(),
                    ));
                }
            }
        }
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

pub struct AssetSelectorWindowBuilder<'a> {
    window_builder: WindowBuilder,
    asset_types: Vec<Uuid>,
    selected_asset_path: Cow<'a, Path>,
}

impl<'a> AssetSelectorWindowBuilder<'a> {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            asset_types: Default::default(),
            selected_asset_path: Default::default(),
        }
    }

    pub fn with_asset_types(mut self, asset_types: Vec<Uuid>) -> Self {
        self.asset_types = asset_types;
        self
    }

    pub fn with_selected_asset_path(mut self, selected_asset_path: Cow<'a, Path>) -> Self {
        self.selected_asset_path = selected_asset_path;
        self
    }

    pub fn build(
        self,
        sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
        ctx: &mut BuildContext,
    ) -> Handle<UiNode> {
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_row(0)
                .with_margin(Thickness::uniform(2.0))
                .with_height(22.0)
                .with_tab_index(Some(0)),
        )
        .build(ctx);

        let selector =
            AssetSelectorBuilder::new(WidgetBuilder::new().on_row(1).with_tab_index(Some(1)))
                .with_selected_asset_path(self.selected_asset_path)
                .with_asset_types(self.asset_types)
                .build(sender, resource_manager, ctx);

        let ok;
        let cancel;
        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_margin(Thickness::uniform(2.0))
                .with_child({
                    ok = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_height(22.0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_tab_index(Some(2)),
                    )
                    .with_text("Select")
                    .build(ctx);
                    ok
                })
                .with_child({
                    cancel = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_height(22.0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_tab_index(Some(3)),
                    )
                    .with_text("Cancel")
                    .build(ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(search_bar)
                .with_child(selector)
                .with_child(buttons),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let window = AssetSelectorWindow {
            window: self.window_builder.with_content(content).build_window(ctx),
            selector,
            ok,
            cancel,
            selected_resource: None,
            search_bar,
        };

        ctx.add_node(UiNode::new(window))
    }

    pub fn build_for_type_and_open<T: TypedResourceData>(
        resource: Option<&Resource<T>>,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
        ui: &mut UserInterface,
    ) -> Handle<UiNode> {
        let selector = AssetSelectorWindowBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .with_title(WindowTitle::text("Select a Resource"))
                .with_remove_on_close(true)
                .open(false),
        )
        .with_selected_asset_path(
            resource
                .and_then(|resource| resource_manager.resource_path(resource))
                .unwrap_or_default()
                .into(),
        )
        .with_asset_types(vec![<T as TypeUuidProvider>::type_uuid()])
        .build(icon_request_sender, resource_manager, &mut ui.build_ctx());

        ui.send_message(WindowMessage::open_modal(
            selector,
            MessageDirection::ToWidget,
            true,
            false,
        ));

        selector
    }
}

#[derive(Reflect, Visit, Debug)]
pub struct AssetSelectorMixin<T: TypedResourceData> {
    pub selector: Cell<Handle<UiNode>>,
    pub select: Handle<UiNode>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub icon_request_sender: Sender<IconRequest>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub resource_manager: ResourceManager,
    #[visit(skip)]
    #[reflect(hidden)]
    pub phantom_data_send_sync: PhantomDataSendSync<T>,
}

impl<T: TypedResourceData> Clone for AssetSelectorMixin<T> {
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            select: self.select,
            icon_request_sender: self.icon_request_sender.clone(),
            resource_manager: self.resource_manager.clone(),
            phantom_data_send_sync: self.phantom_data_send_sync,
        }
    }
}

impl<T: TypedResourceData> AssetSelectorMixin<T> {
    pub fn new(
        select: Handle<UiNode>,
        icon_request_sender: Sender<IconRequest>,
        resource_manager: ResourceManager,
    ) -> Self {
        Self {
            selector: Default::default(),
            select,
            icon_request_sender,
            resource_manager,
            phantom_data_send_sync: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &self,
        resource: Option<&Resource<T>>,
        ui: &mut UserInterface,
        message: &UiMessage,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.select {
                self.selector
                    .set(AssetSelectorWindowBuilder::build_for_type_and_open::<T>(
                        resource,
                        self.icon_request_sender.clone(),
                        self.resource_manager.clone(),
                        ui,
                    ));
            }
        }
    }

    pub fn preview_ui_message<F>(&self, ui: &UserInterface, message: &mut UiMessage, converter: F)
    where
        F: FnOnce(&UntypedResource) -> UiMessage,
    {
        if message.destination() == self.selector.get() {
            if let Some(WindowMessage::Close) = message.data() {
                self.selector.set(Handle::NONE);
            } else if let Some(AssetSelectorMessage::Select(resource)) = message.data() {
                ui.send_message(converter(resource))
            }
        }
    }

    pub fn request_preview(&self, widget_handle: Handle<UiNode>, resource: &Resource<T>) {
        self.icon_request_sender
            .send(IconRequest {
                widget_handle,
                resource: resource.clone().into_untyped(),
                force_update: false,
            })
            .unwrap()
    }
}
