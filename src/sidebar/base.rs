use crate::gui::make_dropdown_list_option;
use crate::sidebar::make_section;
use crate::{
    scene::commands::{
        graph::{
            MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SetNameCommand,
            SetPhysicsBindingCommand, SetTagCommand,
        },
        lod::SetLodGroupCommand,
    },
    send_sync_message,
    sidebar::{
        lod::LodGroupEditor, make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::vec::vec3::Vec3EditorMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, RotationOrder},
        pool::Handle,
    },
    gui::{
        button::ButtonBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, TextBoxMessage, TextMessage,
            UiMessageData, WidgetMessage,
        },
        text::TextBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        Thickness,
    },
    scene::{base::PhysicsBinding, node::Node},
};
use std::sync::mpsc::Sender;

pub struct BaseSection {
    pub section: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    resource: Handle<UiNode>,
    tag: Handle<UiNode>,
    create_lod_group: Handle<UiNode>,
    remove_lod_group: Handle<UiNode>,
    edit_lod_group: Handle<UiNode>,
    physics_binding: Handle<UiNode>,
}

impl BaseSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let node_name;
        let position;
        let rotation;
        let scale;
        let resource;
        let tag;
        let physics_binding;
        let create_lod_group;
        let remove_lod_group;
        let edit_lod_group;
        let section = make_section(
            "Node Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Name", 0))
                    .with_child({
                        node_name = TextBoxBuilder::new(
                            WidgetBuilder::new()
                                .on_row(0)
                                .on_column(1)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .build(ctx);
                        node_name
                    })
                    .with_child(make_text_mark(ctx, "Position", 1))
                    .with_child({
                        position = make_vec3_input_field(ctx, 1);
                        position
                    })
                    .with_child(make_text_mark(ctx, "Rotation", 2))
                    .with_child({
                        rotation = make_vec3_input_field(ctx, 2);
                        rotation
                    })
                    .with_child(make_text_mark(ctx, "Scale", 3))
                    .with_child({
                        scale = make_vec3_input_field(ctx, 3);
                        scale
                    })
                    .with_child(make_text_mark(ctx, "Resource", 4))
                    .with_child({
                        resource = TextBuilder::new(WidgetBuilder::new().on_column(1).on_row(4))
                            .build(ctx);
                        resource
                    })
                    .with_child(make_text_mark(ctx, "Tag", 5))
                    .with_child({
                        tag = TextBoxBuilder::new(WidgetBuilder::new().on_column(1).on_row(5))
                            .build(ctx);
                        tag
                    })
                    .with_child(make_text_mark(ctx, "Physics Binding", 6))
                    .with_child({
                        physics_binding = DropdownListBuilder::new(
                            WidgetBuilder::new()
                                .on_row(6)
                                .on_column(1)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_close_on_selection(true)
                        .with_items(vec![
                            make_dropdown_list_option(ctx, "Node With Body"),
                            make_dropdown_list_option(ctx, "Body With Node"),
                        ])
                        .build(ctx);
                        physics_binding
                    })
                    .with_child(make_text_mark(ctx, "LOD Group", 7))
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .on_row(7)
                                .on_column(1)
                                .with_child({
                                    create_lod_group = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_margin(Thickness::uniform(1.0))
                                            .on_column(0),
                                    )
                                    .with_text("Create Group")
                                    .build(ctx);
                                    create_lod_group
                                })
                                .with_child({
                                    remove_lod_group = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_enabled(false)
                                            .with_margin(Thickness::uniform(1.0))
                                            .on_column(1),
                                    )
                                    .with_text("Remove Group")
                                    .build(ctx);
                                    remove_lod_group
                                })
                                .with_child({
                                    edit_lod_group = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_enabled(false)
                                            .with_margin(Thickness::uniform(1.0))
                                            .on_column(2),
                                    )
                                    .with_text("Edit Group...")
                                    .build(ctx);
                                    edit_lod_group
                                }),
                        )
                        .add_row(Row::stretch())
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .build(ctx),
                    ),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::stretch())
            .build(ctx),
            ctx,
        );

        Self {
            section,
            node_name,
            position,
            rotation,
            scale,
            resource,
            tag,
            physics_binding,
            create_lod_group,
            remove_lod_group,
            edit_lod_group,
        }
    }

    pub fn sync_to_model(&self, node: &Node, ui: &UserInterface) {
        send_sync_message(
            ui,
            TextBoxMessage::text(
                self.node_name,
                MessageDirection::ToWidget,
                node.name().to_owned(),
            ),
        );

        // Prevent edit names of nodes that were created from resource.
        // This is strictly necessary because resolving depends on node
        // names.
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.node_name,
                MessageDirection::ToWidget,
                node.resource().is_none() || node.is_resource_instance_root(),
            ),
        );

        send_sync_message(
            ui,
            TextMessage::text(
                self.resource,
                MessageDirection::ToWidget,
                if let Some(resource) = node.resource() {
                    let state = resource.state();
                    state.path().to_string_lossy().into_owned()
                } else {
                    "None".to_owned()
                },
            ),
        );

        send_sync_message(
            ui,
            TextBoxMessage::text(self.tag, MessageDirection::ToWidget, node.tag().to_owned()),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.position,
                MessageDirection::ToWidget,
                **node.local_transform().position(),
            ),
        );

        let euler = node.local_transform().rotation().euler_angles();
        let euler_degrees = Vector3::new(
            euler.0.to_degrees(),
            euler.1.to_degrees(),
            euler.2.to_degrees(),
        );
        send_sync_message(
            ui,
            Vec3EditorMessage::value(self.rotation, MessageDirection::ToWidget, euler_degrees),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.scale,
                MessageDirection::ToWidget,
                **node.local_transform().scale(),
            ),
        );

        let id = match node.physics_binding() {
            PhysicsBinding::NodeWithBody => 0,
            PhysicsBinding::BodyWithNode => 1,
        };
        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.physics_binding,
                MessageDirection::ToWidget,
                Some(id),
            ),
        );

        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.create_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_none(),
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.remove_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_some(),
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.edit_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_some(),
            ),
        );
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        sender: &Sender<Message>,
        node: &Node,
        node_handle: Handle<Node>,
        ui: &mut UserInterface,
        lod_editor: &mut LodGroupEditor,
    ) {
        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.create_lod_group {
                    sender
                        .send(Message::do_scene_command(SetLodGroupCommand::new(
                            node_handle,
                            Some(Default::default()),
                        )))
                        .unwrap();
                } else if message.destination() == self.remove_lod_group {
                    sender
                        .send(Message::do_scene_command(SetLodGroupCommand::new(
                            node_handle,
                            None,
                        )))
                        .unwrap();
                } else if message.destination() == self.edit_lod_group {
                    lod_editor.open(ui);
                }
            }
            UiMessageData::User(msg) => {
                if let Some(&Vec3EditorMessage::Value(value)) = msg.cast::<Vec3EditorMessage<f32>>()
                {
                    let transform = node.local_transform();
                    if message.destination() == self.rotation {
                        let old_rotation = **transform.rotation();
                        let euler = Vector3::new(
                            value.x.to_radians(),
                            value.y.to_radians(),
                            value.z.to_radians(),
                        );
                        let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                        if old_rotation.ne(&new_rotation) {
                            sender
                                .send(Message::do_scene_command(RotateNodeCommand::new(
                                    node_handle,
                                    old_rotation,
                                    new_rotation,
                                )))
                                .unwrap();
                        }
                    } else if message.destination() == self.position {
                        let old_position = **transform.position();
                        if old_position != value {
                            sender
                                .send(Message::do_scene_command(MoveNodeCommand::new(
                                    node_handle,
                                    old_position,
                                    value,
                                )))
                                .unwrap();
                        }
                    } else if message.destination() == self.scale {
                        let old_scale = **transform.scale();
                        if old_scale != value {
                            sender
                                .send(Message::do_scene_command(ScaleNodeCommand::new(
                                    node_handle,
                                    old_scale,
                                    value,
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::TextBox(TextBoxMessage::Text(value)) => {
                if message.destination() == self.node_name {
                    let old_name = node.name();
                    if old_name != value {
                        sender
                            .send(Message::do_scene_command(SetNameCommand::new(
                                node_handle,
                                value.to_owned(),
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.tag {
                    let old_tag = node.tag();
                    if old_tag != value {
                        sender
                            .send(Message::do_scene_command(SetTagCommand::new(
                                node_handle,
                                value.to_owned(),
                            )))
                            .unwrap();
                    }
                }
            }

            UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(index))) => {
                if message.destination() == self.physics_binding {
                    let id = match node.physics_binding() {
                        PhysicsBinding::NodeWithBody => 0,
                        PhysicsBinding::BodyWithNode => 1,
                    };

                    if id != *index {
                        let value = match *index {
                            0 => PhysicsBinding::NodeWithBody,
                            1 => PhysicsBinding::BodyWithNode,
                            _ => unreachable!(),
                        };
                        sender
                            .send(Message::do_scene_command(SetPhysicsBindingCommand::new(
                                node_handle,
                                value,
                            )))
                            .unwrap();
                    }
                }
            }

            _ => (),
        }
    }
}
