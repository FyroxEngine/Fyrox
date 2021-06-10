use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    send_sync_message,
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData, Vec3EditorMessage},
        widget::WidgetBuilder,
    },
    sound::source::SoundSource,
};
use std::sync::mpsc::Sender;

pub struct SoundSection {
    pub section: Handle<UiNode>,
    position: Handle<UiNode>,
}

impl SoundSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let position;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Position", 0))
                .with_child({
                    position = make_vec3_input_field(ctx, 0);
                    position
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self { section, position }
    }

    pub fn sync_to_model(&mut self, source: &SoundSource, ui: &mut Ui) {
        if let SoundSource::Spatial(spatial) = source {
            send_sync_message(
                ui,
                Vec3EditorMessage::value(
                    self.position,
                    MessageDirection::ToWidget,
                    spatial.position(),
                ),
            );
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        source: &SoundSource,
        handle: Handle<SoundSource>,
    ) {
        scope_profile!();

        if let SoundSource::Spatial(spatial) = source {
            match *message.data() {
                UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                    if spatial.position() != value {}
                }
                _ => {}
            }
        }
    }
}
