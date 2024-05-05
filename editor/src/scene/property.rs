use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        algebra::Vector2, make_pretty_type_name, parking_lot::Mutex, pool::Handle,
        reflect::prelude::*, sstorage::ImmutableString, type_traits::prelude::*, uuid_provider,
        visitor::prelude::*,
    },
    fxhash::FxHashSet,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, OsEvent, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder, TreeRootMessage},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{Window, WindowBuilder, WindowMessage},
        BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySelectorMessage {
    Selection(Vec<PropertyDescriptorData>),
}

impl PropertySelectorMessage {
    define_constructor!(PropertySelectorMessage:Selection => fn selection(Vec<PropertyDescriptorData>), layout: false);
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

        if !items.is_empty() || allowed_types.map_or(true, |types| types.contains(&self.type_id)) {
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
    F: FnMut(&FieldInfo) -> bool,
{
    let mut descriptors = Vec::new();

    object.fields_info(&mut |fields_info| {
        for field_info in fields_info.iter() {
            if !filter(field_info) {
                continue;
            }

            let field_ref = field_info.reflect_value;

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
                        type_name: field_info.type_name.to_owned(),
                        type_id: field_info.value.type_id(),
                        children_properties: Default::default(),
                        read_only: field_info.read_only,
                    };

                    for i in 0..array.reflect_len() {
                        let item = array.reflect_index(i).unwrap();
                        let item_path = format!("{}[{}]", path, i);
                        descriptor.children_properties.push(PropertyDescriptor {
                            path: item_path.clone(),
                            display_name: format!("[{}]", i),
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
                            type_name: field_info.type_name.to_owned(),
                            type_id: field_info.value.type_id(),
                            children_properties: Default::default(),
                            read_only: field_info.read_only,
                        };

                        for i in 0..hash_map.reflect_len() {
                            let (key, value) = hash_map.reflect_get_at(i).unwrap();

                            // TODO: Here we just using `Debug` impl to obtain string representation for keys. This is
                            // fine for most cases in the engine.
                            let mut key_str = format!("{:?}", key);

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

                            let item_path = format!("{}[{}]", path, key_str);

                            descriptor.children_properties.push(PropertyDescriptor {
                                path: item_path.clone(),
                                display_name: format!("[{}]", key_str),
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
                    type_name: field_info.type_name.to_owned(),
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
pub struct PropertySelector {
    widget: Widget,
    #[reflect(hidden)]
    #[visit(skip)]
    selected_property_path: Vec<PropertyDescriptorData>,
    tree_root: Handle<UiNode>,
    search_bar: Handle<UiNode>,
}

define_widget_deref!(PropertySelector);

uuid_provider!(PropertySelector = "8e58e123-48a1-4e18-9e90-fd35a1669bdc");

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
        } else if let Some(PropertySelectorMessage::Selection(selection)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && &self.selected_property_path != selection
            {
                self.selected_property_path.clone_from(selection);
                ui.send_message(message.reverse());
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
}

impl PropertySelectorBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            property_descriptors: Default::default(),
            allowed_types: Default::default(),
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

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let tree_root;
        let search_bar;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_bar = SearchBarBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    search_bar
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(fyrox::gui::BRUSH_DARK)
                            .on_row(1)
                            .on_column(0)
                            .with_child(
                                ScrollViewerBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_content({
                                    tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                                        .with_items(make_views_for_property_descriptor_collection(
                                            ctx,
                                            &self.property_descriptors,
                                            self.allowed_types.as_ref(),
                                        ))
                                        .build(ctx);
                                    tree_root
                                })
                                .build(ctx),
                            ),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(22.0))
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let selector = PropertySelector {
            widget: self.widget_builder.with_child(content).build(),
            selected_property_path: Default::default(),
            tree_root,
            search_bar,
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone, Visit, Reflect, Debug, ComponentProvider)]
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
                ui.send_message(PropertySelectorMessage::selection(
                    self.handle,
                    MessageDirection::FromWidget,
                    ui.node(self.selector)
                        .query_component::<PropertySelector>()
                        .unwrap()
                        .selected_property_path
                        .clone(),
                ));

                ui.send_message(WindowMessage::close(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
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
                        .map_or(true, |types| types.contains(&d.type_id))
                });

                ui.send_message(WidgetMessage::enabled(
                    self.ok,
                    MessageDirection::ToWidget,
                    enabled,
                ));
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
}

impl PropertySelectorWindowBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
        Self {
            window_builder,
            property_descriptors: Default::default(),
            allowed_types: None,
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
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("OK")
                                .build(ctx);
                                ok
                            })
                            .with_child({
                                cancel = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_margin(Thickness::uniform(1.0)),
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
        .add_row(Row::strict(22.0))
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
