use crate::scene::commands::decal::{SetDecalDiffuseTextureCommand, SetDecalNormalTextureCommand};
use crate::{
    gui::{BuildContext, EditorUiNode, Ui, UiMessage, UiNode},
    make_relative_path,
    scene::commands::{
        terrain::{SetTerrainLayerTextureCommand, TerrainLayerTextureKind},
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::Thickness;
use rg3d::{
    core::{pool::Handle, scope_profile},
    engine::resource_manager::ResourceManager,
    gui::{
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{ImageMessage, MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    resource::texture::Texture,
    scene::{node::Node, terrain::Layer},
    utils::into_gui_texture,
};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

pub struct DecalSection {
    pub section: Handle<UiNode>,
    diffuse_texture: Handle<UiNode>,
    normal_texture: Handle<UiNode>,
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
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            diffuse_texture,
            normal_texture,
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

        if let UiMessageData::Widget(WidgetMessage::Drop(handle)) = *message.data() {
            if let UiNode::User(EditorUiNode::AssetItem(item)) = ui.node(handle) {
                let relative_path = make_relative_path(&item.path);

                if message.destination() == self.diffuse_texture {
                    let texture = resource_manager.request_texture(relative_path);

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
                    let texture = resource_manager.request_texture(relative_path);

                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetDecalNormalTexture(SetDecalNormalTextureCommand::new(
                                node_handle,
                                Some(texture.clone()),
                            )),
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
    }
}
