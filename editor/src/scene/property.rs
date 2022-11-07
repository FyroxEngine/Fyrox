use fyrox::fxhash::FxHashSet;
use fyrox::{
    core::{algebra::Vector2, pool::Handle, reflect::Reflect},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        define_constructor, define_widget_deref,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, OsEvent, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        tree::{TreeBuilder, TreeRootBuilder, TreeRootMessage},
        utils,
        widget::{Widget, WidgetBuilder},
        window::{Window, WindowBuilder, WindowMessage},
        BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness,
        UiNode, UserInterface,
    },
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySelectorMessage {
    Selection(Vec<String>),
}

impl PropertySelectorMessage {
    define_constructor!(PropertySelectorMessage:Selection => fn selection(Vec<String>), layout: false);
}

pub struct PropertyDescriptor {
    path: String,
    display_name: String,
    type_name: &'static str,
    type_id: TypeId,
    children_properties: Vec<PropertyDescriptor>,
}

struct PropertyDescriptorData {
    path: String,
}

fn make_pretty_type_name(type_name: &str) -> String {
    let mut colon_position = None;
    let mut byte_pos = 0;
    for c in type_name.chars() {
        byte_pos += c.len_utf8();
        if c == ':' {
            colon_position = Some(byte_pos);
        } else if c == '<' {
            break;
        }
    }
    if let Some(colon_position) = colon_position {
        type_name.split_at(colon_position).1.to_owned()
    } else {
        type_name.to_owned()
    }
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

impl PropertyDescriptor {
    fn make_view(
        &self,
        ctx: &mut BuildContext,
        allowed_types: Option<&FxHashSet<TypeId>>,
    ) -> Handle<UiNode> {
        let items = make_views_for_property_descriptor_collection(
            ctx,
            &self.children_properties,
            allowed_types,
        );

        if !items.is_empty() || allowed_types.map_or(true, |types| types.contains(&self.type_id)) {
            TreeBuilder::new(
                WidgetBuilder::new().with_user_data(Rc::new(PropertyDescriptorData {
                    path: self.path.clone(),
                })),
            )
            .with_items(items)
            .with_content(
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .with_text(format!(
                        "{} ({})",
                        self.display_name,
                        make_pretty_type_name(self.type_name)
                    ))
                    .build(ctx),
            )
            .build(ctx)
        } else {
            Handle::NONE
        }
    }
}

pub fn object_to_property_tree(parent_path: &str, object: &dyn Reflect) -> Vec<PropertyDescriptor> {
    let mut descriptors = Vec::new();
    for (field_info, field_ref) in object.fields_info().iter().zip(object.fields()) {
        let path = if parent_path.is_empty() {
            field_info.name.to_owned()
        } else {
            format!("{}.{}", parent_path, field_info.name)
        };

        if let Some(array) = field_ref.as_array() {
            let mut descriptor = PropertyDescriptor {
                path: path.clone(),
                display_name: field_info.display_name.to_owned(),
                type_name: field_info.type_name,
                type_id: field_info.value.type_id(),
                children_properties: Default::default(),
            };

            for i in 0..array.reflect_len() {
                let item = array.reflect_index(i).unwrap();
                let item_path = format!("{}[{}]", path, i);
                descriptor.children_properties.push(PropertyDescriptor {
                    path: item_path.clone(),
                    display_name: format!("[{}]", i),
                    type_name: field_info.type_name,
                    type_id: field_info.value.type_id(),
                    children_properties: object_to_property_tree(&item_path, item),
                })
            }
        } else {
            descriptors.push(PropertyDescriptor {
                path,
                display_name: field_info.display_name.to_owned(),
                type_name: field_info.type_name,
                type_id: field_info.value.type_id(),
                children_properties: object_to_property_tree(field_info.name, field_ref),
            })
        }
    }
    descriptors
}

#[derive(Clone)]
pub struct PropertySelector {
    widget: Widget,
    selected_property_path: Vec<String>,
    tree_root: Handle<UiNode>,
    #[allow(dead_code)]
    allowed_types: Option<FxHashSet<TypeId>>,
}

define_widget_deref!(PropertySelector);

impl Control for PropertySelector {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
                                .user_data_ref::<PropertyDescriptorData>()
                                .unwrap()
                                .path
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
                self.selected_property_path = selection.clone();
                ui.send_message(message.reverse());
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
        let filter_text;
        let clear_filter;
        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_child({
                                filter_text = TextBoxBuilder::new(
                                    WidgetBuilder::new().on_row(0).on_column(0),
                                )
                                .build(ctx);
                                filter_text
                            })
                            .with_child({
                                clear_filter = ButtonBuilder::new(
                                    WidgetBuilder::new().on_row(0).on_column(1).with_width(20.0),
                                )
                                .with_content(utils::make_cross(ctx, 10.0, 2.0))
                                .build(ctx);
                                clear_filter
                            }),
                    )
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .add_row(Row::strict(22.0))
                    .build(ctx),
                )
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
            allowed_types: self.allowed_types,
        };

        ctx.add_node(UiNode::new(selector))
    }
}

#[derive(Clone)]
pub struct PropertySelectorWindow {
    window: Window,
    selector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
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

impl Control for PropertySelectorWindow {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.window.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.window.resolve(node_map)
    }

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

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.window.update(dt, sender)
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
                    .with_allowed_types(self.allowed_types)
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
        };

        ctx.add_node(UiNode::new(window))
    }
}
