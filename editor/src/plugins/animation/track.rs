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

#![allow(clippy::manual_map)]

use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        asset::Resource,
        core::{
            algebra::{UnitQuaternion, Vector2, Vector3, Vector4},
            color::Color,
            log::Log,
            parking_lot::Mutex,
            pool::{ErasedHandle, Handle},
            reflect::{prelude::*, Reflect},
            type_traits::prelude::*,
            uuid_provider,
            variable::InheritableVariable,
            visitor::prelude::*,
        },
        fxhash::{FxHashMap, FxHashSet},
        generic_animation::{
            container::{TrackDataContainer, TrackValueKind},
            track::{Track, TrackBinding},
            value::{ValueBinding, ValueType},
            Animation,
        },
        graph::{SceneGraph, SceneGraphNode},
        graphics::DrawParameters,
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::{Button, ButtonMessage},
            check_box::{CheckBox, CheckBoxBuilder, CheckBoxMessage},
            draw::DrawingContext,
            grid::{Column, Grid, GridBuilder, Row},
            menu::{ContextMenuBuilder, MenuItem, MenuItemMessage},
            message::{MessageData, MessageDirection, OsEvent, UiMessage},
            popup::PopupBuilder,
            scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBar, SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            style::{resource::StyleResourceExt, Style},
            text::{Text, TextBuilder, TextMessage},
            text_box::EmptyTextPlaceholder,
            tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootBuilder, TreeRootMessage},
            utils::make_image_button_with_tooltip,
            utils::make_simple_tooltip,
            widget::{Widget, WidgetBuilder, WidgetMessage},
            window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, Control, Orientation, RcUiNodeHandle, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        resource::{model::ModelResource, texture::TextureBytes},
        scene::{
            mesh::buffer::{TriangleBuffer, VertexBuffer},
            sound::Samples,
        },
    },
    load_image,
    menu::create_menu_item,
    message::MessageSender,
    plugins::animation::{
        animation_container_ref,
        command::{
            AddTrackCommand, RemoveTrackCommand, SetTrackEnabledCommand, SetTrackTargetCommand,
            SetTrackValueBindingCommand,
        },
        selection::{AnimationSelection, SelectedEntity},
    },
    scene::{
        commands::ChangeSelectionCommand,
        property::{
            object_to_property_tree, PropertyDescriptorData, PropertySelectorMessage,
            PropertySelectorWindow, PropertySelectorWindowBuilder,
        },
        selector::{
            AllowedType, HierarchyNode, NodeSelectorMessage, NodeSelectorWindow,
            NodeSelectorWindowBuilder,
        },
        Selection,
    },
    utils::{self, make_square_image_button_with_tooltip},
};
use std::{
    any::TypeId,
    cmp::Ordering,
    collections::hash_map::Entry,
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc},
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
    remove_track: Handle<MenuItem>,
    set_target: Handle<MenuItem>,
    rebind: Handle<MenuItem>,
    target_node_selector: Handle<NodeSelectorWindow>,
    property_rebinding_selector: Handle<PropertySelectorWindow>,
    duplicate: Handle<MenuItem>,
}

impl TrackContextMenu {
    pub const REMOVE_SELECTED: Uuid = uuid!("5763584b-451f-442b-a701-860b6ebe8ade");
    pub const SET_TARGET: Uuid = uuid!("18bd0b4b-4c8d-4a47-aa32-0169dfb4f766");
    pub const REBIND: Uuid = uuid!("56adb9f1-ea0f-4d1a-8f55-b09184e0b5cc");
    pub const DUPLICATE: Uuid = uuid!("17ae02cc-0139-4697-9ce9-4ed680402be4");

    fn new(ctx: &mut BuildContext) -> Self {
        let remove_track;
        let set_target;
        let rebind;
        let duplicate;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
                .with_content(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_child({
                                remove_track = create_menu_item(
                                    "Remove Selected Tracks",
                                    Self::REMOVE_SELECTED,
                                    vec![],
                                    ctx,
                                );
                                remove_track
                            })
                            .with_child({
                                set_target = create_menu_item(
                                    "Set Target...",
                                    Self::SET_TARGET,
                                    vec![],
                                    ctx,
                                );
                                set_target
                            })
                            .with_child({
                                rebind = create_menu_item("Rebind...", Self::REBIND, vec![], ctx);
                                rebind
                            })
                            .with_child({
                                duplicate =
                                    create_menu_item("Duplicate", Self::DUPLICATE, vec![], ctx);
                                duplicate
                            }),
                    )
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            remove_track,
            set_target,
            rebind,
            target_node_selector: Default::default(),
            property_rebinding_selector: Default::default(),
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
        Some((
            TrackValueKind::UnitQuaternionEuler,
            ValueType::UnitQuaternionF32,
        ))
    } else if property_type == TypeId::of::<UnitQuaternion<f64>>() {
        Some((
            TrackValueKind::UnitQuaternionEuler,
            ValueType::UnitQuaternionF64,
        ))
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
impl MessageData for TrackViewMessage {}

#[derive(Clone, Debug, Reflect, Visit, ComponentProvider)]
#[reflect(derived_type = "UiNode")]
struct TrackView {
    #[component(include)]
    pub tree: Tree,
    id: Uuid,
    target: ErasedHandle,
    track_enabled_switch: Handle<CheckBox>,
    track_enabled: bool,
    name_text: Handle<Text>,
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

uuid_provider!(TrackView = "c1e930da-d55d-492e-b87b-16c1adf03319");

impl Control for TrackView {
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

    fn update(&mut self, dt: f32, ui: &mut UserInterface) {
        self.tree.update(dt, ui)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        if let Some(CheckBoxMessage::Check(Some(value))) =
            message.data_from(self.track_enabled_switch)
        {
            if self.track_enabled != *value {
                ui.send(self.handle, TrackViewMessage::TrackEnabled(*value));
            }
        } else if let Some(msg) = message.data_for::<TrackViewMessage>(self.handle) {
            match msg {
                TrackViewMessage::TrackEnabled(enabled) => {
                    if self.track_enabled != *enabled {
                        self.track_enabled = *enabled;

                        ui.send(
                            self.track_enabled_switch,
                            CheckBoxMessage::Check(Some(*enabled)),
                        );

                        ui.send_message(message.reverse());
                    }
                }
                TrackViewMessage::TrackName(name) => {
                    ui.send(self.name_text, TextMessage::Text(name.clone()));
                }
                TrackViewMessage::TrackTargetIsValid(result) => {
                    ui.send(
                        self.name_text,
                        WidgetMessage::Foreground(if result.is_ok() {
                            ui.style.property(Style::BRUSH_TEXT)
                        } else {
                            ui.style.property(Style::BRUSH_ERROR)
                        }),
                    );

                    match result {
                        Ok(_) => {
                            ui.send(self.name_text, WidgetMessage::Tooltip(None));
                        }
                        Err(reason) => {
                            let tooltip = make_simple_tooltip(&mut ui.build_ctx(), reason.as_str());

                            ui.send(self.name_text, WidgetMessage::Tooltip(Some(tooltip)));
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
    target: ErasedHandle,
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

    pub fn with_target(mut self, target: ErasedHandle) -> Self {
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

    pub fn build(self, ctx: &mut BuildContext) -> Handle<TrackView> {
        let name_text;
        let track_enabled_switch;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_child({
                    track_enabled_switch = CheckBoxBuilder::new(
                        WidgetBuilder::new()
                            .with_height(18.0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center),
                    )
                    .checked(Some(self.track_enabled))
                    .build(ctx);
                    track_enabled_switch
                })
                .with_child({
                    name_text = TextBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .on_column(1),
                    )
                    .with_text(self.name)
                    .build(ctx);
                    name_text
                }),
        )
        .add_row(Row::auto())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .build(ctx);

        let track_view = TrackView {
            tree: self.tree_builder.with_content(grid).build_tree(ctx),
            id: self.id,
            target: self.target,
            track_enabled: self.track_enabled,
            track_enabled_switch,
            name_text,
        };

        ctx.add(track_view)
    }
}

struct Toolbar {
    panel: Handle<Grid>,
    search_bar: Handle<SearchBar>,
    collapse_all: Handle<Button>,
    expand_all: Handle<Button>,
}

impl Toolbar {
    fn new(ctx: &mut BuildContext) -> Self {
        let search_bar;
        let collapse_all;
        let expand_all;
        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    search_bar = SearchBarBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .on_column(0),
                    )
                    .with_empty_text_placeholder(EmptyTextPlaceholder::Text("Search for a track"))
                    .build(ctx);
                    search_bar
                })
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_child({
                                collapse_all = make_image_button_with_tooltip(
                                    ctx,
                                    16.0,
                                    16.0,
                                    load_image!("../../../resources/collapse.png"),
                                    "Collapse All",
                                    None,
                                );
                                collapse_all
                            })
                            .with_child({
                                expand_all = make_image_button_with_tooltip(
                                    ctx,
                                    16.0,
                                    16.0,
                                    load_image!("../../../resources/expand.png"),
                                    "Expand All",
                                    None,
                                );
                                expand_all
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .add_row(Row::strict(26.0))
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        Self {
            panel,
            search_bar,
            collapse_all,
            expand_all,
        }
    }
}

pub struct TrackList {
    toolbar: Toolbar,
    pub panel: Handle<Grid>,
    tree_root: Handle<TreeRoot>,
    add_track: Handle<Button>,
    add_position_track: Handle<Button>,
    add_rotation_track: Handle<Button>,
    add_scale_track: Handle<Button>,
    node_selector: Handle<NodeSelectorWindow>,
    property_selector: Handle<PropertySelectorWindow>,
    selected_node: ErasedHandle,
    group_views: FxHashMap<ErasedHandle, Handle<Tree>>,
    track_views: FxHashMap<Uuid, Handle<TrackView>>,
    curve_views: FxHashMap<Uuid, Handle<Tree>>,
    context_menu: TrackContextMenu,
    property_binding_mode: PropertyBindingMode,
    scroll_viewer: Handle<ScrollViewer>,
    selected_animation: ErasedHandle,
}

#[derive(Clone)]
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
        let scroll_viewer;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(toolbar.panel)
                .with_child({
                    scroll_viewer = ScrollViewerBuilder::new(
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
                    .build(ctx);
                    scroll_viewer
                })
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(2)
                            .on_column(0)
                            .with_foreground(ctx.style.property(Style::BRUSH_LIGHT))
                            .with_child(
                                StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_child({
                                            add_track = make_square_image_button_with_tooltip(
                                                ctx,
                                                load_image!(
                                                    "../../../resources/property_track.png"
                                                ),
                                                "Add Property Track.\n\
                                            Create generic property binding to a numeric property.",
                                                Some(0),
                                            );
                                            add_track
                                        })
                                        .with_child({
                                            add_position_track =
                                                make_square_image_button_with_tooltip(
                                                    ctx,
                                                    load_image!(
                                                        "../../../resources/position_track.png"
                                                    ),
                                                    "Add Position Track.\n\
                                            Creates a binding to a local position of a node. \
                                            Such binding is much more performant than generic \
                                            property binding",
                                                    Some(1),
                                                );
                                            add_position_track
                                        })
                                        .with_child({
                                            add_scale_track = make_square_image_button_with_tooltip(
                                                ctx,
                                                load_image!("../../../resources/scaling_track.png"),
                                                "Add Scale Track.\n\
                                            Creates a binding to a local scale of a node. \
                                            Such binding is much more performant than generic \
                                            property binding",
                                                Some(2),
                                            );
                                            add_scale_track
                                        })
                                        .with_child({
                                            add_rotation_track =
                                                make_square_image_button_with_tooltip(
                                                    ctx,
                                                    load_image!(
                                                        "../../../resources/rotation_track.png"
                                                    ),
                                                    "Add Rotation Track.\n\
                                            Creates a binding to a local rotation of a node. \
                                            Such binding is much more performant than generic \
                                            property binding",
                                                    Some(3),
                                                );
                                            add_rotation_track
                                        }),
                                )
                                .with_orientation(Orientation::Horizontal)
                                .build(ctx),
                            ),
                    )
                    .with_corner_radius(3.0.into())
                    .with_stroke_thickness(Thickness::uniform(1.0).into())
                    .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
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
            scroll_viewer,
            selected_animation: Default::default(),
        }
    }

    pub fn handle_ui_message<G, N>(
        &mut self,
        message: &UiMessage,
        selection: &AnimationSelection<N>,
        root: Handle<N>,
        sender: &MessageSender,
        ui: &mut UserInterface,
        graph: &G,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode<SceneGraph = G>,
    {
        let selected_animation = animation_container_ref(graph, selection.animation_player)
            .and_then(|c| c.try_get(selection.animation).ok());

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
                .with_allowed_types(
                    [AllowedType {
                        id: TypeId::of::<N>(),
                        name: std::any::type_name::<N>().to_string(),
                    }]
                    .into_iter()
                    .collect(),
                )
                .with_hierarchy(HierarchyNode::from_scene_node(root, Handle::NONE, graph))
                .build(&mut ui.build_ctx());

                ui.send(
                    self.node_selector,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: true,
                        focus_content: true,
                    },
                );

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
                ui.send(self.tree_root, TreeRootMessage::ExpandAll);
            } else if message.destination() == self.toolbar.collapse_all {
                ui.send(self.tree_root, TreeRootMessage::CollapseAll);
            }
        } else if let Some(SearchBarMessage::Text(text)) =
            message.data_from(self.toolbar.search_bar)
        {
            let filter_text = text.to_lowercase();
            utils::apply_visibility_filter(self.tree_root.to_base(), ui, |node| {
                if let Some(tree) = node.query_component::<Tree>() {
                    if let Some(tree_text) = ui.node(tree.content).query_component::<Text>() {
                        return Some(tree_text.text().to_lowercase().contains(&filter_text));
                    }
                }

                None
            });

            if filter_text.is_empty() {
                // Focus currently selected entity when clearing the filter.
                if let Some(first) = selection.entities.first() {
                    let ui_node = match first {
                        SelectedEntity::Track(id) => self
                            .track_views
                            .get(id)
                            .cloned()
                            .unwrap_or_default()
                            .to_base(),
                        SelectedEntity::Curve(id) => self
                            .curve_views
                            .get(id)
                            .cloned()
                            .unwrap_or_default()
                            .to_base(),
                        _ => Default::default(),
                    };
                    if ui_node.is_some() {
                        ui.send(
                            self.scroll_viewer,
                            ScrollViewerMessage::BringIntoView(ui_node),
                        );
                    }
                }
            }
        } else if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.node_selector
                || message.destination() == self.property_selector
                || message.destination() == self.context_menu.target_node_selector
                || message.destination() == self.context_menu.property_rebinding_selector
            {
                ui.send(message.destination(), WidgetMessage::Remove);
            }
        } else if let Some(NodeSelectorMessage::Selection(node_selection)) = message.data() {
            if message.destination() == self.node_selector {
                if let Some(first) = node_selection.first() {
                    self.selected_node = first.handle;

                    match self.property_binding_mode {
                        PropertyBindingMode::Generic => {
                            self.property_selector =
                                Self::open_property_selector(graph, first.handle.into(), None, ui);
                        }
                        PropertyBindingMode::Position => {
                            sender.do_command(AddTrackCommand::new(
                                selection.animation_player,
                                selection.animation,
                                Track::new_position(),
                                TrackBinding::new(self.selected_node.into()),
                            ));
                        }
                        PropertyBindingMode::Rotation => {
                            sender.do_command(AddTrackCommand::new(
                                selection.animation_player,
                                selection.animation,
                                Track::new_rotation(),
                                TrackBinding::new(self.selected_node.into()),
                            ));
                        }
                        PropertyBindingMode::Scale => {
                            sender.do_command(AddTrackCommand::new(
                                selection.animation_player,
                                selection.animation,
                                Track::new_scale(),
                                TrackBinding::new(self.selected_node.into()),
                            ));
                        }
                    }
                }
            } else if message.destination() == self.context_menu.target_node_selector {
                if let Some(first) = node_selection.first() {
                    let mut commands = Vec::new();

                    for entity in selection.entities.iter() {
                        if let SelectedEntity::Track(id) = entity {
                            commands.push(Command::new(SetTrackTargetCommand {
                                animation_player_handle: selection.animation_player,
                                animation_handle: selection.animation,
                                track: *id,
                                target: first.handle.into(),
                            }));
                        }
                    }

                    sender.do_command(CommandGroup::from(commands));
                }
            }
        } else if let Some(PropertySelectorMessage::Selection(selected_properties)) = message.data()
        {
            if message.is_from(self.property_selector) {
                if let Ok(node) = graph.try_get_node(self.selected_node.into()) {
                    for property_path in selected_properties {
                        node.resolve_path(&property_path.path, &mut |result| match result {
                            Ok(property) => {
                                let mut property_type = TypeId::of::<u32>();
                                property.as_any(&mut |any| property_type = any.type_id());

                                let types = type_id_to_supported_type(property_type);

                                if let Some((track_value_kind, actual_value_type)) = types {
                                    let track = Track::new(
                                        TrackDataContainer::new(track_value_kind),
                                        ValueBinding::Property {
                                            name: property_path.path.clone().into(),
                                            value_type: actual_value_type,
                                        },
                                    );

                                    sender.do_command(AddTrackCommand::new(
                                        selection.animation_player,
                                        selection.animation,
                                        track,
                                        TrackBinding::new(self.selected_node.into()),
                                    ));
                                }
                            }
                            Err(e) => {
                                Log::err(format!(
                                    "Invalid property path {property_path:?}. Error: {e:?}!"
                                ));
                            }
                        })
                    }
                } else {
                    Log::err("Invalid node handle!");
                }
            } else if message.is_from(self.context_menu.property_rebinding_selector) {
                if let Some(entry) = selected_properties.first() {
                    if let Some(animation) = selected_animation {
                        self.rebind_property(entry, graph, selection, animation, sender);
                    }
                }
            }
        } else if let Some(TreeRootMessage::Select(tree_selection)) =
            message.data_from(self.tree_root)
        {
            let new_selection = Selection::new(AnimationSelection {
                animation_player: selection.animation_player,
                animation: selection.animation,
                entities: tree_selection
                    .iter()
                    .filter_map(|s| {
                        let selected_widget = ui.node(s.to_base());
                        if let Some(track_data) = selected_widget.query_component::<TrackView>() {
                            Some(SelectedEntity::Track(track_data.id))
                        } else if let Some(curve_data) =
                            selected_widget.user_data_cloned::<CurveViewData>()
                        {
                            Some(SelectedEntity::Curve(curve_data.id))
                        } else {
                            None
                        }
                    })
                    .collect(),
            });

            sender.do_command(ChangeSelectionCommand::new(new_selection));
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.context_menu.remove_track {
                if selected_animation.is_some() {
                    let mut commands = vec![Command::new(ChangeSelectionCommand::new(
                        Selection::new(AnimationSelection {
                            animation_player: selection.animation_player,
                            animation: selection.animation,
                            // Just reset inner selection.
                            entities: vec![],
                        }),
                    ))];

                    for entity in selection.entities.iter() {
                        if let SelectedEntity::Track(id) = entity {
                            commands.push(Command::new(RemoveTrackCommand::new(
                                selection.animation_player,
                                selection.animation,
                                *id,
                            )));
                        }
                    }

                    sender.do_command(CommandGroup::from(commands));
                }
            } else if message.destination() == self.context_menu.set_target {
                self.context_menu.target_node_selector = NodeSelectorWindowBuilder::new(
                    WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                        .with_title(WindowTitle::text("Select a New Target Node")),
                )
                .with_hierarchy(HierarchyNode::from_scene_node(root, Handle::NONE, graph))
                .build(&mut ui.build_ctx());

                ui.send(
                    self.context_menu.target_node_selector,
                    WindowMessage::Open {
                        alignment: WindowAlignment::Center,
                        modal: true,
                        focus_content: true,
                    },
                );
            } else if message.destination() == self.context_menu.rebind {
                if let Some(animation) = selected_animation {
                    self.on_rebind_clicked(graph, selection, animation, ui);
                }
            } else if message.destination() == self.context_menu.duplicate {
                if let Some(animation) = selected_animation {
                    let commands = selection
                        .entities
                        .iter()
                        .filter_map(|e| match e {
                            SelectedEntity::Track(track_id) => {
                                let state = animation.tracks_data().state();
                                let tracks_data = state.data_ref()?;
                                let binding = animation.track_bindings().get(track_id)?;

                                let index = tracks_data
                                    .tracks()
                                    .iter()
                                    .position(|t| t.id() == *track_id)
                                    .unwrap();

                                let mut track = tracks_data.tracks()[index].clone();

                                track.set_id(Uuid::new_v4());

                                Some(Command::new(AddTrackCommand::new(
                                    selection.animation_player,
                                    selection.animation,
                                    track,
                                    TrackBinding::new(binding.target),
                                )))
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>();

                    sender.do_command(CommandGroup::from(commands));
                }
            }
        } else if let Some(TrackViewMessage::TrackEnabled(enabled)) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if let Some(animation) = selected_animation {
                    if let Some(track_view_ref) = ui
                        .node(message.destination())
                        .query_component::<TrackView>()
                    {
                        if animation.track_bindings().contains_key(&track_view_ref.id) {
                            sender.do_command(SetTrackEnabledCommand {
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

    fn open_property_selector<G, N>(
        graph: &G,
        node: Handle<N>,
        existing_binding: Option<&ValueBinding>,
        ui: &mut UserInterface,
    ) -> Handle<PropertySelectorWindow>
    where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        let mut descriptors = Vec::new();
        if let Ok(node) = graph.try_get_node(node) {
            node.as_reflect(&mut |node| {
                descriptors = object_to_property_tree("", node, &mut |field: &FieldRef| {
                    let type_id = field.value.field_value_as_reflect().type_id();
                    type_id != TypeId::of::<TextureBytes>()
                        // Vertex buffer cannot be animated (mainly because it contains untyped data).
                        && type_id != TypeId::of::<VertexBuffer>()
                        // Mesh topology cannot be animated.
                        && type_id != TypeId::of::<TriangleBuffer>()
                        // Makes no sense to animate drawing parameters.
                        && type_id != TypeId::of::<DrawParameters>()
                        && type_id != TypeId::of::<Samples>()
                        // Do not allow animating prefab's content.
                        && type_id != TypeId::of::<ModelResource>()
                        && type_id != TypeId::of::<Resource<UserInterface>>()
                });
            });
        }

        let property_selector = PropertySelectorWindowBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .with_title(WindowTitle::text("Select a Numeric Property To Animate"))
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
        .with_selected_property_paths(
            existing_binding
                .and_then(|binding| {
                    if let ValueBinding::Property { name, value_type } = binding {
                        Some(vec![PropertyDescriptorData {
                            name: name.to_string(),
                            path: name.to_string(),
                            type_id: value_type.into_type_id(),
                        }])
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
        )
        .with_property_descriptors(descriptors)
        .build(&mut ui.build_ctx());

        ui.send(
            property_selector,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );

        property_selector
    }

    fn on_rebind_clicked<G, N>(
        &mut self,
        graph: &G,
        selection: &AnimationSelection<N>,
        animation: &Animation<Handle<N>>,
        ui: &mut UserInterface,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        let Some(first_selected_track) = selection.first_selected_track() else {
            return;
        };

        let tracks_data_state = animation.tracks_data().state();
        let Some(tracks_data) = tracks_data_state.data_ref() else {
            return;
        };

        let Some(binding) = animation.track_bindings().get(&first_selected_track) else {
            return;
        };

        let Some(track) = tracks_data
            .tracks
            .iter()
            .find(|track| track.id() == first_selected_track)
        else {
            return;
        };

        self.context_menu.property_rebinding_selector =
            Self::open_property_selector(graph, binding.target(), Some(track.value_binding()), ui);
    }

    fn rebind_property<G, N>(
        &self,
        desc: &PropertyDescriptorData,
        graph: &G,
        selection: &AnimationSelection<N>,
        animation: &Animation<Handle<N>>,
        sender: &MessageSender,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        let Some(first_selected_track) = selection.first_selected_track() else {
            return;
        };

        let Some(binding) = animation.track_bindings().get(&first_selected_track) else {
            return;
        };

        let Ok(node) = graph.try_get_node(binding.target()) else {
            Log::err("Invalid node handle!");
            return;
        };

        node.resolve_path(&desc.path, &mut |result| match result {
            Ok(property) => {
                let mut property_type = TypeId::of::<u32>();
                property.as_any(&mut |any| property_type = any.type_id());

                let types = type_id_to_supported_type(property_type);

                if let Some((_, actual_value_type)) = types {
                    sender.do_command(SetTrackValueBindingCommand {
                        animation_player_handle: selection.animation_player,
                        animation_handle: selection.animation,
                        track: first_selected_track,
                        binding: ValueBinding::Property {
                            name: desc.path.clone().into(),
                            value_type: actual_value_type,
                        },
                    });
                }
            }
            Err(e) => {
                Log::err(format!("Invalid property path {desc:?}. Error: {e:?}!"));
            }
        })
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        ui.send(self.tree_root, TreeRootMessage::Items(vec![]));
        self.group_views.clear();
        self.track_views.clear();
        self.selected_node = Default::default();
    }

    pub fn sync_to_model<G, N>(
        &mut self,
        animation: &Animation<Handle<N>>,
        graph: &G,
        selection: &AnimationSelection<N>,
        ui: &mut UserInterface,
    ) where
        G: SceneGraph<Node = N>,
        N: SceneGraphNode,
    {
        let state = animation.tracks_data().state();
        let Some(tracks_data) = state.data_ref() else {
            return;
        };

        if Handle::<Animation<Handle<N>>>::from(self.selected_animation) != selection.animation {
            self.clear(ui);
            self.selected_animation = selection.animation.into();
        }

        match tracks_data.tracks().len().cmp(&self.track_views.len()) {
            Ordering::Less => {
                for track_view in self.track_views.clone().values() {
                    let track_view_ref = &ui[*track_view];
                    if tracks_data
                        .tracks()
                        .iter()
                        .all(|t| t.id() != track_view_ref.id)
                    {
                        for curve_item in track_view_ref.tree.items.iter().cloned() {
                            let curve_item_ref =
                                ui[curve_item].user_data_cloned::<CurveViewData>().unwrap();
                            assert!(self.curve_views.remove(&curve_item_ref.id).is_some());
                        }

                        assert!(self.track_views.remove(&track_view_ref.id).is_some());

                        // Remove group if it is empty.
                        if let Some(group) = self.group_views.get(&track_view_ref.target) {
                            ui.send_sync(*group, TreeMessage::RemoveItem(track_view.transmute()));

                            if ui[*group].items.len() <= 1 {
                                ui.send_sync(self.tree_root, TreeRootMessage::RemoveItem(*group));
                                assert!(self.group_views.remove(&track_view_ref.target).is_some());
                            }
                        }
                    }
                }
            }
            Ordering::Equal => {
                // Nothing to do.
            }
            Ordering::Greater => {
                for model_track in tracks_data.tracks().iter() {
                    let Some(model_track_binding) =
                        animation.track_bindings().get(&model_track.id())
                    else {
                        continue;
                    };

                    if self
                        .track_views
                        .values()
                        .all(|v| ui[*v].id != model_track.id())
                    {
                        let parent_group =
                            match self.group_views.entry(model_track_binding.target().into()) {
                                Entry::Occupied(entry) => *entry.get(),
                                Entry::Vacant(entry) => {
                                    let ctx = &mut ui.build_ctx();
                                    let group = TreeBuilder::new(WidgetBuilder::new())
                                        .with_content(
                                            TextBuilder::new(
                                                WidgetBuilder::new().with_vertical_alignment(
                                                    VerticalAlignment::Center,
                                                ),
                                            )
                                            .with_text(format!(
                                                "{} ({}:{})",
                                                graph
                                                    .try_get_node(model_track_binding.target())
                                                    .map(|n| n.name())
                                                    .unwrap_or_default(),
                                                model_track_binding.target().index(),
                                                model_track_binding.target().generation()
                                            ))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                    ui.send_sync(self.tree_root, TreeRootMessage::AddItem(group));
                                    *entry.insert(group)
                                }
                            };

                        let ctx = &mut ui.build_ctx();

                        let colors = [
                            Color::opaque(120, 0, 0),
                            Color::opaque(0, 120, 0),
                            Color::opaque(0, 0, 120),
                            Color::opaque(120, 0, 120),
                            Color::opaque(0, 120, 120),
                            Color::opaque(120, 120, 0),
                        ];

                        let curves = model_track
                            .data_container()
                            .curves_ref()
                            .iter()
                            .enumerate()
                            .map(|(i, curve)| {
                                let curve_name = match model_track.data_container().value_kind() {
                                    TrackValueKind::Real => "Value",
                                    TrackValueKind::Vector2
                                    | TrackValueKind::Vector3
                                    | TrackValueKind::Vector4
                                    | TrackValueKind::UnitQuaternion => {
                                        ["X", "Y", "Z", "W"].get(i).unwrap_or(&"_")
                                    }
                                    TrackValueKind::UnitQuaternionEuler => match i {
                                        0 => "Pitch",
                                        1 => "Yaw",
                                        2 => "Roll",
                                        _ => "Unknown",
                                    },
                                };

                                let curve_view = TreeBuilder::new(
                                    WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(
                                        CurveViewData { id: curve.id() },
                                    ))),
                                )
                                .with_content(
                                    GridBuilder::new(
                                        WidgetBuilder::new()
                                            .with_child(
                                                BorderBuilder::new(
                                                    WidgetBuilder::new()
                                                        .on_column(0)
                                                        .with_foreground(
                                                            Brush::Solid(Color::TRANSPARENT).into(),
                                                        )
                                                        .with_background(
                                                            Brush::Solid(colors[i]).into(),
                                                        ),
                                                )
                                                .with_pad_by_corner_radius(false)
                                                .with_corner_radius(2.0f32.into())
                                                .build(ctx),
                                            )
                                            .with_child(
                                                TextBuilder::new(
                                                    WidgetBuilder::new().on_column(1).with_margin(
                                                        Thickness {
                                                            top: 2.0,
                                                            left: 3.0,
                                                            ..Default::default()
                                                        },
                                                    ),
                                                )
                                                .with_text(curve_name)
                                                .build(ctx),
                                            ),
                                    )
                                    .add_row(Row::auto())
                                    .add_column(Column::strict(6.0))
                                    .add_column(Column::stretch())
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
                        .with_track_enabled(model_track_binding.is_enabled())
                        .with_id(model_track.id())
                        .with_target(model_track_binding.target().into())
                        .with_name(format!("{}", model_track.value_binding()))
                        .build(ctx);

                        ui.send_sync(parent_group, TreeMessage::AddItem(track_view.transmute()));

                        assert!(self
                            .track_views
                            .insert(model_track.id(), track_view)
                            .is_none());
                    }
                }
            }
        }

        let mut any_track_selected = false;
        let tree_selection = selection
            .entities
            .iter()
            .filter_map(|e| match e {
                SelectedEntity::Track(id) => {
                    any_track_selected = true;
                    self.track_views.get(id).cloned().map(|v| v.transmute())
                }
                SelectedEntity::Curve(id) => self.curve_views.get(id).cloned(),
                SelectedEntity::Signal(_) => None,
            })
            .collect();

        ui.send_sync(self.tree_root, TreeRootMessage::Select(tree_selection));

        ui.send_sync(
            self.context_menu.remove_track,
            WidgetMessage::Enabled(any_track_selected),
        );
        ui.send_sync(
            self.context_menu.set_target,
            WidgetMessage::Enabled(any_track_selected),
        );

        for model_track in tracks_data.tracks() {
            let Some(model_track_binding) = animation.track_bindings().get(&model_track.id())
            else {
                continue;
            };

            if let Some(track_view) = self.track_views.get(&model_track.id()) {
                let track_view_ref = &ui[*track_view];
                if track_view_ref.track_enabled != model_track_binding.is_enabled() {
                    ui.send_sync(
                        *track_view,
                        TrackViewMessage::TrackEnabled(model_track_binding.is_enabled()),
                    );
                }

                let mut validation_result = Ok(());
                if let Ok(target) = graph.try_get_node(model_track_binding.target()) {
                    if let Some(parent_group) =
                        self.group_views.get(&model_track_binding.target().into())
                    {
                        let content = ui[*parent_group].content;
                        ui.send_sync(
                            content,
                            TextMessage::Text(format!(
                                "{} ({}:{})",
                                target.name(),
                                model_track_binding.target().index(),
                                model_track_binding.target().generation()
                            )),
                        );
                    }

                    ui.send_sync(
                        *track_view,
                        TrackViewMessage::TrackName(format!("{}", model_track.value_binding())),
                    );

                    if let ValueBinding::Property { name, value_type } = model_track.value_binding()
                    {
                        target.resolve_path(name, &mut |result| match result {
                            Ok(value) => {
                                let mut property_type = TypeId::of::<u32>();
                                value.as_any(&mut |any| property_type = any.type_id());

                                if let Some((_, type_)) = type_id_to_supported_type(property_type) {
                                    if *value_type != type_ {
                                        validation_result = Err(format!(
                                            "Property type mismatch. Expected {value_type:?}, got {type_:?}"
                                        ));
                                    }
                                } else {
                                    validation_result = Err(format!(
                                        "Unsupported property type of {property_type:?} type id."
                                    ));
                                }
                            }
                            Err(err) => {
                                validation_result = Err(format!(
                                    "Unable to resolve property path {name}. Reason: {err:?}"
                                ));
                            }
                        });
                    }
                } else {
                    validation_result =
                        Err("Invalid handle. The target node does not exist!".to_owned());
                }

                ui.send_sync(
                    *track_view,
                    TrackViewMessage::TrackTargetIsValid(validation_result),
                );
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::plugins::animation::track::TrackViewBuilder;
    use fyrox::gui::tree::TreeBuilder;
    use fyrox::{gui::test::test_widget_deletion, gui::widget::WidgetBuilder};

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| {
            TrackViewBuilder::new(TreeBuilder::new(WidgetBuilder::new())).build(ctx)
        });
    }
}
