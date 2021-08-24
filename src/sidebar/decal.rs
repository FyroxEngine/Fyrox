use crate::{
    gui::{BuildContext, EditorUiNode, Ui, UiMessage, UiNode},
    make_relative_path,
    scene::commands::{
        decal::{
            SetDecalColorCommand, SetDecalDiffuseTextureCommand, SetDecalLayerIndexCommand,
            SetDecalNormalTextureCommand,
        },
        SceneCommand,
    },
    send_sync_message,
    sidebar::{
        make_color_input_field, make_int_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    engine::resource_manager::ResourceManager,
    gui::{
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ColorFieldMessage, ImageMessage, MessageDirection, NumericUpDownMessage, UiMessageData,
            WidgetMessage,
        },
        widget::WidgetBuilder,
        Thickness,
    },
    resource::texture::Texture,
    scene::node::Node,
    utils::into_gui_texture,
};
use std::sync::mpsc::Sender;

pub struct DecalSection {
    pub section: Handle<UiNode>,
    diffuse_texture: Handle<UiNode>,
    normal_texture: Handle<UiNode>,
    color: Handle<UiNode>,
    layer_index: Handle<UiNode>,
}

fn make_texture_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    ImageBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .on_row(row)
            .with_allow_drop(true)
            .with_margin(Thickness::uniform(1.0)),
    )
    .build(ctx)
}

fn send_image_sync_message(ui: &Ui, image: Handle<UiNode>, texture: Option<Texture>) {
    send_sync_message(
        ui,
        ImageMessage::texture(
            image,
            MessageDirection::ToWidget,
            texture.map(into_gui_texture),
        ),
    );
}

impl DecalSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let diffuse_texture;
        let normal_texture;
        let color;
        let layer_index;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Diffuse Texture", 0))
                .with_child({
                    diffuse_texture = make_texture_field(ctx, 0);
                    diffuse_texture
                })
                .with_child(make_text_mark(ctx, "Normal Texture", 1))
                .with_child({
                    normal_texture = make_texture_field(ctx, 1);
                    normal_texture
                })
                .with_child(make_text_mark(ctx, "Color", 2))
                .with_child({
                    color = make_color_input_field(ctx, 2);
                    color
                })
                .with_child(make_text_mark(ctx, "LayerIndex", 3))
                .with_child({
                    layer_index = make_int_input_field(ctx, 3, 0, 255, 1);
                    layer_index
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            diffuse_texture,
            normal_texture,
            color,
            layer_index,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_decal()),
        );

        if let Node::Decal(decal) = node {
            send_image_sync_message(ui, self.diffuse_texture, decal.diffuse_texture_value());
            send_image_sync_message(ui, self.normal_texture, decal.normal_texture_value());
            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.layer_index,
                    MessageDirection::ToWidget,
                    decal.layer() as f32,
                ),
            );
            send_sync_message(
                ui,
                ColorFieldMessage::color(self.color, MessageDirection::ToWidget, decal.color()),
            );
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        ui: &mut Ui,
        resource_manager: ResourceManager,
        node_handle: Handle<Node>,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        match *message.data() {
            UiMessageData::Widget(WidgetMessage::Drop(handle)) => {
                if let UiNode::User(EditorUiNode::AssetItem(item)) = ui.node(handle) {
                    let relative_path = make_relative_path(&item.path);

                    if message.destination() == self.diffuse_texture {
                        let texture = resource_manager.request_texture(relative_path, None);

                        sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetDecalDiffuseTexture(
                                    SetDecalDiffuseTextureCommand::new(
                                        node_handle,
                                        Some(texture.clone()),
                                    ),
                                ),
                            ))
                            .unwrap();

                        ui.send_message(ImageMessage::texture(
                            self.diffuse_texture,
                            MessageDirection::ToWidget,
                            Some(into_gui_texture(texture)),
                        ));
                    } else if message.destination() == self.normal_texture {
                        let texture = resource_manager.request_texture(relative_path, None);

                        sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetDecalNormalTexture(
                                    SetDecalNormalTextureCommand::new(
                                        node_handle,
                                        Some(texture.clone()),
                                    ),
                                ),
                            ))
                            .unwrap();

                        ui.send_message(ImageMessage::texture(
                            self.normal_texture,
                            MessageDirection::ToWidget,
                            Some(into_gui_texture(texture)),
                        ));
                    }
                }
            }
            UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value))
                if message.destination() == self.layer_index =>
            {
                sender
                    .send(Message::DoSceneCommand(SceneCommand::SetDecalLayerIndex(
                        SetDecalLayerIndexCommand::new(node_handle, value.clamp(0.0, 255.0) as u8),
                    )))
                    .unwrap();
            }
            UiMessageData::ColorField(ColorFieldMessage::Color(color))
                if message.destination() == self.color =>
            {
                sender
                    .send(Message::DoSceneCommand(SceneCommand::SetDecalColor(
                        SetDecalColorCommand::new(node_handle, color),
                    )))
                    .unwrap();
            }
            _ => {}
        }
    }
}
