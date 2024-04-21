use crate::fyrox::graph::SceneGraph;
use crate::fyrox::gui::HorizontalAlignment;
use crate::fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_bar::{ScrollBarBuilder, ScrollBarMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, VerticalAlignment,
    },
    scene::{
        node::Node,
        sound::{Sound, Status},
    },
};
use crate::{
    scene::{GameScene, Selection},
    send_sync_message, Message,
};

pub struct AudioPreviewPanel {
    pub window: Handle<UiNode>,
    preview: Handle<UiNode>,
    play: Handle<UiNode>,
    pause: Handle<UiNode>,
    stop: Handle<UiNode>,
    rewind: Handle<UiNode>,
    time: Handle<UiNode>,
    sounds_state: Vec<(Handle<Node>, Node)>,
    scene_viewer_frame: Handle<UiNode>,
}

impl AudioPreviewPanel {
    pub fn new(scene_viewer_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let preview;
        let play;
        let pause;
        let stop;
        let rewind;
        let time;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("AudioPreviewPanel")
                .with_width(300.0)
                .with_height(70.0),
        )
        .with_title(WindowTitle::text("Audio Preview Panel"))
        .open(false)
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .on_row(0)
                                .with_child({
                                    preview = CheckBoxBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_content(
                                        TextBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(0)
                                                .with_vertical_alignment(VerticalAlignment::Center),
                                        )
                                        .with_text("Preview")
                                        .build(ctx),
                                    )
                                    .build(ctx);
                                    preview
                                })
                                .with_child({
                                    play = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Play")
                                    .build(ctx);
                                    play
                                })
                                .with_child({
                                    pause = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(2)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Pause")
                                    .build(ctx);
                                    pause
                                })
                                .with_child({
                                    stop = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(3)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Stop")
                                    .build(ctx);
                                    stop
                                })
                                .with_child({
                                    rewind = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_column(4)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Rewind")
                                    .build(ctx);
                                    rewind
                                }),
                        )
                        .add_row(Row::stretch())
                        .add_column(Column::strict(80.0))
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .add_column(Column::stretch())
                        .build(ctx),
                    )
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .on_row(1)
                                .with_child(
                                    TextBuilder::new(
                                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Time, s")
                                    .build(ctx),
                                )
                                .with_child({
                                    time = ScrollBarBuilder::new(WidgetBuilder::new().on_column(1))
                                        .with_min(0.0)
                                        .build(ctx);
                                    time
                                }),
                        )
                        .add_column(Column::auto())
                        .add_column(Column::stretch())
                        .add_row(Row::strict(20.0))
                        .build(ctx),
                    ),
            )
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .add_row(Row::strict(20.0))
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            preview,
            play,
            pause,
            stop,
            rewind,
            time,
            sounds_state: vec![],
            scene_viewer_frame,
        }
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if let Message::DoCommand(_)
        | Message::UndoCurrentSceneCommand
        | Message::RedoCurrentSceneCommand = message
        {
            self.leave_preview_mode(game_scene, engine);
        }

        if let Message::SelectionChanged { .. } = message {
            let scene = &engine.scenes[game_scene.scene];
            if let Some(selection) = editor_selection.as_graph() {
                let any_sound_selected = selection
                    .nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<Sound>(*n).is_some());
                if any_sound_selected {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::open_and_align(
                            self.window,
                            MessageDirection::ToWidget,
                            self.scene_viewer_frame,
                            HorizontalAlignment::Right,
                            VerticalAlignment::Top,
                            Thickness::top_right(5.0),
                            false,
                            false,
                        ));
                } else {
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(WindowMessage::close(
                            self.window,
                            MessageDirection::ToWidget,
                        ));
                }
            }
        }
    }

    fn enter_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        assert!(self.sounds_state.is_empty());

        let scene = &engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        let mut set = false;
        if let Some(new_graph_selection) = editor_selection.as_graph() {
            for &node_handle in &new_graph_selection.nodes {
                if let Some(sound) = scene.graph.try_get_of_type::<Sound>(node_handle) {
                    if !set {
                        if let Some(buffer) = sound.buffer() {
                            let mut state = buffer.state();
                            if let Some(buffer) = state.data() {
                                let duration_secs = buffer.duration().as_secs_f32();

                                send_sync_message(
                                    engine.user_interfaces.first(),
                                    ScrollBarMessage::max_value(
                                        self.time,
                                        MessageDirection::ToWidget,
                                        duration_secs,
                                    ),
                                );

                                send_sync_message(
                                    engine.user_interfaces.first(),
                                    ScrollBarMessage::value(
                                        self.time,
                                        MessageDirection::ToWidget,
                                        sound.playback_time().clamp(0.0, duration_secs),
                                    ),
                                );
                            }
                        }

                        set = true;
                    }

                    self.sounds_state
                        .push((node_handle, scene.graph[node_handle].clone_box()));

                    assert!(node_overrides.insert(node_handle));
                }
            }
        }
    }

    pub fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        for (sound_handle, original) in self.sounds_state.drain(..) {
            scene.graph[sound_handle] = original;

            assert!(node_overrides.remove(&sound_handle));
        }

        send_sync_message(
            engine.user_interfaces.first(),
            CheckBoxMessage::checked(self.preview, MessageDirection::ToWidget, Some(false)),
        );

        scene.graph.sound_context.state().destroy_sound_sources();
    }

    pub fn is_in_preview_mode(&self) -> bool {
        !self.sounds_state.is_empty()
    }

    pub fn update(&self, editor_selection: &Selection, game_scene: &GameScene, engine: &Engine) {
        let scene = &engine.scenes[game_scene.scene];
        if let Some(new_graph_selection) = editor_selection.as_graph() {
            for &node_handle in &new_graph_selection.nodes {
                if let Some(sound) = scene.graph.try_get_of_type::<Sound>(node_handle) {
                    send_sync_message(
                        engine.user_interfaces.first(),
                        ScrollBarMessage::value(
                            self.time,
                            MessageDirection::ToWidget,
                            sound.playback_time(),
                        ),
                    );

                    break;
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        if let Some(selection) = editor_selection.as_graph() {
            if let Some(ButtonMessage::Click) = message.data() {
                let scene = &mut engine.scenes[game_scene.scene];

                for &node in &selection.nodes {
                    if let Some(sound) = scene.graph.try_get_mut_of_type::<Sound>(node) {
                        if message.destination() == self.play {
                            sound.set_status(Status::Playing);
                        } else if message.destination() == self.pause {
                            sound.set_status(Status::Paused);
                        } else if message.destination() == self.stop {
                            sound.set_status(Status::Stopped);
                        } else if message.destination() == self.rewind {
                            sound.set_playback_time(0.0);
                        }
                    }
                }
            } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data() {
                if message.destination() == self.preview
                    && message.direction() == MessageDirection::FromWidget
                {
                    if *value {
                        self.enter_preview_mode(editor_selection, game_scene, engine);
                    } else {
                        self.leave_preview_mode(game_scene, engine);
                    }
                }
            } else if let Some(ScrollBarMessage::Value(playback_position)) = message.data() {
                if message.destination() == self.time
                    && message.direction() == MessageDirection::FromWidget
                {
                    let scene = &mut engine.scenes[game_scene.scene];

                    for &node in &selection.nodes {
                        if let Some(sound) = scene.graph.try_get_mut_of_type::<Sound>(node) {
                            sound.set_playback_time(*playback_position);
                        }
                    }
                }
            }
        }
    }
}
