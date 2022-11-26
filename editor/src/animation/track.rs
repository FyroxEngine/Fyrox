#![allow(clippy::manual_map)]

use crate::{
    animation::{
        command::AddTrackCommand,
        selection::{AnimationSelection, SelectedEntity},
    },
    scene::{
        commands::ChangeSelectionCommand,
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene, Selection,
    },
    Message,
};
use fyrox::{
    animation::{
        container::{TrackFramesContainer, TrackValueKind},
        value::ValueBinding,
        Animation, NodeTrack,
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
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeMessage, TreeRootBuilder, TreeRootMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
    scene::{graph::Graph, node::Node, Scene},
    utils::log::Log,
};
use std::{any::TypeId, cmp::Ordering, collections::hash_map::Entry, rc::Rc, sync::mpsc::Sender};

pub struct TrackList {
    pub panel: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    add_track: Handle<UiNode>,
    node_selector: Handle<UiNode>,
    property_selector: Handle<UiNode>,
    selected_node: Handle<Node>,
    group_views: FxHashMap<Handle<Node>, Handle<UiNode>>,
    track_views: Vec<Handle<UiNode>>,
}

struct TrackViewData {
    id: Uuid,
    target: Handle<Node>,
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
        let tree_root;
        let add_track;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    ScrollViewerBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
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
                            .on_row(1)
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
        .add_row(Row::stretch())
        .add_row(Row::strict(22.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            panel,
            tree_root,
            add_track,
            node_selector: Default::default(),
            property_selector: Default::default(),
            selected_node: Default::default(),
            group_views: Default::default(),
            track_views: Default::default(),
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

                                let container = if property_type == TypeId::of::<f32>()
                                    || property_type == TypeId::of::<f64>()
                                    || property_type == TypeId::of::<u64>()
                                    || property_type == TypeId::of::<i64>()
                                    || property_type == TypeId::of::<u32>()
                                    || property_type == TypeId::of::<i32>()
                                    || property_type == TypeId::of::<u16>()
                                    || property_type == TypeId::of::<i16>()
                                    || property_type == TypeId::of::<u8>()
                                    || property_type == TypeId::of::<i8>()
                                    || property_type == TypeId::of::<bool>()
                                {
                                    Some(TrackFramesContainer::new(TrackValueKind::Real))
                                } else if property_type == TypeId::of::<Vector2<f32>>()
                                    || property_type == TypeId::of::<Vector2<f64>>()
                                    || property_type == TypeId::of::<Vector2<u64>>()
                                    || property_type == TypeId::of::<Vector2<i64>>()
                                    || property_type == TypeId::of::<Vector2<u32>>()
                                    || property_type == TypeId::of::<Vector2<i32>>()
                                    || property_type == TypeId::of::<Vector2<u16>>()
                                    || property_type == TypeId::of::<Vector2<i16>>()
                                    || property_type == TypeId::of::<Vector2<u8>>()
                                    || property_type == TypeId::of::<Vector2<i8>>()
                                    || property_type == TypeId::of::<Vector2<bool>>()
                                {
                                    Some(TrackFramesContainer::new(TrackValueKind::Vector2))
                                } else if property_type == TypeId::of::<Vector3<f32>>()
                                    || property_type == TypeId::of::<Vector3<f64>>()
                                    || property_type == TypeId::of::<Vector3<u64>>()
                                    || property_type == TypeId::of::<Vector3<i64>>()
                                    || property_type == TypeId::of::<Vector3<u32>>()
                                    || property_type == TypeId::of::<Vector3<i32>>()
                                    || property_type == TypeId::of::<Vector3<u16>>()
                                    || property_type == TypeId::of::<Vector3<i16>>()
                                    || property_type == TypeId::of::<Vector3<u8>>()
                                    || property_type == TypeId::of::<Vector3<i8>>()
                                    || property_type == TypeId::of::<Vector3<bool>>()
                                {
                                    Some(TrackFramesContainer::new(TrackValueKind::Vector3))
                                } else if property_type == TypeId::of::<Vector4<f32>>()
                                    || property_type == TypeId::of::<Vector4<f64>>()
                                    || property_type == TypeId::of::<Vector4<u64>>()
                                    || property_type == TypeId::of::<Vector4<i64>>()
                                    || property_type == TypeId::of::<Vector4<u32>>()
                                    || property_type == TypeId::of::<Vector4<i32>>()
                                    || property_type == TypeId::of::<Vector4<u16>>()
                                    || property_type == TypeId::of::<Vector4<i16>>()
                                    || property_type == TypeId::of::<Vector4<u8>>()
                                    || property_type == TypeId::of::<Vector4<i8>>()
                                    || property_type == TypeId::of::<Vector4<bool>>()
                                {
                                    Some(TrackFramesContainer::new(TrackValueKind::Vector4))
                                } else if property_type == TypeId::of::<UnitQuaternion<f32>>() {
                                    Some(TrackFramesContainer::new(TrackValueKind::UnitQuaternion))
                                } else {
                                    None
                                };

                                if let Some(container) = container {
                                    let mut track = NodeTrack::new(
                                        container,
                                        ValueBinding::Property(property_path.path.clone()),
                                    );

                                    track.set_serialize_frames(true);
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
                            if let Some(track_data) =
                                selected_widget.user_data_ref::<TrackViewData>()
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

    pub fn sync_to_model(&mut self, animation: &Animation, graph: &Graph, ui: &mut UserInterface) {
        match animation.tracks().len().cmp(&self.track_views.len()) {
            Ordering::Less => {
                for track_view in self.track_views.clone().iter() {
                    let track_view_ref = ui.node(*track_view);
                    let track_view_data = track_view_ref.user_data_ref::<TrackViewData>().unwrap();
                    if animation
                        .tracks()
                        .iter()
                        .all(|t| t.id() != track_view_data.id)
                    {
                        ui.send_message(TreeRootMessage::remove_item(
                            self.tree_root,
                            MessageDirection::ToWidget,
                            *track_view,
                        ));

                        self.track_views.remove(
                            self.track_views
                                .iter()
                                .position(|v| *v == *track_view)
                                .unwrap(),
                        );

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
                                ui.send_message(TreeRootMessage::remove_item(
                                    self.tree_root,
                                    MessageDirection::ToWidget,
                                    *group,
                                ));
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
                        .iter()
                        .map(|v| ui.node(*v))
                        .all(|v| v.user_data_ref::<TrackViewData>().unwrap().id != model_track.id())
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

                                ui.send_message(TreeRootMessage::add_item(
                                    self.tree_root,
                                    MessageDirection::ToWidget,
                                    group,
                                ));

                                *entry.insert(group)
                            }
                        };

                        let ctx = &mut ui.build_ctx();

                        let track_view =
                            TreeBuilder::new(WidgetBuilder::new().with_user_data(Rc::new(
                                TrackViewData {
                                    id: model_track.id(),
                                    target: model_track.target(),
                                },
                            )))
                            .with_items(
                                model_track
                                    .frames_container()
                                    .curves_ref()
                                    .iter()
                                    .enumerate()
                                    .map(|(i, curve)| {
                                        TreeBuilder::new(WidgetBuilder::new().with_user_data(
                                            Rc::new(CurveViewData { id: curve.id() }),
                                        ))
                                        .with_content(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_text(format!(
                                                    "Curve - {}",
                                                    ["X", "Y", "Z", "W"].get(i).unwrap_or(&"_"),
                                                ))
                                                .build(ctx),
                                        )
                                        .build(ctx)
                                    })
                                    .collect(),
                            )
                            .with_content(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_text(format!("{}", model_track.binding()))
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx),
                            )
                            .build(ctx);

                        ui.send_message(TreeMessage::add_item(
                            parent_group,
                            MessageDirection::ToWidget,
                            track_view,
                        ));

                        self.track_views.push(track_view);
                    }
                }
            }
        }
    }
}
