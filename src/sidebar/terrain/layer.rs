use crate::{
    send_sync_message,
    sidebar::{make_section, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::{ButtonMessage, MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::{graph::Graph, node::Node, terrain::Layer},
};
use std::sync::mpsc::Sender;

pub struct LayerSection {
    pub section: Handle<UiNode>,
    material: Handle<UiNode>,
}

impl LayerSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let material;
        let section = make_section(
            "Layer Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Material", 0))
                    .with_child({
                        material = ButtonBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                            .with_text("...")
                            .build(ctx);
                        material
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self { section, material }
    }

    pub fn sync_to_model(&mut self, layer: Option<&Layer>, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, layer.is_some()),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        graph: &Graph,
        node_handle: Handle<Node>,
        layer_index: usize,
        sender: &Sender<Message>,
    ) {
        scope_profile!();
        if let UiMessageData::Button(ButtonMessage::Click) = message.data() {}
        if message.destination() == self.material {
            sender
                .send(Message::OpenMaterialEditor(
                    graph[node_handle].as_terrain().layers()[layer_index]
                        .material
                        .clone(),
                ))
                .unwrap();
        }
    }
}
