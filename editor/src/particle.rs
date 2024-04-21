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
        numeric::{NumericUpDownBuilder, NumericUpDownMessage},
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, Thickness, UiNode, VerticalAlignment,
    },
    scene::{node::Node, particle_system::ParticleSystem},
};
use crate::{
    scene::{GameScene, Selection},
    send_sync_message, Message, FIXED_TIMESTEP,
};

pub struct ParticleSystemPreviewControlPanel {
    pub window: Handle<UiNode>,
    preview: Handle<UiNode>,
    play: Handle<UiNode>,
    pause: Handle<UiNode>,
    stop: Handle<UiNode>,
    rewind: Handle<UiNode>,
    time: Handle<UiNode>,
    set_time: Handle<UiNode>,
    particle_systems_state: Vec<(Handle<Node>, Node)>,
    desired_playback_time: f32,
    scene_viewer_frame: Handle<UiNode>,
}

impl ParticleSystemPreviewControlPanel {
    pub fn new(scene_viewer_frame: Handle<UiNode>, ctx: &mut BuildContext) -> Self {
        let preview;
        let play;
        let pause;
        let stop;
        let rewind;

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    preview = CheckBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_content(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_vertical_alignment(VerticalAlignment::Center)
                                .with_margin(Thickness::uniform(1.0)),
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
                            .on_row(0)
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
                            .on_row(0)
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
                            .on_row(0)
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
                            .on_row(0)
                            .on_column(4)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text("Rewind")
                    .build(ctx);
                    rewind
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let time;
        let set_time;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_name("ParticleSystemPanel")
                .with_width(300.0)
                .with_height(70.0),
        )
        .open(false)
        .with_title(WindowTitle::text("Particle System"))
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new().with_child(grid).with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .on_column(0)
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Playback Time")
                                .build(ctx),
                            )
                            .with_child({
                                time = NumericUpDownBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_min_value(0.0f32)
                                .with_max_value(10.0 * 60.0) // 10 Minutes
                                .with_value(0.0f32)
                                .build(ctx);
                                time
                            })
                            .with_child({
                                set_time = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(2)
                                        .with_width(33.0)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Set")
                                .build(ctx);
                                set_time
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .add_column(Column::auto())
                    .build(ctx),
                ),
            )
            .add_row(Row::stretch())
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            play,
            pause,
            stop,
            rewind,
            time,
            preview,
            particle_systems_state: Default::default(),
            set_time,
            desired_playback_time: 0.0,
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
                let any_particle_system_selected = selection
                    .nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<ParticleSystem>(*n).is_some());
                if any_particle_system_selected {
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
        assert!(self.particle_systems_state.is_empty());

        let scene = &engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        if let Some(new_graph_selection) = editor_selection.as_graph() {
            // Enable particle systems from new selection.
            for &node_handle in &new_graph_selection.nodes {
                if scene
                    .graph
                    .try_get_of_type::<ParticleSystem>(node_handle)
                    .is_some()
                {
                    self.particle_systems_state
                        .push((node_handle, scene.graph[node_handle].clone_box()));

                    assert!(node_overrides.insert(node_handle));
                }
            }
        }
    }

    pub fn leave_preview_mode(&mut self, game_scene: &mut GameScene, engine: &mut Engine) {
        let scene = &mut engine.scenes[game_scene.scene];
        let node_overrides = game_scene.graph_switches.node_overrides.as_mut().unwrap();

        for (particle_system_handle, original) in self.particle_systems_state.drain(..) {
            scene.graph[particle_system_handle] = original;

            assert!(node_overrides.remove(&particle_system_handle));
        }

        send_sync_message(
            engine.user_interfaces.first(),
            CheckBoxMessage::checked(self.preview, MessageDirection::ToWidget, Some(false)),
        );
    }

    pub fn is_in_preview_mode(&self) -> bool {
        !self.particle_systems_state.is_empty()
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
                    if let Some(particle_system) =
                        scene.graph.try_get_mut_of_type::<ParticleSystem>(node)
                    {
                        if message.destination() == self.play {
                            particle_system.play(true);
                        } else if message.destination() == self.pause {
                            particle_system.play(false);
                        } else if message.destination() == self.stop {
                            particle_system.play(false);
                            particle_system.clear_particles();
                        } else if message.destination() == self.rewind {
                            particle_system.clear_particles();
                        } else if message.destination() == self.set_time {
                            particle_system.rewind(FIXED_TIMESTEP, self.desired_playback_time);
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
            } else if let Some(NumericUpDownMessage::Value(desired_playback_time)) = message.data()
            {
                if message.destination() == self.time
                    && message.direction() == MessageDirection::FromWidget
                {
                    self.desired_playback_time = *desired_playback_time;
                }
            }
        }
    }
}
