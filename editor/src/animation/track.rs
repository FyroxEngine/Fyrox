use crate::{
    animation::{
        command::{AddTrackCommand, AnimationCommand},
        data::DataModel,
        message::Message,
    },
    scene::{
        property::{
            object_to_property_tree, PropertySelectorMessage, PropertySelectorWindowBuilder,
        },
        selector::{HierarchyNode, NodeSelectorMessage, NodeSelectorWindowBuilder},
        EditorScene,
    },
};
use fyrox::{
    animation::{
        container::{TrackFramesContainer, TrackValueKind},
        definition::ResourceTrack,
        value::ValueBinding,
    },
    core::{pool::Handle, reflect::ResolvePath, uuid::Uuid, variable::InheritableVariable},
    engine::Engine,
    fxhash::FxHashSet,
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, VerticalAlignment,
    },
    scene::node::Node,
    utils::log::Log,
};
use std::{any::TypeId, cmp::Ordering, rc::Rc, sync::mpsc::Sender};

pub struct TrackList {
    pub panel: Handle<UiNode>,
    list: Handle<UiNode>,
    add_track: Handle<UiNode>,
    node_selector: Handle<UiNode>,
    property_selector: Handle<UiNode>,
    selected_node: Handle<Node>,
}

struct TrackViewData {
    id: Uuid,
}

impl TrackList {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let list;
        let add_track;

        let panel = GridBuilder::new(
            WidgetBuilder::new()
                .with_enabled(false)
                .with_child({
                    list = ListViewBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .build(ctx);
                    list
                })
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
            list,
            add_track,
            node_selector: Default::default(),
            property_selector: Default::default(),
            selected_node: Default::default(),
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: Option<&EditorScene>,
        engine: &mut Engine,
        sender: &Sender<Message>,
    ) {
        let ui = &mut engine.user_interface;

        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.add_track {
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];

                    self.node_selector = NodeSelectorWindowBuilder::new(
                        WindowBuilder::new(
                            WidgetBuilder::new().with_width(300.0).with_height(400.0),
                        )
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
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];
                    if let Some(first) = selection.first() {
                        self.selected_node = *first;

                        self.property_selector = PropertySelectorWindowBuilder::new(
                            WindowBuilder::new(
                                WidgetBuilder::new().with_width(300.0).with_height(400.0),
                            )
                            .with_title(WindowTitle::text("Select a Numeric Property To Animate"))
                            .open(false),
                        )
                        .with_allowed_types(Some(FxHashSet::from_iter([
                            TypeId::of::<InheritableVariable<f32>>(),
                            TypeId::of::<InheritableVariable<f64>>(),
                            TypeId::of::<InheritableVariable<u64>>(),
                            TypeId::of::<InheritableVariable<i64>>(),
                            TypeId::of::<InheritableVariable<u32>>(),
                            TypeId::of::<InheritableVariable<i32>>(),
                            TypeId::of::<InheritableVariable<u16>>(),
                            TypeId::of::<InheritableVariable<i16>>(),
                            TypeId::of::<InheritableVariable<u8>>(),
                            TypeId::of::<InheritableVariable<i8>>(),
                            TypeId::of::<InheritableVariable<bool>>(),
                            TypeId::of::<f32>(),
                            TypeId::of::<f64>(),
                            TypeId::of::<u64>(),
                            TypeId::of::<i64>(),
                            TypeId::of::<u32>(),
                            TypeId::of::<i32>(),
                            TypeId::of::<u16>(),
                            TypeId::of::<i16>(),
                            TypeId::of::<u8>(),
                            TypeId::of::<i8>(),
                            TypeId::of::<bool>(),
                        ])))
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
            }
        } else if let Some(PropertySelectorMessage::Selection(selected_properties)) = message.data()
        {
            if message.destination() == self.property_selector
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(editor_scene) = editor_scene {
                    let scene = &engine.scenes[editor_scene.scene];
                    if let Some(node) = scene.graph.try_get(self.selected_node) {
                        for property_path in selected_properties {
                            match node.as_reflect().resolve_path(property_path) {
                                Ok(_property) => {
                                    // TODO: Check property type.
                                    let mut track =
                                        ResourceTrack::new(TrackFramesContainer::with_n_curves(
                                            TrackValueKind::Vector3,
                                            3,
                                        ));
                                    track
                                        .set_binding(ValueBinding::Property(property_path.clone()));

                                    sender
                                        .send(Message::DoCommand(AnimationCommand::new(
                                            AddTrackCommand::new(track),
                                        )))
                                        .unwrap();
                                }
                                Err(e) => {
                                    Log::err(format!(
                                        "Invalid property path {}. Error: {:?}!",
                                        property_path, e
                                    ));
                                }
                            }
                        }
                    } else {
                        Log::err("Invalid node handle!");
                    }
                }
            }
        }
    }

    pub fn sync_to_model(&mut self, engine: &mut Engine, data_model: Option<&DataModel>) {
        let ui = &mut engine.user_interface;

        ui.send_message(WidgetMessage::enabled(
            self.panel,
            MessageDirection::ToWidget,
            data_model.is_some(),
        ));

        if let Some(data_model) = data_model {
            let data_ref = data_model.resource.data_ref();
            let definition = &data_ref.animation_definition;
            let track_views = ui
                .node(self.list)
                .query_component::<ListView>()
                .unwrap()
                .items
                .clone();
            match definition.tracks().len().cmp(&track_views.len()) {
                Ordering::Less => {
                    for track_view in track_views.iter() {
                        let track_view_ref = ui.node(*track_view);
                        let track_view_data =
                            track_view_ref.user_data_ref::<TrackViewData>().unwrap();
                        if definition
                            .tracks()
                            .iter()
                            .all(|t| t.id() != track_view_data.id)
                        {
                            ui.send_message(ListViewMessage::remove_item(
                                self.list,
                                MessageDirection::ToWidget,
                                *track_view,
                            ));
                        }
                    }
                }
                Ordering::Equal => {
                    // Nothing to do.
                }
                Ordering::Greater => {
                    for model_track in definition.tracks().iter() {
                        if track_views.iter().map(|v| ui.node(*v)).all(|v| {
                            v.user_data_ref::<TrackViewData>().unwrap().id != model_track.id()
                        }) {
                            let track_view = DecoratorBuilder::new(BorderBuilder::new(
                                WidgetBuilder::new()
                                    .with_user_data(Rc::new(TrackViewData {
                                        id: model_track.id(),
                                    }))
                                    .with_height(18.0)
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_child(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text(format!("{}", model_track.binding()))
                                            .with_vertical_text_alignment(VerticalAlignment::Center)
                                            .build(&mut ui.build_ctx()),
                                    ),
                            ))
                            .build(&mut ui.build_ctx());

                            ui.send_message(ListViewMessage::add_item(
                                self.list,
                                MessageDirection::ToWidget,
                                track_view,
                            ))
                        }
                    }
                }
            }
        }
    }
}
