// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        graph::SceneGraph,
        gui::{
            BuildContext, Thickness, UserInterface, VerticalAlignment,
            button::{Button, ButtonBuilder, ButtonMessage},
            check_box::{CheckBox, CheckBoxBuilder, CheckBoxMessage},
            grid::{Column, Grid, GridBuilder, Row},
            message::UiMessage,
            scroll_bar::{ScrollBar, ScrollBarBuilder, ScrollBarMessage},
            text::TextBuilder,
            widget::{WidgetBuilder, WidgetMessage},
        },
        scene::{
            node::Node,
            sound::{Sound, Status},
        },
    },
    game_scene::{GameScene, Selection},
};
use fyrox::core::uuid::{Uuid, uuid};

pub struct AudioPreviewPanel {
    pub root_widget: Handle<Grid>,
    preview: Handle<CheckBox>,
    play: Handle<Button>,
    pause: Handle<Button>,
    stop: Handle<Button>,
    rewind: Handle<Button>,
    time: Handle<ScrollBar>,
    sounds_state: Vec<(Handle<Node>, Node)>,
}

fn make_button(text: &str, column: usize, uuid: Uuid, ctx: &mut BuildContext) -> Handle<Button> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_id(uuid)
            .on_column(column)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_text(text)
    .build(ctx)
}

impl AudioPreviewPanel {
    pub const PREVIEW: Uuid = uuid!("0bf43ccf-bcdb-4d4d-8522-2098da6bc393");
    pub const PLAY: Uuid = uuid!("d32cf9fa-69f2-4340-b81b-5eae20fe5124");
    pub const STOP: Uuid = uuid!("466bc0de-4a95-43ce-add3-854b8fc09186");
    pub const REWIND: Uuid = uuid!("155b6de0-1b2d-4254-a9f9-8c8d14e26dbf");
    pub const PAUSE: Uuid = uuid!("b28b99ad-60a8-452a-afd1-1add8377cc70");

    pub fn new(ctx: &mut BuildContext) -> Self {
        let preview;
        let play;
        let pause;
        let stop;
        let rewind;
        let time;
        let root_widget = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .with_child({
                                preview = CheckBoxBuilder::new(
                                    WidgetBuilder::new()
                                        .with_id(Self::PREVIEW)
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
                                play = make_button("Play", 1, Self::PLAY, ctx);
                                play
                            })
                            .with_child({
                                pause = make_button("Pause", 2, Self::PAUSE, ctx);
                                pause
                            })
                            .with_child({
                                stop = make_button("Stop", 3, Self::STOP, ctx);
                                stop
                            })
                            .with_child({
                                rewind = make_button("Rewind", 4, Self::REWIND, ctx);
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
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text("Time, s")
                                .build(ctx),
                            )
                            .with_child({
                                time = ScrollBarBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .show_value(true)
                                .with_value_precision(2)
                                .with_min(0.0)
                                .build(ctx);
                                time
                            }),
                    )
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_row(Row::strict(26.0))
                    .build(ctx),
                ),
        )
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .add_row(Row::strict(26.0))
        .build(ctx);

        Self {
            root_widget,
            preview,
            play,
            pause,
            stop,
            rewind,
            time,
            sounds_state: vec![],
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.root_widget, WidgetMessage::Remove);
    }

    fn set_buttons_enabled(&self, ui: &UserInterface, enabled: bool) {
        for button in [self.play, self.pause, self.rewind, self.stop] {
            ui.send(button, WidgetMessage::Enabled(enabled));
        }
    }

    fn enter_preview_mode(
        &mut self,
        editor_selection: &Selection,
        game_scene: &mut GameScene,
        engine: &mut Engine,
    ) {
        assert!(self.sounds_state.is_empty());

        let ui = engine.user_interfaces.first();
        let scene = &engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        let mut set = false;
        if let Some(new_graph_selection) = editor_selection.as_graph() {
            for &node_handle in &new_graph_selection.nodes {
                if let Ok(sound) = scene.graph.try_get_of_type::<Sound>(node_handle) {
                    if !set {
                        if let Some(buffer) = sound.buffer() {
                            let mut state = buffer.state();
                            if let Some(buffer) = state.data() {
                                let duration_secs = buffer.duration().as_secs_f32();
                                ui.send_sync_many(
                                    self.time,
                                    [
                                        ScrollBarMessage::MaxValue(duration_secs),
                                        ScrollBarMessage::Value(
                                            sound.playback_time().clamp(0.0, duration_secs),
                                        ),
                                    ],
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

        self.set_buttons_enabled(ui, true);
    }

    pub fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        for (sound_handle, original) in self.sounds_state.drain(..) {
            scene.graph[sound_handle] = original;

            assert!(node_overrides.remove(&sound_handle));
        }
        let ui = engine.user_interfaces.first();
        ui.send_sync(self.preview, CheckBoxMessage::Check(Some(false)));

        scene.graph.sound_context.state().destroy_sound_sources();

        self.set_buttons_enabled(ui, false);
    }

    pub fn is_in_preview_mode(&self) -> bool {
        !self.sounds_state.is_empty()
    }

    pub fn update(&self, editor_selection: &Selection, game_scene: &GameScene, engine: &Engine) {
        let scene = &engine.scenes[game_scene.scene];
        if let Some(new_graph_selection) = editor_selection.as_graph() {
            for &node_handle in &new_graph_selection.nodes {
                if let Ok(sound) = scene.graph.try_get_of_type::<Sound>(node_handle)
                    && sound.status() == Status::Playing
                {
                    engine
                        .user_interfaces
                        .first()
                        .send_sync(self.time, ScrollBarMessage::Value(sound.playback_time()));
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
                    if let Ok(sound) = scene.graph.try_get_mut_of_type::<Sound>(node) {
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
            } else if let Some(CheckBoxMessage::Check(Some(value))) =
                message.data_from(self.preview)
            {
                if *value {
                    self.enter_preview_mode(editor_selection, game_scene, engine);
                } else {
                    self.leave_preview_mode(game_scene, engine);
                }
            } else if let Some(msg) = message.data_from::<ScrollBarMessage>(self.time) {
                let scene = &mut engine.scenes[game_scene.scene];

                for &node in &selection.nodes {
                    if let Ok(sound) = scene.graph.try_get_mut_of_type::<Sound>(node) {
                        match msg {
                            ScrollBarMessage::Value(playback_position) => {
                                sound.set_playback_time(*playback_position);
                            }
                            ScrollBarMessage::StartDragging => {
                                sound.pause();
                            }
                            ScrollBarMessage::EndDragging => {
                                sound.play();
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
    }
}
