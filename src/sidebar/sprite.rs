use crate::sidebar::make_section;
use crate::{
    scene::commands::sprite::{
        SetSpriteColorCommand, SetSpriteRotationCommand, SetSpriteSizeCommand,
    },
    send_sync_message,
    sidebar::{
        make_color_input_field, make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{ColorFieldMessage, MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct SpriteSection {
    pub section: Handle<UiNode>,
    size: Handle<UiNode>,
    rotation: Handle<UiNode>,
    color: Handle<UiNode>,
    sender: Sender<Message>,
}

impl SpriteSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let size;
        let rotation;
        let color;
        let section = make_section(
            "Sprite Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Size", 0))
                    .with_child({
                        size = make_f32_input_field(ctx, 0, 0.0, f32::MAX, 0.1);
                        size
                    })
                    .with_child(make_text_mark(ctx, "Rotation", 1))
                    .with_child({
                        rotation = make_f32_input_field(ctx, 1, 0.0, f32::MAX, 0.1);
                        rotation
                    })
                    .with_child(make_text_mark(ctx, "Color", 2))
                    .with_child({
                        color = make_color_input_field(ctx, 2);
                        color
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            size,
            rotation,
            sender,
            color,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_sprite()),
        );

        if let Node::Sprite(sprite) = node {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(self.size, MessageDirection::ToWidget, sprite.size()),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.rotation,
                    MessageDirection::ToWidget,
                    sprite.rotation(),
                ),
            );

            send_sync_message(
                ui,
                ColorFieldMessage::color(self.color, MessageDirection::ToWidget, sprite.color()),
            );
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        scope_profile!();

        if let Node::Sprite(sprite) = node {
            match message.data() {
                UiMessageData::User(msg) => {
                    if let Some(&NumericUpDownMessage::Value(value)) =
                        msg.cast::<NumericUpDownMessage<f32>>()
                    {
                        if message.destination() == self.size && sprite.size().ne(&value) {
                            self.sender
                                .send(Message::do_scene_command(SetSpriteSizeCommand::new(
                                    handle, value,
                                )))
                                .unwrap();
                        } else if message.destination() == self.rotation
                            && sprite.rotation().ne(&value)
                        {
                            self.sender
                                .send(Message::do_scene_command(SetSpriteRotationCommand::new(
                                    handle, value,
                                )))
                                .unwrap();
                        }
                    }
                }
                &UiMessageData::ColorField(ColorFieldMessage::Color(color)) => {
                    if message.destination() == self.color && sprite.color() != color {
                        self.sender
                            .send(Message::do_scene_command(SetSpriteColorCommand::new(
                                handle, color,
                            )))
                            .unwrap();
                    }
                }
                _ => {}
            }
        }
    }
}
