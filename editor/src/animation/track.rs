#![allow(clippy::manual_map)]

use crate::{
    animation::{
        command::{
            AddTrackCommand, RemoveTrackCommand, SetTrackEnabledCommand, SetTrackTargetCommand,
        },
        selection::{AnimationSelection, SelectedEntity},
    },
    gui::make_image_button_with_tooltip,
    load_image,
    menu::create_menu_item,
    message::MessageSender,
    scene::{
        commands::{ChangeSelectionCommand, CommandGroup, SceneCommand},
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene, Selection,
    },
    send_sync_message, utils,
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
        color::Color,
        log::Log,
        pool::Handle,
        reflect::{Reflect, ResolvePath},
        uuid::Uuid,
        variable::InheritableVariable,
    },
    fxhash::{FxHashMap, FxHashSet},
    gui::{
        brush::Brush,
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
        BuildContext, Control, NodeHandleMapping, Orientation, RcUiNodeHandle, Thickness, UiNode,
        UserInterface, VerticalAlignment, BRUSH_BRIGHT, BRUSH_TEXT,
    },
    scene::{animation::AnimationPlayer, graph::Graph, node::Node, Scene},
};
use std::{
    any::{Any, TypeId},
    cmp::Ordering,
    collections::hash_map::Entry,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::mpsc::Sender,
};

#[derive(PartialEq, Eq)]
enum PropertyBindingMode {
    Generic,
    Position,
    Rotation,
    Scale,
}

struct TrackContextMenu {
    menu: RcUiNodeHandle,
    remove_track: Handle<UiNode>,
    set_target: Handle<UiNode>,
    target_node_selector: Handle<UiNode>,
    duplicate: Handle<UiNode>,
}

impl TrackContextMenu {
    fn new(ctx: &mut BuildContext) -> Self {
        let remove_track;
        let set_target;
        let duplicate;
        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            remove_track = create_menu_item("Remove Selected Tracks", vec![], ctx);
                            remove_track
                        })
                        .with_child({
                            set_target = create_menu_item("Set Target...", vec![], ctx);
                            set_target
                        })
                        .with_child({
                            duplicate = create_menu_item("Duplicate", vec![], ctx);
                            duplicate
                        }),
                )
                .build(ctx),
            )
            .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove_track,
            set_target,
            target_node_selector: Default::default(),
            duplicate,
        }
    }
}

fn type_id_to_supported_type(property_type: TypeId) -> Option<(TrackValueKind, ValueType)> {
    if property_type == TypeId::of::<f32>() {
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
        Some((TrackValueKind::UnitQuaternion, ValueType::UnitQuaternionF32))
    } else if property_type == TypeId::of::<UnitQuaternion<f64>>() {
        Some((TrackValueKind::UnitQuaternion, ValueType::UnitQuaternionF64))
    } else {
        None
    }
}

#[allow(clippy::enum_variant_names)] // GTFO
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackViewMessage {
    TrackEnabled(bool),
    TrackName(String),
    TrackTargetIsValid(Result<(), String>),
}

impl TrackViewMessage {
    define_constructor!(Self:TrackEnabled => fn track_enabled(bool), layout: false);
    define_constructor!(Self:TrackName => fn track_name(String), layout: false);
    define_constructor!(Self:TrackTargetIsValid => fn track_target_is_valid(Result<(), String>), layout: false);
}

#[derive(Clone)]
struct TrackView {
    tree: Tree,
    id: Uuid,
    target: Handle<Node>,
    track_enabled_switch: Handle<UiNode>,
    track_enabled: bool,
    name_text: Handle<UiNode>,
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
        } else if let Some(msg) = message.data::<TrackViewMessage>() {
            if message.destination() == self.handle
                && message.direction() == MessageDirection::ToWidget
            {
                match msg {
                    TrackViewMessage::TrackEnabled(enabled) => {
                        if self.track_enabled != *enabled {
                            self.track_enabled = *enabled;

                            ui.send_message(CheckBoxMessage::checked(
                                self.track_enabled_switch,
                                MessageDirection::ToWidget,
                                Some(*enabled),
                            ));

                            ui.send_message(message.reverse());
                        }
                    }
                    TrackViewMessage::TrackName(name) => {
                        ui.send_message(TextMessage::text(
                            self.name_text,
                            MessageDirection::ToWidget,
                            name.clone(),
                        ));
                    }
                    TrackViewMessage::TrackTargetIsValid(result) => {
                        ui.send_message(WidgetMessage::foreground(
                            self.name_text,
                            MessageDirection::ToWidget,
                            if result.is_ok() {
                                BRUSH_TEXT
                            } else {
                                Brush::Solid(Color::RED)
                            },
                        ));

                        match result {
                            Ok(_) => {
                                ui.send_message(WidgetMessage::tooltip(
                                    self.name_text,
                                    MessageDirection::ToWidget,
                                    None,
                                ));
                            }
                            Err(reason) => {
                                let tooltip =
                                    make_simple_tooltip(&mut ui.build_ctx(), reason.as_str());

                                ui.send_message(WidgetMessage::tooltip(
                                    self.name_text,
                                    MessageDirection::ToWidget,
                                    Some(tooltip),
                                ));
                            }
                        }
                    }
                }
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
        let name_text;
        let track_enabled_switch = CheckBoxBuilder::new(WidgetBuilder::new().with_height(18.0))
            .with_content({
                name_text =
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                        .with_text(self.name)
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .build(ctx);
                name_text
            })
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
            name_text,
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
    add_position_track: Handle<UiNode>,
    add_rotation_track: Handle<UiNode>,
    add_scale_track: Handle<UiNode>,
    node_selector: Handle<UiNode>,
    property_selector: Handle<UiNode>,
    selected_node: Handle<Node>,
    group_views: FxHashMap<Handle<Node>, Handle<UiNode>>,
    track_views: FxHashMap<Uuid, Handle<UiNode>>,
    curve_views: FxHashMap<Uuid, Handle<UiNode>>,
    context_menu: TrackContextMenu,
    property_binding_mode: PropertyBindingMode,
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
        let add_position_track;
        let add_rotation_track;
        let add_scale_track;

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
                                add_track = make_image_button_with_tooltip(
                                    ctx,
                                    22.0,
                                    22.0,
                                    load_image(include_bytes!(
                                        "../../resources/embed/property_track.png"
                                    )),
                                    "Add Property Track.\n\
                                    Create generic property binding to a numeric property.",
                                );
                                add_track
                            })
                            .with_child({
                                add_position_track = make_image_button_with_tooltip(
                                    ctx,
                                    22.0,
                                    22.0,
                                    load_image(include_bytes!(
                                        "../../resources/embed/position_track.png"
                                    )),
                                    "Add Position Track.\n\
                                    Creates a binding to a local position of a node. \
                                    Such binding is much more performant than generic \
                                    property binding",
                                );
                                add_position_track
                            })
                            .with_child({
                                add_scale_track = make_image_button_with_tooltip(
                                    ctx,
                                    22.0,
                                    22.0,
                                    load_image(include_bytes!(
                                        "../../resources/embed/scaling_track.png"
                                    )),
                                    "Add Scale Track.\n\
                                    Creates a binding to a local scale of a node. \
                                    Such binding is much more performant than generic \
                                    property binding",
                                );
                                add_scale_track
                            })
                            .with_child({
                                add_rotation_track = make_image_button_with_tooltip(
                                    ctx,
                                    22.0,
                                    22.0,
                                    load_image(include_bytes!(
                                        "../../resources/embed/rotation_track.png"
                                    )),
                                    "Add Rotation Track.\n\
                                    Creates a binding to a local rotation of a node. \
                                    Such binding is much more performant than generic \
                                    property binding",
                                );
                                add_rotation_track
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::strict(28.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            toolbar,
            context_menu: TrackContextMenu::new(ctx),
            panel,
            tree_root,
            add_track,
            add_position_track,
            add_rotation_track,
            add_scale_track,
            node_selector: Default::default(),
            property_selector: Default::default(),
            selected_node: Default::default(),
            group_views: Default::default(),
            track_views: Default::default(),
            curve_views: Default::default(),
            property_binding_mode: PropertyBindingMode::Generic,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        sender: &MessageSender,
        animation_player: Handle<Node>,
        animation: Handle<Animation>,
        ui: &mut UserInterface,
        scene: &Scene,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_track
                || message.destination() == self.add_position_track
                || message.destination() == self.add_scale_track
                || message.destination() == self.add_rotation_track
            {
                self.node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a Node To Animate")),
                )
                .with_hierarchy(HierarchyNode::from_scene_node(
                    editor_scene.scene_content_root,
                    editor_scene.editor_objects_root,
                    &scene.graph,
                ))
                .build(&mut ui.build_ctx());

                ui.send_message(WindowMessage::open_modal(
                    self.node_selector,
                    MessageDirection::ToWidget,
                    true,
                ));

                if message.destination() == self.add_track {
                    self.property_binding_mode = PropertyBindingMode::Generic;
                } else if message.destination() == self.add_position_track {
                    self.property_binding_mode = PropertyBindingMode::Position;
                } else if message.destination() == self.add_scale_track {
                    self.property_binding_mode = PropertyBindingMode::Scale;
                } else if message.destination() == self.add_rotation_track {
                    self.property_binding_mode = PropertyBindingMode::Rotation;
                }
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
                || message.destination() == self.context_menu.target_node_selector
            {
                ui.send_message(WidgetMessage::remove(
                    message.destination(),
                    MessageDirection::ToWidget,
                ));
            }
        } else if let Some(NodeSelectorMessage::Selection(node_selection)) = message.data() {
            if message.destination() == self.node_selector {
                if let Some(first) = node_selection.first() {
                    self.selected_node = *first;

                    match self.property_binding_mode {
                        PropertyBindingMode::Generic => {
                            let mut descriptors = vec![];
                            scene.graph[*first].as_reflect(&mut |node| {
                                descriptors = object_to_property_tree("", node);
                            });

                            self.property_selector = PropertySelectorWindowBuilder::new(
                                WindowBuilder::new(
                                    WidgetBuilder::new().with_width(300.0).with_height(400.0),
                                )
                                .with_title(WindowTitle::text(
                                    "Select a Numeric Property To Animate",
                                ))
                                .open(false),
                            )
                            .with_allowed_types(Some(FxHashSet::from_iter(define_allowed_types! {
                                f32, f64, u64, i64, u32, i32, u16, i16, u8, i8, bool,

                                Vector2<f32>, Vector2<f64>, Vector2<u64>, Vector2<i64>,
                                Vector2<u32>, Vector2<i32>,
                                Vector2<i16>, Vector2<u16>, Vector2<i8>, Vector2<u8>,

                                Vector3<f32>, Vector3<f64>, Vector3<u64>, Vector3<i64>,
                                Vector3<u32>, Vector3<i32>,
                                Vector3<i16>, Vector3<u16>, Vector3<i8>, Vector3<u8>,

                                Vector4<f32>, Vector4<f64>, Vector4<u64>, Vector4<i64>,
                                Vector4<u32>, Vector4<i32>,
                                Vector4<i16>, Vector4<u16>, Vector4<i8>, Vector4<u8>,

                                UnitQuaternion<f32>
                            })))
                            .with_property_descriptors(descriptors)
                            .build(&mut ui.build_ctx());

                            ui.send_message(WindowMessage::open_modal(
                                self.property_selector,
                                MessageDirection::ToWidget,
                                true,
                            ));
                        }
                        PropertyBindingMode::Position => {
                            sender.do_scene_command(AddTrackCommand::new(
                                animation_player,
                                animation,
                                Track::new_position().with_target(self.selected_node),
                            ));
                        }
                        PropertyBindingMode::Rotation => {
                            sender.do_scene_command(AddTrackCommand::new(
                                animation_player,
                                animation,
                                Track::new_rotation().with_target(self.selected_node),
                            ));
                        }
                        PropertyBindingMode::Scale => {
                            sender.do_scene_command(AddTrackCommand::new(
                                animation_player,
                                animation,
                                Track::new_scale().with_target(self.selected_node),
                            ));
                        }
                    }
                }
            } else if message.destination() == self.context_menu.target_node_selector {
                if let Selection::Animation(ref scene_selection) = editor_scene.selection {
                    if let Some(first) = node_selection.first() {
                        let mut commands = vec![];

                        for entity in scene_selection.entities.iter() {
                            if let SelectedEntity::Track(id) = entity {
                                commands.push(SceneCommand::new(SetTrackTargetCommand {
                                    animation_player_handle: scene_selection.animation_player,
                                    animation_handle: scene_selection.animation,
                                    track: *id,
                                    target: *first,
                                }));
                            }
                        }

                        sender.do_scene_command(CommandGroup::from(commands));
                    }
                }
            }
        } else if let Some(PropertySelectorMessage::Selection(selected_properties)) = message.data()
        {
            if message.destination() == self.property_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(node) = scene.graph.try_get(self.selected_node) {
                    for property_path in selected_properties {
                        node.resolve_path(&property_path.path, &mut |result| match result {
                            Ok(property) => {
                                let mut property_type = TypeId::of::<u32>();
                                property.as_any(&mut |any| property_type = any.type_id());

                                let types = type_id_to_supported_type(property_type);

                                if let Some((track_value_kind, actual_value_type)) = types {
                                    let mut track = Track::new(
                                        TrackDataContainer::new(track_value_kind),
                                        ValueBinding::Property {
                                            name: property_path.path.clone(),
                                            value_type: actual_value_type,
                                        },
                                    );

                                    track.set_target(self.selected_node);

                                    sender.do_scene_command(AddTrackCommand::new(
                                        animation_player,
                                        animation,
                                        track,
                                    ));
                                }
                            }
                            Err(e) => {
                                Log::err(format!(
                                    "Invalid property path {:?}. Error: {:?}!",
                                    property_path, e
                                ));
                            }
                        })
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

                sender.do_scene_command(ChangeSelectionCommand::new(
                    selection,
                    editor_scene.selection.clone(),
                ));
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

                            sender.do_scene_command(CommandGroup::from(commands));
                        }
                    }
                }
            } else if message.destination() == self.context_menu.set_target {
                self.context_menu.target_node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a New Target Node")),
                )
                .with_hierarchy(HierarchyNode::from_scene_node(
                    editor_scene.scene_content_root,
                    editor_scene.editor_objects_root,
                    &scene.graph,
                ))
                .build(&mut ui.build_ctx());

                ui.send_message(WindowMessage::open_modal(
                    self.context_menu.target_node_selector,
                    MessageDirection::ToWidget,
                    true,
                ));
            } else if message.destination() == self.context_menu.duplicate {
                if let Selection::Animation(ref selection) = editor_scene.selection {
                    if let Some(animation_player) = scene
                        .graph
                        .try_get(selection.animation_player)
                        .and_then(|n| n.query_component_ref::<AnimationPlayer>())
                    {
                        if let Some(animation) =
                            animation_player.animations().try_get(selection.animation)
                        {
                            let commands = selection
                                .entities
                                .iter()
                                .filter_map(|e| match e {
                                    SelectedEntity::Track(track_id) => {
                                        let index = animation
                                            .tracks()
                                            .iter()
                                            .position(|t| t.id() == *track_id)
                                            .unwrap();

                                        let mut track = animation.tracks()[index].clone();

                                        track.set_id(Uuid::new_v4());

                                        Some(SceneCommand::new(AddTrackCommand::new(
                                            selection.animation_player,
                                            selection.animation,
                                            track,
                                        )))
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>();

                            sender.do_scene_command(CommandGroup::from(commands));
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
                                    sender.do_scene_command(SetTrackEnabledCommand {
                                        animation_player_handle: selection.animation_player,
                                        animation_handle: selection.animation,
                                        track: track_view_ref.id,
                                        enabled: *enabled,
                                    })
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
                                WidgetBuilder::new()
                                    .with_context_menu(self.context_menu.menu.clone()),
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
            send_sync_message(
                ui,
                WidgetMessage::enabled(
                    self.context_menu.set_target,
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

                let mut validation_result = Ok(());
                if let Some(target) = graph.try_get(track_model.target()) {
                    if let Some(parent_group) = self.group_views.get(&track_model.target()) {
                        send_sync_message(
                            ui,
                            TextMessage::text(
                                ui.node(*parent_group)
                                    .query_component::<Tree>()
                                    .unwrap()
                                    .content,
                                MessageDirection::ToWidget,
                                target.name_owned(),
                            ),
                        );
                    }

                    send_sync_message(
                        ui,
                        TrackViewMessage::track_name(
                            *track_view,
                            MessageDirection::ToWidget,
                            format!("{}", track_model.binding()),
                        ),
                    );

                    if let ValueBinding::Property { name, value_type } = track_model.binding() {
                        target.resolve_path(name, &mut |result| match result {
                            Ok(value) => {
                                let mut property_type = TypeId::of::<u32>();
                                value.as_any(&mut |any| property_type = any.type_id());

                                if let Some((_, type_)) = type_id_to_supported_type(property_type) {
                                    if *value_type != type_ {
                                        validation_result = Err(format!(
                                            "Property type mismatch. Expected {:?}, got {:?}",
                                            value_type, type_
                                        ));
                                    }
                                } else {
                                    validation_result = Err(format!(
                                        "Unsupported property type of {:?} type id.",
                                        property_type
                                    ));
                                }
                            }
                            Err(err) => {
                                validation_result = Err(format!(
                                    "Unable to resolve property path {}. Reason: {:?}",
                                    name, err
                                ));
                            }
                        });
                    }
                } else {
                    validation_result =
                        Err("Invalid handle. The target node does not exist!".to_owned());
                }

                send_sync_message(
                    ui,
                    TrackViewMessage::track_target_is_valid(
                        *track_view,
                        MessageDirection::ToWidget,
                        validation_result,
                    ),
                );
            }
        }
    }
}
