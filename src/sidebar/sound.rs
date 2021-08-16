use crate::scene::commands::sound::{
    SetSpatialSoundSourceMaxDistanceCommand, SetSpatialSoundSourceRadiusCommand,
    SetSpatialSoundSourceRolloffFactorCommand,
};
use crate::{
    asset::AssetKind,
    gui::{BuildContext, EditorUiNode, Ui, UiMessage, UiNode},
    make_relative_path,
    scene::commands::{
        sound::{
            SetSoundSourceBufferCommand, SetSoundSourceGainCommand, SetSoundSourceLoopingCommand,
            SetSoundSourceNameCommand, SetSoundSourcePitchCommand, SetSoundSourcePlayOnceCommand,
            SetSpatialSoundSourcePositionCommand,
        },
        SceneCommand,
    },
    send_sync_message,
    sidebar::{
        make_bool_input_field, make_f32_input_field, make_text_mark, make_vec3_input_field,
        COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::{
    core::{futures::executor::block_on, pool::Handle, scope_profile},
    engine::resource_manager::ResourceManager,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{
            CheckBoxMessage, MessageDirection, NumericUpDownMessage, TextBoxMessage, UiMessageData,
            Vec3EditorMessage, WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        Thickness, VerticalAlignment,
    },
    sound::source::{spatial::SpatialSource, SoundSource},
};
use std::sync::mpsc::Sender;

struct SpatialSection {
    pub section: Handle<UiNode>,
    position: Handle<UiNode>,
    radius: Handle<UiNode>,
    rolloff_factor: Handle<UiNode>,
    max_distance: Handle<UiNode>,
}

impl SpatialSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let position;
        let radius;
        let rolloff_factor;
        let max_distance;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Position", 0))
                .with_child({
                    position = make_vec3_input_field(ctx, 0);
                    position
                })
                .with_child(make_text_mark(ctx, "Radius", 1))
                .with_child({
                    radius = make_f32_input_field(ctx, 1, 0.0, f32::MAX, 0.1);
                    radius
                })
                .with_child(make_text_mark(ctx, "Rolloff Factor", 2))
                .with_child({
                    rolloff_factor = make_f32_input_field(ctx, 2, 0.0, f32::MAX, 0.1);
                    rolloff_factor
                })
                .with_child(make_text_mark(ctx, "Max Distance", 3))
                .with_child({
                    max_distance = make_f32_input_field(ctx, 3, 0.0, f32::MAX, 0.1);
                    max_distance
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
            position,
            radius,
            rolloff_factor,
            max_distance,
        }
    }

    pub fn sync_to_model(&mut self, spatial: &SpatialSource, ui: &mut Ui) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.position,
                MessageDirection::ToWidget,
                spatial.position(),
            ),
        );

        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, spatial.radius()),
        );
        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.rolloff_factor,
                MessageDirection::ToWidget,
                spatial.rolloff_factor(),
            ),
        );
        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.max_distance,
                MessageDirection::ToWidget,
                spatial.max_distance(),
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        spatial: &SpatialSource,
        handle: Handle<SoundSource>,
    ) {
        scope_profile!();

        match *message.data() {
            UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                if spatial.position() != value && message.destination() == self.position {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSpatialSoundSourcePosition(
                                SetSpatialSoundSourcePositionCommand::new(handle, value),
                            ),
                        ))
                        .unwrap();
                }
            }
            UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if spatial.radius().ne(&value) && message.destination() == self.radius {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSpatialSoundSourceRadius(
                                SetSpatialSoundSourceRadiusCommand::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if spatial.rolloff_factor().ne(&value)
                    && message.destination() == self.rolloff_factor
                {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSpatialSoundSourceRolloffFactor(
                                SetSpatialSoundSourceRolloffFactorCommand::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if spatial.max_distance().ne(&value)
                    && message.destination() == self.max_distance
                {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSpatialSoundSourceMaxDistance(
                                SetSpatialSoundSourceMaxDistanceCommand::new(handle, value),
                            ),
                        ))
                        .unwrap();
                }
            }
            _ => {}
        }
    }
}

pub struct SoundSection {
    pub section: Handle<UiNode>,
    spatial_section: SpatialSection,
    gain: Handle<UiNode>,
    buffer: Handle<UiNode>,
    name: Handle<UiNode>,
    pitch: Handle<UiNode>,
    looping: Handle<UiNode>,
    play_once: Handle<UiNode>,
}

impl SoundSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let spatial_section = SpatialSection::new(ctx);

        let gain;
        let buffer;
        let name;
        let pitch;
        let looping;
        let play_once;
        let section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(make_text_mark(ctx, "Gain", 0))
                            .with_child({
                                gain = make_f32_input_field(ctx, 0, 0.0, f32::MAX, 0.1);
                                gain
                            })
                            .with_child(make_text_mark(ctx, "Buffer", 1))
                            .with_child({
                                buffer = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .on_column(1)
                                        .with_allow_drop(true)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_editable(false)
                                .with_text("<None>")
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx);
                                buffer
                            })
                            .with_child(make_text_mark(ctx, "Name", 2))
                            .with_child({
                                name = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(2)
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("<None>")
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .build(ctx);
                                name
                            })
                            .with_child(make_text_mark(ctx, "Pitch", 3))
                            .with_child({
                                pitch = make_f32_input_field(ctx, 3, 0.0, f32::MAX, 0.1);
                                pitch
                            })
                            .with_child(make_text_mark(ctx, "Looping", 4))
                            .with_child({
                                looping = make_bool_input_field(ctx, 4);
                                looping
                            })
                            .with_child(make_text_mark(ctx, "Play Once", 5))
                            .with_child({
                                play_once = make_bool_input_field(ctx, 5);
                                play_once
                            }),
                    )
                    .add_column(Column::strict(COLUMN_WIDTH))
                    .add_column(Column::stretch())
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT))
                    .add_row(Row::strict(ROW_HEIGHT))
                    .build(ctx),
                )
                .with_child(spatial_section.section),
        )
        .build(ctx);

        Self {
            section,
            spatial_section,
            gain,
            name,
            pitch,
            buffer,
            looping,
            play_once,
        }
    }

    pub fn sync_to_model(&mut self, source: &SoundSource, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(
                self.spatial_section.section,
                MessageDirection::ToWidget,
                matches!(source, SoundSource::Spatial(_)),
            ),
        );

        if let SoundSource::Spatial(spatial) = source {
            self.spatial_section.sync_to_model(spatial, ui);
        }

        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.pitch,
                MessageDirection::ToWidget,
                source.pitch() as f32,
            ),
        );
        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.gain, MessageDirection::ToWidget, source.gain()),
        );
        send_sync_message(
            ui,
            TextBoxMessage::text(self.name, MessageDirection::ToWidget, source.name_owned()),
        );
        send_sync_message(
            ui,
            TextBoxMessage::text(
                self.buffer,
                MessageDirection::ToWidget,
                source
                    .buffer()
                    .map(|b| {
                        b.data_ref()
                            .external_data_path()
                            .to_string_lossy()
                            .to_string()
                    })
                    .unwrap_or_else(|| "<None>".to_owned()),
            ),
        );
        send_sync_message(
            ui,
            CheckBoxMessage::checked(
                self.play_once,
                MessageDirection::ToWidget,
                Some(source.is_play_once()),
            ),
        );
        send_sync_message(
            ui,
            CheckBoxMessage::checked(
                self.looping,
                MessageDirection::ToWidget,
                Some(source.is_looping()),
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        sender: &Sender<Message>,
        source: &SoundSource,
        handle: Handle<SoundSource>,
        ui: &Ui,
        resource_manager: ResourceManager,
    ) {
        scope_profile!();

        if message.direction() != MessageDirection::FromWidget {
            return;
        }

        if let SoundSource::Spatial(spatial) = source {
            self.spatial_section
                .handle_message(message, sender, spatial, handle);
        }

        match message.data() {
            &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if source.gain().ne(&value) && message.destination() == self.gain {
                    sender
                        .send(Message::DoSceneCommand(SceneCommand::SetSoundSourceGain(
                            SetSoundSourceGainCommand::new(handle, value),
                        )))
                        .unwrap();
                } else if (source.pitch() as f32).ne(&value) && message.destination() == self.pitch
                {
                    sender
                        .send(Message::DoSceneCommand(SceneCommand::SetSoundSourcePitch(
                            SetSoundSourcePitchCommand::new(handle, value as f64),
                        )))
                        .unwrap();
                }
            }
            &UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) => {
                if source.is_play_once() != value && message.destination() == self.play_once {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSoundSourcePlayOnce(
                                SetSoundSourcePlayOnceCommand::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if source.is_looping() != value && message.destination() == self.looping {
                    sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetSoundSourceLooping(SetSoundSourceLoopingCommand::new(
                                handle, value,
                            )),
                        ))
                        .unwrap();
                }
            }
            UiMessageData::TextBox(TextBoxMessage::Text(text)) => {
                if message.destination() == self.name && source.name() != text {
                    sender
                        .send(Message::DoSceneCommand(SceneCommand::SetSoundSourceName(
                            SetSoundSourceNameCommand::new(handle, text.clone()),
                        )))
                        .unwrap();
                }
            }
            UiMessageData::Widget(WidgetMessage::Drop(dropped)) => {
                if message.destination() == self.buffer {
                    // Set buffer.
                    if let UiNode::User(EditorUiNode::AssetItem(item)) = ui.node(*dropped) {
                        // Make sure all resources loaded with relative paths only.
                        // This will make scenes portable.
                        let relative_path = make_relative_path(&item.path);

                        if item.kind == AssetKind::Sound {
                            if let Ok(buffer) = block_on(
                                resource_manager.request_sound_buffer(&relative_path, false),
                            ) {
                                sender
                                    .send(Message::DoSceneCommand(
                                        SceneCommand::SetSoundSourceBuffer(
                                            SetSoundSourceBufferCommand::new(handle, Some(buffer)),
                                        ),
                                    ))
                                    .unwrap();
                            } else {
                                sender
                                    .send(Message::Log(format!(
                                        "Unable to load sound buffer {}!",
                                        relative_path.display()
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
