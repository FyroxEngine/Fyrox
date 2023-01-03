#![allow(clippy::manual_map)]

use crate::{
    animation::{
        command::{AddTrackCommand, RemoveTrackCommand, SetTrackEnabledCommand},
        selection::{AnimationSelection, SelectedEntity},
    },
    load_image,
    menu::create_menu_item,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene, Selection,
    },
    send_sync_message, utils, Message,
};
use fyrox::{
    animation::{
        container::{TrackDataContainer, TrackValueKind},
        track::Track,
        value::{ValueBinding, ValueType},
        Animation,
    },
    core::{
        algebra::{UnitQuaternion, Vector2, Vector3, Vector4},
        pool::Handle,
        reflect::ResolvePath,
        uuid::Uuid,
        variable::InheritableVariable,
    },
    fxhash::{FxHashMap, FxHashSet},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        define_constructor,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        menu::MenuItemMessage,
        message::{MessageDirection, OsEvent, UiMessage},
        popup::PopupBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder, TextMessage},
        text_box::{TextBoxBuilder, TextCommitMode},
        tree::{Tree, TreeBuilder, TreeMessage, TreeRootBuilder, TreeRootMessage},
        utils::{make_cross, make_simple_tooltip},
        widget::{Widget, WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Control, NodeHandleMapping, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_BRIGHT,
    },
    scene::{animation::AnimationPlayer, graph::Graph, node::Node, Scene},
    utils::log::Log,
};
use std::{
    any::{Any, TypeId},
    cmp::Ordering,
    collections::hash_map::Entry,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::mpsc::Sender,
};

struct TrackContextMenu {
    menu: Handle<UiNode>,
    remove_track: Handle<UiNode>,
}

impl TrackContextMenu {
    fn new(ctx: &mut BuildContext) -> Self {
        let remove_track;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    remove_track = create_menu_item("Remove Selected Tracks", vec![], ctx);
                    remove_track
                }))
                .build(ctx),
            )
            .build(ctx);

        Self { menu, remove_track }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackViewMessage {
    TrackEnabled(bool),
}

impl TrackViewMessage {
    define_constructor!(TrackViewMessage:TrackEnabled => fn track_enabled(bool), layout: false);
}

#[derive(Clone)]
struct TrackView {
    tree: Tree,
    id: Uuid,
    target: Handle<Node>,
    track_enabled_switch: Handle<UiNode>,
    track_enabled: bool,
}

impl Deref for TrackView {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree.widget
    }
}

impl DerefMut for TrackView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree.widget
    }
}

impl Control for TrackView {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.tree.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.tree.resolve(node_map)
    }

    fn on_remove(&self, sender: &Sender<UiMessage>) {
        self.tree.on_remove(sender)
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.tree.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.tree.update(dt, sender)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.track_enabled_switch
                && message.direction() == MessageDirection::FromWidget
                && self.track_enabled != *value
            {
                ui.send_message(TrackViewMessage::track_enabled(
                    self.handle,
                    MessageDirection::ToWidget,
                    *value,
                ));
            }
        } else if let Some(TrackViewMessage::TrackEnabled(enabled)) = message.data() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
                && self.track_enabled != *enabled
            {
                self.track_enabled = *enabled;

                ui.send_message(CheckBoxMessage::checked(
                    self.track_enabled_switch,
                    MessageDirection::ToWidget,
                    Some(*enabled),
                ));

                ui.send_message(message.reverse());
            }
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.tree.preview_message(ui, message)
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.tree.handle_os_event(self_handle, ui, event)
    }
}

struct TrackViewBuilder {
    tree_builder: TreeBuilder,
    id: Uuid,
    target: Handle<Node>,
    name: String,
    track_enabled: bool,
}

impl TrackViewBuilder {
    pub fn new(tree_builder: TreeBuilder) -> Self {
        Self {
            tree_builder,
            id: Default::default(),
            target: Default::default(),
            name: Default::default(),
            track_enabled: true,
        }
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn with_target(mut self, target: Handle<Node>) -> Self {
        self.target = target;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_track_enabled(mut self, track_enabled: bool) -> Self {
        self.track_enabled = track_enabled;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let track_enabled_switch = CheckBoxBuilder::new(WidgetBuilder::new().with_height(18.0))
            .with_content(
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                    .with_text(self.name)
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            )
            .checked(Some(self.track_enabled))
            .build(ctx);

        let track_view = TrackView {
            tree: self
                .tree_builder
                .with_content(track_enabled_switch)
                .build_tree(ctx),
            id: self.id,
            target: self.target,
            track_enabled: self.track_enabled,
            track_enabled_switch,
        };

        ctx.add_node(UiNode::new(track_view))
    }
}

struct Toolbar {
    panel: Handle<UiNode>,
    search_text: Handle<UiNode>,
    clear_search_text: Handle<UiNode>,
    collapse_all: Handle<UiNode>,
    expand_all: Handle<UiNode>,
}

impl Toolbar {
    fn new(ctx: &mut BuildContext) -> Self {
        let search_text;
        let clear_search_text;
        let collapse_all;
        let expand_all;
        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_text = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(0),
                    )
                    .with_text_commit_mode(TextCommitMode::Immediate)
                    .build(ctx);
                    search_text
                })
                .with_child({
                    clear_search_text = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(1)
                            .with_tooltip(make_simple_tooltip(ctx, "Clear Filter Text")),
                    )
                    .with_content(make_cross(ctx, 12.0, 2.0))
                    .build(ctx);
                    clear_search_text
                })
                .with_child({
                    collapse_all = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(2)
                            .with_tooltip(make_simple_tooltip(ctx, "Collapse All")),
                    )
                    .with_content(
                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_background(BRUSH_BRIGHT)
                                .with_width(16.0)
                                .with_height(16.0),
                        )
                        .with_opt_texture(load_image(include_bytes!(
                            "../../resources/embed/collapse.png"
                        )))
                        .build(ctx),
                    )
                    .build(ctx);
                    collapse_all
                })
                .with_child({
                    expand_all = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(3)
                            .with_tooltip(make_simple_tooltip(ctx, "Expand All")),
                    )
                    .with_content(
                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_background(BRUSH_BRIGHT)
                                .with_width(16.0)
                                .with_height(16.0),
                        )
                        .with_opt_texture(load_image(include_bytes!(
                            "../../resources/embed/expand.png"
                        )))
                        .build(ctx),
                    )
                    .build(ctx);
                    expand_all
                }),
        )
        .add_row(Row::strict(22.0))
        .add_column(Column::stretch())
        .add_column(Column::strict(22.0))
        .add_column(Column::strict(22.0))
        .add_column(Column::strict(22.0))
        .build(ctx);

        Self {
            panel,
            search_text,
            clear_search_text,
            collapse_all,
            expand_all,
        }
    }
}

pub struct TrackList {
    toolbar: Toolbar,
    pub panel: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    add_track: Handle<UiNode>,
    node_selector: Handle<UiNode>,
    property_selector: Handle<UiNode>,
    selected_node: Handle<Node>,
    group_views: FxHashMap<Handle<Node>, Handle<UiNode>>,
    track_views: FxHashMap<Uuid, Handle<UiNode>>,
    curve_views: FxHashMap<Uuid, Handle<UiNode>>,
    context_menu: TrackContextMenu,
}

struct CurveViewData {
    id: Uuid,
}

macro_rules! define_allowed_types {
    ($($ty:ty),*) => {
        [
            $(
                TypeId::of::<InheritableVariable<$ty>>(),
                TypeId::of::<$ty>(),
            )*
        ]
    }
}

impl TrackList {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let toolbar = Toolbar::new(ctx);

        let tree_root;
        let add_track;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.panel)
                .with_child(
                    ScrollViewerBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_content({
                        tree_root = TreeRootBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                        )
                        .build(ctx);
                        tree_root
                    })
                    .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                add_track = ButtonBuilder::new(WidgetBuilder::new())
                                    .with_text("Add Track..")
                                    .build(ctx);
                                add_track
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::strict(22.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            toolbar,
            context_menu: TrackContextMenu::new(ctx),
            panel,
            tree_root,
            add_track,
            node_selector: Default::default(),
            property_selector: Default::default(),
            selected_node: Default::default(),
            group_views: Default::default(),
            track_views: Default::default(),
            curve_views: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &Sender<Message>,
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
        ui: &mut UserInterface,
        scene: &Scene,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_track {
                self.node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a Node To Animate")),
                )
                .with_hierarchy(HierarchyNode::from_scene_node(
                    scene.graph.get_root(),
                    editor_scene.editor_objects_root,
                    &scene.graph,
                ))
                .build(&mut ui.build_ctx());

                ui.send_message(WindowMessage::open_modal(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    true,
                ));
            } else if message.destination() == self.toolbar.expand_all {
                ui.send_message(TreeRootMessage::expand_all(
                    self.tree_root,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.toolbar.collapse_all {
                ui.send_message(TreeRootMessage::collapse_all(
                    self.tree_root,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.toolbar.clear_search_text {
                ui.send_message(TextMessage::text(
                    self.toolbar.search_text,
                    MessageDirection::ToWidget,
                    Default::default(),
                ));
            }
        } else if let Some(TextMessage::Text(text)) = message.data() {
            if message.destination() == self.toolbar.search_text
                && message.direction() == MessageDirection::FromWidget
            {
                let filter_text = text.to_lowercase();
                utils::apply_visibility_filter(self.tree_root, ui, |node| {
                    if let Some(tree) = node.query_component::<Tree>() {
                        if let Some(tree_text) = ui.node(tree.content).query_component::<Text>() {
                            return Some(tree_text.text().to_lowercase().contains(&filter_text));
                        }
                    }

                    None
                });
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.node_selector
                || message.destination() == self.property_selector
            {
                ui.send_message(WidgetMessage::remove(
                    message.destination(),
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(NodeSelectorMessage::Selection(selection)) = message.data() {
            if message.destination() == self.node_selector {
                if let Some(first) = selection.first() {
                    self.selected_node = *first;

                    self.property_selector = PropertySelectorWindowBuilder::new(
                                WindowBuilder::new(
                                    WidgetBuilder::new().with_width(300.0).with_height(400.0),
                                )
                                    .with_title(WindowTitle::text("Select a Numeric Property To Animate"))
                                    .open(false),
                            )
                                .with_allowed_types(Some(FxHashSet::from_iter(define_allowed_types! {
                            f32, f64, u64, i64, u32, i32, u16, i16, u8, i8, bool,

                            Vector2<f32>, Vector2<f64>, Vector2<u64>, Vector2<i64>, Vector2<u32>, Vector2<i32>,
                            Vector2<i16>, Vector2<u16>, Vector2<i8>, Vector2<u8>,

                            Vector3<f32>, Vector3<f64>, Vector3<u64>, Vector3<i64>, Vector3<u32>, Vector3<i32>,
                            Vector3<i16>, Vector3<u16>, Vector3<i8>, Vector3<u8>,

                            Vector4<f32>, Vector4<f64>, Vector4<u64>, Vector4<i64>, Vector4<u32>, Vector4<i32>,
                            Vector4<i16>, Vector4<u16>, Vector4<i8>, Vector4<u8>,

                            UnitQuaternion<f32>
                        })))
                                .with_property_descriptors(object_to_property_tree(
                                    "",
                                    scene.graph[*first].as_reflect(),
                                ))
                                .build(&mut ui.build_ctx());

                    ui.send_message(WindowMessage::open_modal(
                        self.property_selector,
                        MessageDirection::ToWidget,
                        true,
                    ));
                }
            }
        } else if let Some(PropertySelectorMessage::Selection(selected_properties)) = message.data()
        {
            if message.destination() == self.property_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(node) = scene.graph.try_get(self.selected_node) {
                    for property_path in selected_properties {
                        match node.as_reflect().resolve_path(&property_path.path) {
                            Ok(property) => {
                                let property_type = property.as_any().type_id();

                                let types = if property_type == TypeId::of::<f32>() {
                                    Some((TrackValueKind::Real, ValueType::F32))
                                } else if property_type == TypeId::of::<f64>() {
                                    Some((TrackValueKind::Real, ValueType::F64))
                                } else if property_type == TypeId::of::<u64>() {
                                    Some((TrackValueKind::Real, ValueType::U64))
                                } else if property_type == TypeId::of::<i64>() {
                                    Some((TrackValueKind::Real, ValueType::I64))
                                } else if property_type == TypeId::of::<u32>() {
                                    Some((TrackValueKind::Real, ValueType::U32))
                                } else if property_type == TypeId::of::<i32>() {
                                    Some((TrackValueKind::Real, ValueType::I32))
                                } else if property_type == TypeId::of::<u16>() {
                                    Some((TrackValueKind::Real, ValueType::U16))
                                } else if property_type == TypeId::of::<i16>() {
                                    Some((TrackValueKind::Real, ValueType::I16))
                                } else if property_type == TypeId::of::<u8>() {
                                    Some((TrackValueKind::Real, ValueType::U8))
                                } else if property_type == TypeId::of::<i8>() {
                                    Some((TrackValueKind::Real, ValueType::I8))
                                } else if property_type == TypeId::of::<bool>() {
                                    Some((TrackValueKind::Real, ValueType::Bool))
                                } else if property_type == TypeId::of::<Vector2<f32>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2F32))
                                } else if property_type == TypeId::of::<Vector2<f64>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2F64))
                                } else if property_type == TypeId::of::<Vector2<u64>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2U64))
                                } else if property_type == TypeId::of::<Vector2<i64>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2I64))
                                } else if property_type == TypeId::of::<Vector2<u32>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2U32))
                                } else if property_type == TypeId::of::<Vector2<i32>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2I32))
                                } else if property_type == TypeId::of::<Vector2<u16>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2U16))
                                } else if property_type == TypeId::of::<Vector2<i16>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2I16))
                                } else if property_type == TypeId::of::<Vector2<u8>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2U8))
                                } else if property_type == TypeId::of::<Vector2<i8>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2I8))
                                } else if property_type == TypeId::of::<Vector2<bool>>() {
                                    Some((TrackValueKind::Vector2, ValueType::Vector2Bool))
                                } else if property_type == TypeId::of::<Vector3<f32>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3F32))
                                } else if property_type == TypeId::of::<Vector3<f64>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3F64))
                                } else if property_type == TypeId::of::<Vector3<u64>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3U64))
                                } else if property_type == TypeId::of::<Vector3<i64>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3I64))
                                } else if property_type == TypeId::of::<Vector3<u32>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3U32))
                                } else if property_type == TypeId::of::<Vector3<i32>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3I32))
                                } else if property_type == TypeId::of::<Vector3<u16>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3U16))
                                } else if property_type == TypeId::of::<Vector3<i16>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3I16))
                                } else if property_type == TypeId::of::<Vector3<u8>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3U8))
                                } else if property_type == TypeId::of::<Vector3<i8>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3I8))
                                } else if property_type == TypeId::of::<Vector3<bool>>() {
                                    Some((TrackValueKind::Vector3, ValueType::Vector3Bool))
                                } else if property_type == TypeId::of::<Vector4<f32>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4F32))
                                } else if property_type == TypeId::of::<Vector4<f64>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4F64))
                                } else if property_type == TypeId::of::<Vector4<u64>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4U64))
                                } else if property_type == TypeId::of::<Vector4<i64>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4I64))
                                } else if property_type == TypeId::of::<Vector4<u32>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4U32))
                                } else if property_type == TypeId::of::<Vector4<i32>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4I32))
                                } else if property_type == TypeId::of::<Vector4<u16>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4U16))
                                } else if property_type == TypeId::of::<Vector4<i16>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4I16))
                                } else if property_type == TypeId::of::<Vector4<u8>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4U8))
                                } else if property_type == TypeId::of::<Vector4<i8>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4I8))
                                } else if property_type == TypeId::of::<Vector4<bool>>() {
                                    Some((TrackValueKind::Vector4, ValueType::Vector4Bool))
                                } else if property_type == TypeId::of::<UnitQuaternion<f32>>() {
                                    Some((
                                        TrackValueKind::UnitQuaternion,
                                        ValueType::UnitQuaternionF32,
                                    ))
                                } else if property_type == TypeId::of::<UnitQuaternion<f64>>() {
                                    Some((
                                        TrackValueKind::UnitQuaternion,
                                        ValueType::UnitQuaternionF64,
                                    ))
                                } else {
                                    None
                                };

                                if let Some((track_value_kind, actual_value_type)) = types {
                                    let mut track = Track::new(
                                        TrackDataContainer::new(track_value_kind),
                                        ValueBinding::Property {
                                            name: property_path.path.clone(),
                                            value_type: actual_value_type,
                                        },
                                    );

                                    track.set_target(self.selected_node);

                                    sender
                                        .send(Message::do_scene_command(AddTrackCommand::new(
                                            animation_player,
                                            animation,
                                            track,
                                        )))
                                        .unwrap();
                                }
                            }
                            Err(e) => {
                                Log::err(format!(
                                    "Invalid property path {:?}. Error: {:?}!",
                                    property_path, e
                                ));
                            }
                        }
                    }
                } else {
                    Log::err("Invalid node handle!");
                }
            }
        } else if let Some(TreeRootMessage::Selected(selection)) = message.data() {
            if message.destination() == self.tree_root
                && message.direction == MessageDirection::FromWidget
            {
                let selection = Selection::Animation(AnimationSelection {
                    animation_player,
                    animation,
                    entities: selection
                        .iter()
                        .filter_map(|s| {
                            let selected_widget = ui.node(*s);
                            if let Some(track_data) = selected_widget.query_component::<TrackView>()
                            {
                                Some(SelectedEntity::Track(track_data.id))
                            } else if let Some(curve_data) =
                                selected_widget.user_data_ref::<CurveViewData>()
                            {
                                Some(SelectedEntity::Curve(curve_data.id))
                            } else {
                                None
                            }
                        })
                        .collect(),
                });

                sender
                    .send(Message::do_scene_command(ChangeSelectionCommand::new(
                        selection,
                        editor_scene.selection.clone(),
                    )))
                    .unwrap();
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.context_menu.remove_track {
                if let Selection::Animation(ref selection) = editor_scene.selection {
                    if let Some(animation_player) = scene
                        .graph
                        .try_get(selection.animation_player)
                        .and_then(|n| n.query_component_ref::<AnimationPlayer>())
                    {
                        if let Some(animation) =
                            animation_player.animations().try_get(selection.animation)
                        {
                            let mut commands =
                                vec![SceneCommand::new(ChangeSelectionCommand::new(
                                    Selection::Animation(AnimationSelection {
                                        animation_player: selection.animation_player,
                                        animation: selection.animation,
                                        // Just reset inner selection.
                                        entities: vec![],
                                    }),
                                    editor_scene.selection.clone(),
                                ))];

                            for entity in selection.entities.iter() {
                                if let SelectedEntity::Track(id) = entity {
                                    let index = animation
                                        .tracks()
                                        .iter()
                                        .position(|t| t.id() == *id)
                                        .unwrap();

                                    commands.push(SceneCommand::new(RemoveTrackCommand::new(
                                        selection.animation_player,
                                        selection.animation,
                                        index,
                                    )));
                                }
                            }

                            sender
                                .send(Message::do_scene_command(CommandGroup::from(commands)))
                                .unwrap();
                        }
                    }
                }
            }
        } else if let Some(TrackViewMessage::TrackEnabled(enabled)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if let Selection::Animation(ref selection) = editor_scene.selection {
                    if let Some(animation_player) = scene
                        .graph
                        .try_get(selection.animation_player)
                        .and_then(|n| n.query_component_ref::<AnimationPlayer>())
                    {
                        if let Some(animation) =
                            animation_player.animations().try_get(selection.animation)
                        {
                            if let Some(track_view_ref) = ui
                                .node(message.destination())
                                .query_component::<TrackView>()
                            {
                                if animation
                                    .tracks()
                                    .iter()
                                    .any(|t| t.id() == track_view_ref.id)
                                {
                                    sender
                                        .send(Message::do_scene_command(SetTrackEnabledCommand {
                                            animation_player_handle: selection.animation_player,
                                            animation_handle: selection.animation,
                                            track: track_view_ref.id,
                                            enabled: *enabled,
                                        }))
                                        .unwrap()
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        ui.send_message(TreeRootMessage::items(
            self.tree_root,
            MessageDirection::ToWidget,
            vec![],
        ));
        self.group_views.clear();
        self.track_views.clear();
        self.selected_node = Handle::NONE;
    }

    pub fn sync_to_model(
        &mut self,
        animation: &Animation,
        graph: &Graph,
        editor_scene: &EditorScene,
        ui: &mut UserInterface,
    ) {
        match animation.tracks().len().cmp(&self.track_views.len()) {
            Ordering::Less => {
                for track_view in self.track_views.clone().values() {
                    let track_view_ref = ui.node(*track_view);
                    let track_view_data = track_view_ref.query_component::<TrackView>().unwrap();
                    if animation
                        .tracks()
                        .iter()
                        .all(|t| t.id() != track_view_data.id)
                    {
                        for curve_item in track_view_ref
                            .query_component::<Tree>()
                            .unwrap()
                            .items
                            .iter()
                            .cloned()
                        {
                            let curve_item_ref = ui
                                .node(curve_item)
                                .user_data_ref::<CurveViewData>()
                                .unwrap();
                            assert!(self.curve_views.remove(&curve_item_ref.id).is_some());
                        }

                        send_sync_message(
                            ui,
                            TreeRootMessage::remove_item(
                                self.tree_root,
                                MessageDirection::ToWidget,
                                *track_view,
                            ),
                        );

                        assert!(self.track_views.remove(&track_view_data.id).is_some());

                        // Remove group if it is empty.
                        if let Some(group) = self.group_views.get(&track_view_data.target) {
                            if ui
                                .node(*group)
                                .query_component::<Tree>()
                                .unwrap()
                                .items
                                .len()
                                <= 1
                            {
                                send_sync_message(
                                    ui,
                                    TreeRootMessage::remove_item(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        *group,
                                    ),
                                );

                                assert!(self.group_views.remove(&track_view_data.target).is_some());
                            }
                        }
                    }
                }
            }
            Ordering::Equal => {
                // Nothing to do.
            }
            Ordering::Greater => {
                for model_track in animation.tracks().iter() {
                    if self
                        .track_views
                        .values()
                        .map(|v| ui.node(*v))
                        .all(|v| v.query_component::<TrackView>().unwrap().id != model_track.id())
                    {
                        let parent_group = match self.group_views.entry(model_track.target()) {
                            Entry::Occupied(entry) => *entry.get(),
                            Entry::Vacant(entry) => {
                                let ctx = &mut ui.build_ctx();
                                let group = TreeBuilder::new(WidgetBuilder::new())
                                    .with_content(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text(format!(
                                                "{} ({}:{})",
                                                graph
                                                    .try_get(model_track.target())
                                                    .map(|n| n.name())
                                                    .unwrap_or_default(),
                                                model_track.target().index(),
                                                model_track.target().generation()
                                            ))
                                            .build(ctx),
                                    )
                                    .build(ctx);
                                send_sync_message(
                                    ui,
                                    TreeRootMessage::add_item(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        group,
                                    ),
                                );

                                *entry.insert(group)
                            }
                        };

                        let ctx = &mut ui.build_ctx();

                        let curves = model_track
                            .data_container()
                            .curves_ref()
                            .iter()
                            .enumerate()
                            .map(|(i, curve)| {
                                let curve_view = TreeBuilder::new(
                                    WidgetBuilder::new()
                                        .with_user_data(Rc::new(CurveViewData { id: curve.id() })),
                                )
                                .with_content(
                                    TextBuilder::new(WidgetBuilder::new())
                                        .with_text(format!(
                                            "Curve - {}",
                                            ["X", "Y", "Z", "W"].get(i).unwrap_or(&"_"),
                                        ))
                                        .build(ctx),
                                )
                                .build(ctx);

                                self.curve_views.insert(curve.id(), curve_view);

                                curve_view
                            })
                            .collect();

                        let track_view = TrackViewBuilder::new(
                            TreeBuilder::new(
                                WidgetBuilder::new().with_context_menu(self.context_menu.menu),
                            )
                            .with_items(curves),
                        )
                        .with_track_enabled(model_track.is_enabled())
                        .with_id(model_track.id())
                        .with_target(model_track.target())
                        .with_name(format!("{}", model_track.binding()))
                        .build(ctx);

                        send_sync_message(
                            ui,
                            TreeMessage::add_item(
                                parent_group,
                                MessageDirection::ToWidget,
                                track_view,
                            ),
                        );

                        assert!(self
                            .track_views
                            .insert(model_track.id(), track_view)
                            .is_none());
                    }
                }
            }
        }

        if let Selection::Animation(ref selection) = editor_scene.selection {
            let mut any_track_selected = false;
            let tree_selection = selection
                .entities
                .iter()
                .filter_map(|e| match e {
                    SelectedEntity::Track(id) => {
                        any_track_selected = true;
                        self.track_views.get(id).cloned()
                    }
                    SelectedEntity::Curve(id) => self.curve_views.get(id).cloned(),
                    SelectedEntity::Signal(_) => None,
                })
                .collect();

            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, tree_selection),
            );

            send_sync_message(
                ui,
                WidgetMessage::enabled(
                    self.context_menu.remove_track,
                    MessageDirection::ToWidget,
                    any_track_selected,
                ),
            );
        }

        for track_model in animation.tracks() {
            if let Some(track_view) = self.track_views.get(&track_model.id()) {
                let track_view_ref = ui.node(*track_view).query_component::<TrackView>().unwrap();
                if track_view_ref.track_enabled != track_model.is_enabled() {
                    send_sync_message(
                        ui,
                        TrackViewMessage::track_enabled(
                            *track_view,
                            MessageDirection::ToWidget,
                            track_model.is_enabled(),
                        ),
                    );
                }
            }
        }
    }
}
