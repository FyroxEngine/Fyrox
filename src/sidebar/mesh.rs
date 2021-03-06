use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{SceneCommand, SetMeshCastShadowsCommand},
    send_sync_message,
    sidebar::{make_bool_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{CheckBoxMessage, MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct MeshSection {
    pub section: Handle<UiNode>,
    cast_shadows: Handle<UiNode>,
    sender: Sender<Message>,
}

impl MeshSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let cast_shadows;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Cast Shadows", 0))
                .with_child({
                    cast_shadows = make_bool_input_field(ctx, 0);
                    cast_shadows
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            cast_shadows,
            sender,
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
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        scope_profile!();

        if let Node::Mesh(mesh) = node {
            if let UiMessageData::CheckBox(msg) = &message.data() {
                if let CheckBoxMessage::Check(value) = *msg {
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
            }
        }
    }
}
