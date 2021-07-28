use crate::scene::commands::mesh::SetMeshDecalLayerIndexCommand;
use crate::sidebar::make_int_input_field;
use crate::{
    gui::{make_dropdown_list_option, BuildContext, Ui, UiMessage, UiNode},
    scene::commands::{
        mesh::{SetMeshCastShadowsCommand, SetMeshRenderPathCommand},
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_bool_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::NumericUpDownMessage;
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            CheckBoxMessage, DropdownListMessage, MessageDirection, UiMessageData, WidgetMessage,
        },
        widget::WidgetBuilder,
        Thickness,
    },
    scene::{mesh::RenderPath, node::Node},
};
use std::sync::mpsc::Sender;

pub struct MeshSection {
    pub section: Handle<UiNode>,
    cast_shadows: Handle<UiNode>,
    render_path: Handle<UiNode>,
    decal_layer_index: Handle<UiNode>,
    sender: Sender<Message>,
}

impl MeshSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let cast_shadows;
        let render_path;
        let decal_layer_index;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Cast Shadows", 0))
                .with_child({
                    cast_shadows = make_bool_input_field(ctx, 0);
                    cast_shadows
                })
                .with_child(make_text_mark(ctx, "Render Path", 1))
                .with_child({
                    render_path = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_close_on_selection(true)
                    .with_items(vec![
                        make_dropdown_list_option(ctx, "Deferred"),
                        make_dropdown_list_option(ctx, "Forward"),
                    ])
                    .build(ctx);
                    render_path
                })
                .with_child(make_text_mark(ctx, "Decal Layer Index", 2))
                .with_child({
                    decal_layer_index = make_int_input_field(ctx, 2, 0, 255, 1);
                    decal_layer_index
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            cast_shadows,
            render_path,
            sender,
            decal_layer_index,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_mesh()),
        );

        if let Node::Mesh(mesh) = node {
            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.cast_shadows,
                    MessageDirection::ToWidget,
                    Some(mesh.cast_shadows()),
                ),
            );

            let variant = match mesh.render_path() {
                RenderPath::Deferred => 0,
                RenderPath::Forward => 1,
            };

            send_sync_message(
                ui,
                DropdownListMessage::selection(
                    self.render_path,
                    MessageDirection::ToWidget,
                    Some(variant),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.decal_layer_index,
                    MessageDirection::ToWidget,
                    mesh.decal_layer_index() as f32,
                ),
            );
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        scope_profile!();

        if let Node::Mesh(mesh) = node {
            match *message.data() {
                UiMessageData::CheckBox(CheckBoxMessage::Check(value)) => {
                    let value = value.unwrap_or(false);
                    if message.destination() == self.cast_shadows && mesh.cast_shadows().ne(&value)
                    {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetMeshCastShadows(
                                SetMeshCastShadowsCommand::new(handle, value),
                            )))
                            .unwrap();
                    }
                }
                UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(
                    selection,
                ))) => {
                    if message.destination() == self.render_path {
                        let new_render_path = match selection {
                            0 => RenderPath::Deferred,
                            1 => RenderPath::Forward,
                            _ => unreachable!(),
                        };
                        if new_render_path != mesh.render_path() {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetMeshRenderPath(
                                    SetMeshRenderPathCommand::new(handle, new_render_path),
                                )))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::NumericUpDown(NumericUpDownMessage::Value(index))
                    if message.destination() == self.decal_layer_index =>
                {
                    let index = index.clamp(0.0, 255.0) as u8;

                    if index != mesh.decal_layer_index() {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetMeshDecalLayerIndex(
                                    SetMeshDecalLayerIndexCommand::new(handle, index),
                                ),
                            ))
                            .unwrap();
                    }
                }
                _ => {}
            }
        }
    }
}
