use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{
        commands::terrain::{AddTerrainLayerCommand, DeleteTerrainLayerCommand},
        commands::SceneCommand,
    },
    send_sync_message,
    sidebar::{terrain::brush::BrushSection, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{ButtonMessage, ListViewMessage, UiMessageData},
        message::{MessageDirection, WidgetMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        Orientation,
    },
    scene::{
        graph::Graph,
        node::Node,
        terrain::{Brush, BrushKind, BrushMode},
    },
};
use std::sync::mpsc::Sender;

mod brush;

pub struct TerrainSection {
    pub section: Handle<UiNode>,
    brush_section: BrushSection,
    layers: Handle<UiNode>,
    add_layer: Handle<UiNode>,
    remove_layer: Handle<UiNode>,
    brush: Brush,
    current_layer: Option<usize>,
}

impl TerrainSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let brush_section = BrushSection::new(ctx);

        let layers;
        let add_layer;
        let remove_layer;
        let section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .with_child({
                                            add_layer = ButtonBuilder::new(WidgetBuilder::new())
                                                .with_text("Add Layer")
                                                .build(ctx);
                                            add_layer
                                        })
                                        .with_child({
                                            remove_layer = ButtonBuilder::new(WidgetBuilder::new())
                                                .with_text("Remove Layer")
                                                .build(ctx);
                                            remove_layer
                                        }),
                                )
                                .with_orientation(Orientation::Horizontal)
                                .build(ctx),
                            )
                            .with_child({
                                layers = ListViewBuilder::new(
                                    WidgetBuilder::new()
                                        .with_min_size(Vector2::new(0.0, ROW_HEIGHT * 3.0))
                                        .on_row(1)
                                        .on_column(0),
                                )
                                .build(ctx);
                                layers
                            }),
                    )
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .with_child(brush_section.section),
        )
        .with_orientation(Orientation::Vertical)
        .build(ctx);

        let brush = Brush {
            center: Default::default(),
            kind: BrushKind::Circle { radius: 1.0 },
            mode: BrushMode::AlternateHeightMap { amount: 1.0 },
        };

        Self {
            section,
            layers,
            add_layer,
            brush_section,
            brush,
            remove_layer,
            current_layer: None,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_terrain()),
        );

        if let Node::Terrain(terrain) = node {
            let layer_items = terrain
                .chunks_ref()
                .first()
                .unwrap()
                .layers()
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_child(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_text(format!("Layer {}", i))
                                .build(&mut ui.build_ctx()),
                        ),
                    ))
                    .build(&mut ui.build_ctx())
                })
                .collect::<Vec<_>>();

            ui.send_message(ListViewMessage::items(
                self.layers,
                MessageDirection::ToWidget,
                layer_items,
            ));
        }

        self.brush_section.sync_to_model(&self.brush, ui);
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        ui: &mut Ui,
        node: &Node,
        graph: &Graph,
        handle: Handle<Node>,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        if let Node::Terrain(sprite) = node {
            match *message.data() {
                UiMessageData::Button(ButtonMessage::Click) => {
                    if message.destination() == self.add_layer {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::AddTerrainLayer(
                                AddTerrainLayerCommand::new(handle, graph),
                            )))
                            .unwrap();
                    } else if message.destination() == self.remove_layer {
                        if let Some(index) = self.current_layer {
                            sender
                                .send(Message::DoSceneCommand(SceneCommand::DeleteTerrainLayer(
                                    DeleteTerrainLayerCommand::new(handle, index),
                                )))
                                .unwrap()
                        }
                    }
                }
                UiMessageData::ListView(ListViewMessage::SelectionChanged(layer_index)) => {
                    if message.destination() == self.layers && self.current_layer != layer_index {
                        self.current_layer = layer_index;
                        self.sync_to_model(node, ui);
                    }
                }
                _ => {}
            }
        }
    }
}
