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

use crate::fyrox::{
    core::{
        algebra::Vector2, make_pretty_type_name, parking_lot::Mutex, pool::Handle,
        reflect::prelude::*, sstorage::ImmutableString, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    fxhash::FxHashSet,
    graph::BaseSceneGraph,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        message::{KeyCode, MessageDirection, OsEvent, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder, TreeRootMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{Window, WindowBuilder, WindowMessage},
        BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};

use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySelectorMessage {
    Selection(Vec<PropertyDescriptorData>),
    ChooseFocus,
}

impl PropertySelectorMessage {
    define_constructor!(PropertySelectorMessage:Selection => fn selection(Vec<PropertyDescriptorData>), layout: false);
    define_constructor!(PropertySelectorMessage:ChooseFocus => fn choose_focus(), layout: false);
}

pub struct PropertyDescriptor {
    path: String,
    display_name: String,
    type_name: String,
    type_id: TypeId,
    read_only: bool,
    children_properties: Vec<PropertyDescriptor>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDescriptorData {
    pub name: String,
    pub path: String,
    pub type_id: TypeId,
}

fn make_views_for_property_descriptor_collection(
    ctx: &mut BuildContext,
    collection: &[PropertyDescriptor],
    allowed_types: Option<&FxHashSet<TypeId>>,
) -> Vec<Handle<UiNode>> {
    collection
        .iter()
        .filter_map(|p| {
            let view = p.make_view(ctx, allowed_types);
            if view.is_some() {
                Some(view)
            } else {
                None
            }
        })
        .collect()
}

fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
    let node_ref = ui.node(node);

    let mut is_any_match = false;
    for &child in node_ref.children() {
        is_any_match |= apply_filter_recursive(child, filter, ui)
    }

    if let Some(data) = node_ref
        .query_component::<Tree>()
        .and_then(|n| n.user_data_cloned::<PropertyDescriptorData>())
    {
        is_any_match |= data.name.to_lowercase().contains(filter);

        ui.send_message(WidgetMessage::visibility(
            node,
            MessageDirection::ToWidget,
            is_any_match,
        ));
    }

    is_any_match
}

impl PropertyDescriptor {
    fn make_view(
        &self,
        ctx: &mut BuildContext,
        allowed_types: Option<&FxHashSet<TypeId>>,
    ) -> Handle<UiNode> {
        if self.read_only {
            return Handle::NONE;
        }

        let items = make_views_for_property_descriptor_collection(
            ctx,
            &self.children_properties,
            allowed_types,
        );

        if !items.is_empty() || allowed_types.is_none_or(|types| types.contains(&self.type_id)) {
            let name = format!(
                "{} ({})",
                self.display_name,
                make_pretty_type_name(&self.type_name)
            );

            TreeBuilder::new(WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(
                PropertyDescriptorData {
                    name: name.clone(),
                    path: self.path.clone(),
                    type_id: self.type_id,
                },
            ))))
            .with_items(items)
            .with_content(
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .with_text(name)
                    .build(ctx),
            )
            .build(ctx)
        } else {
            Handle::NONE
        }
    }
}

pub fn object_to_property_tree<F>(
    parent_path: &str,
    object: &dyn Reflect,
    filter: &mut F,
) -> Vec<PropertyDescriptor>
where
    F: FnMut(&FieldRef) -> bool,
{
    let mut descriptors = Vec::new();

    object.fields_ref(&mut |fields_ref| {
        for field_info in fields_ref.iter() {
            if !filter(field_info) {
                continue;
            }

            let field_ref = field_info.value.field_value_as_reflect();

            let path = if parent_path.is_empty() {
                field_info.name.to_owned()
            } else {
                format!("{}.{}", parent_path, field_info.name)
            };

            let mut processed = true;

            field_ref.as_array(&mut |array| match array {
                Some(array) => {
                    let mut descriptor = PropertyDescriptor {
                        path: path.clone(),
                        display_name: field_info.display_name.to_owned(),
                        type_name: field_info.value.type_name().to_owned(),
                        type_id: field_info.value.type_id(),
                        children_properties: Default::default(),
                        read_only: field_info.read_only,
                    };

                    for i in 0..array.reflect_len() {
                        let item = array.reflect_index(i).unwrap();
                        let item_path = format!("{path}[{i}]");
                        descriptor.children_properties.push(PropertyDescriptor {
                            path: item_path.clone(),
                            display_name: format!("[{i}]"),
                            type_name: item.type_name().to_owned(),
                            type_id: item.type_id(),
                            read_only: field_info.read_only,
                            children_properties: object_to_property_tree(&item_path, item, filter),
                        })
                    }

                    descriptors.push(descriptor);
                }
                None => {
                    processed = false;
                }
            });

            if !processed {
                field_ref.as_hash_map(&mut |result| match result {
                    Some(hash_map) => {
                        let mut descriptor = PropertyDescriptor {
                            path: path.clone(),
                            display_name: field_info.display_name.to_owned(),
                            type_name: field_info.value.type_name().to_owned(),
                            type_id: field_info.value.type_id(),
                            children_properties: Default::default(),
                            read_only: field_info.read_only,
                        };

                        for i in 0..hash_map.reflect_len() {
                            let (key, value) = hash_map.reflect_get_at(i).unwrap();

                            // TODO: Here we just using `Debug` impl to obtain string representation for keys. This is
                            // fine for most cases in the engine.
                            let mut key_str = format!("{key:?}");

                            let mut is_key_string = false;
                            key.downcast_ref::<String>(&mut |string| {
                                is_key_string |= string.is_some()
                            });
                            key.downcast_ref::<ImmutableString>(&mut |string| {
                                is_key_string |= string.is_some()
                            });

                            if is_key_string {
                                // Strip quotes at the beginning and the end, because Debug impl for String adds
                                // quotes at the beginning and the end, but we want raw value.
                                // TODO: This is unreliable mechanism.
                                key_str.remove(0);
                                key_str.pop();
                            }

                            let item_path = format!("{path}[{key_str}]");

                            descriptor.children_properties.push(PropertyDescriptor {
                                path: item_path.clone(),
                                display_name: format!("[{key_str}]"),
                                type_name: value.type_name().to_owned(),
                                type_id: value.type_id(),
                                read_only: field_info.read_only,
                                children_properties: object_to_property_tree(
                                    &item_path, value, filter,
                                ),
                            })
                        }

                        descriptors.push(descriptor);

                        processed = true;
                    }
                    None => {
                        processed = false;
                    }
                })
            }

            if !processed {
                descriptors.push(PropertyDescriptor {
                    display_name: field_info.display_name.to_owned(),
                    type_name: field_info.value.type_name().to_owned(),
                    type_id: field_info.value.type_id(),
                    read_only: field_info.read_only,
                    children_properties: object_to_property_tree(&path, field_ref, filter),
                    path: path.clone(),
                })
            }
        }
    });

    descriptors
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct PropertySelector {
    widget: Widget,
    #[reflect(hidden)]
    #[visit(skip)]
    selected_property_paths: Vec<PropertyDescriptorData>,
    tree_root: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
}

define_widget_deref!(PropertySelector);

uuid_provider!(PropertySelector = "8e58e123-48a1-4e18-9e90-fd35a1669bdc");

impl PropertySelector {
    fn find_selected_tree_items(&self, ui: &UserInterface) -> Vec<Handle<UiNode>> {
        let mut stack = vec![self.tree_root];
        let mut selected_trees = Vec::new();

        while let Some(node_handle) = stack.pop() {
            let node = ui.node(node_handle);

            if let Some(tree) = node.query_component::<Tree>() {
                if self.selected_property_paths.iter().any(|path| {
                    path.path
                        == tree
                            .user_data_cloned::<PropertyDescriptorData>()
                            .unwrap()
                            .path
                }) {
                    selected_trees.push(node_handle);
                }
            }

            stack.extend_from_slice(node.children());
        }

        selected_trees
    }

    fn sync_selection(&self, ui: &UserInterface) {
        let selected_trees = self.find_selected_tree_items(ui);

        if let Some(first) = selected_trees.first() {
            ui.send_message(ScrollViewerMessage::bring_into_view(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                *first,
            ))
        }

        ui.send_message(TreeRootMessage::select(
            self.tree_root,
            MessageDirection::ToWidget,
            selected_trees,
        ));
    }
}

impl Control for PropertySelector {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TreeRootMessage::Selected(selection)) = message.data() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(PropertySelectorMessage::selection(
                    self.handle,
                    MessageDirection::ToWidget,
                    selection
                        .iter()
                        .map(|s| {
                            ui.node(*s)
                                .user_data_cloned::<PropertyDescriptorData>()
                                .unwrap()
                                .clone()
                        })
                        .collect(),
                ));
            }
        } else if let Some(msg) = message.data::<PropertySelectorMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    PropertySelectorMessage::Selection(selection) => {
                        if &self.selected_property_paths != selection {
                            self.selected_property_paths.clone_from(selection);
                            ui.send_message(message.reverse());
                        }
                    }
                    PropertySelectorMessage::ChooseFocus => {
                        ui.send_message(WidgetMessage::focus(
                            self.search_bar,
                            MessageDirection::ToWidget,
                        ));
                        self.sync_selection(ui);
                    }
                }
            }
        } else if let Some(SearchBarMessage::Text(filter_text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction() == MessageDirection::FromWidget
            {
                apply_filter_recursive(self.tree_root, &filter_text.to_lowercase(), ui);
            }
        }
    }
}

pub struct PropertySelectorBuilder {
    widget_builder: WidgetBuilder,
    property_descriptors: Vec<PropertyDescriptor>,
    allowed_types: Option<FxHashSet<TypeId>>,
    selected_property_paths: Vec<PropertyDescriptorData>,
}

impl PropertySelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            property_descriptors: Default::default(),
            allowed_types: Default::default(),
            selected_property_paths: Default::default(),
        }
    }

    pub fn with_allowed_types(mut self, allowed_types: Option<FxHashSet<TypeId>>) -> Self {
        self.allowed_types = allowed_types;
        self
    }

    pub fn with_property_descriptors(mut self, descriptors: Vec<PropertyDescriptor>) -> Self {
        self.property_descriptors = descriptors;
        self
    }

    pub fn with_selected_property_paths(mut self, paths: Vec<PropertyDescriptorData>) -> Self {
        self.selected_property_paths = paths;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let tree_root;
        let search_bar;

        let scroll_viewer =
            ScrollViewerBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_content({
                    tree_root = TreeRootBuilder::new(WidgetBuilder::new().with_tab_index(Some(1)))
                        .with_items(make_views_for_property_descriptor_collection(
                            ctx,
                            &self.property_descriptors,
                            self.allowed_types.as_ref(),
                        ))
                        .build(ctx);
                    tree_root
                })
                .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_bar = SearchBarBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_tab_index(Some(0)),
                    )
                    .build(ctx);
                    search_bar
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(ctx.style.property(Style::BRUSH_DARK))
                            .on_row(1)
                            .on_column(0)
                            .with_child(scroll_viewer),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let selector = PropertySelector {
            widget: self.widget_builder.with_child(content).build(ctx),
            selected_property_paths: self.selected_property_paths,
            tree_root,
            search_bar,
            scroll_viewer,
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
pub struct PropertySelectorWindow {
    #[component(include)]
    window: Window,
    selector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    #[reflect(hidden)]
    #[visit(skip)]
    allowed_types: Option<FxHashSet<TypeId>>,
}

impl Deref for PropertySelectorWindow {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window.widget
    }
}

impl DerefMut for PropertySelectorWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window.widget
    }
}

impl PropertySelectorWindow {
    pub fn confirm(&self, ui: &UserInterface) {
        ui.send_message(PropertySelectorMessage::selection(
            self.handle,
            MessageDirection::FromWidget,
            ui.node(self.selector)
                .query_component::<PropertySelector>()
                .unwrap()
                .selected_property_paths
                .clone(),
        ));
        ui.send_message(WindowMessage::close(
            self.handle,
            MessageDirection::ToWidget,
        ));
    }
}

uuid_provider!(PropertySelectorWindow = "725e4a10-eca6-4345-9833-d54dae2f20f2");

impl Control for PropertySelectorWindow {
    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.window.on_remove(sender)
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
        self.window.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                self.confirm(ui);
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(PropertySelectorMessage::Selection(selection)) = message.data() {
            if message.destination() == self.selector
                && message.direction() == MessageDirection::FromWidget
            {
                let enabled = selection.iter().all(|d| {
                    self.allowed_types
                        .as_ref()
                        .is_none_or(|types| types.contains(&d.type_id))
                });

                ui.send_message(WidgetMessage::enabled(
                    self.ok,
                    MessageDirection::ToWidget,
                    enabled,
                ));
            }
        } else if let Some(WindowMessage::Open { .. })
        | Some(WindowMessage::OpenAt { .. })
        | Some(WindowMessage::OpenModal { .. })
        | Some(WindowMessage::OpenAndAlign { .. }) = message.data()
        {
            ui.send_message(PropertySelectorMessage::choose_focus(
                self.selector,
                MessageDirection::ToWidget,
            ));
        } else if let Some(WidgetMessage::KeyDown(KeyCode::Enter | KeyCode::NumpadEnter)) =
            message.data()
        {
            if !message.handled() {
                self.confirm(ui);
                message.set_handled(true);
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

pub struct PropertySelectorWindowBuilder {
    window_builder: WindowBuilder,
    property_descriptors: Vec<PropertyDescriptor>,
    allowed_types: Option<FxHashSet<TypeId>>,
    selected_property_paths: Vec<PropertyDescriptorData>,
}

impl PropertySelectorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            property_descriptors: Default::default(),
            allowed_types: None,
            selected_property_paths: Default::default(),
        }
    }

    pub fn with_allowed_types(mut self, allowed_types: Option<FxHashSet<TypeId>>) -> Self {
        self.allowed_types = allowed_types;
        self
    }

    pub fn with_property_descriptors(mut self, descriptors: Vec<PropertyDescriptor>) -> Self {
        self.property_descriptors = descriptors;
        self
    }

    pub fn with_selected_property_paths(mut self, paths: Vec<PropertyDescriptorData>) -> Self {
        self.selected_property_paths = paths;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let selector;
        let ok;
        let cancel;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    selector = PropertySelectorBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_selected_property_paths(self.selected_property_paths)
                    .with_allowed_types(self.allowed_types.clone())
                    .with_property_descriptors(self.property_descriptors)
                    .build(ctx);
                    selector
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Right)
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(2.0))
                            .with_child({
                                ok = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tab_index(Some(2)),
                                )
                                .with_text("OK")
                                .build(ctx);
                                ok
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_tab_index(Some(3)),
                                )
                                .with_text("Cancel")
                                .build(ctx);
                                cancel
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_row(Row::strict(26.0))
        .add_column(Column::stretch())
        .build(ctx);

        let window = PropertySelectorWindow {
            window: self.window_builder.with_content(content).build_window(ctx),
            selector,
            ok,
            cancel,
            allowed_types: self.allowed_types,
        };

        ctx.add_node(UiNode::new(window))
    }
}

#[cfg(test)]
mod test {
    use crate::scene::property::{PropertySelectorBuilder, PropertySelectorWindowBuilder};
    use fyrox::gui::window::WindowBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| PropertySelectorBuilder::new(WidgetBuilder::new()).build(ctx));
        test_widget_deletion(|ctx| {
            PropertySelectorWindowBuilder::new(WindowBuilder::new(WidgetBuilder::new())).build(ctx)
        });
    }
}
