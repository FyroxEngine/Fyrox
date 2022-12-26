use crate::{
    scene::{EditorScene, Selection},
    Message,
};
use fyrox::{
    core::pool::Handle,
    engine::Engine,
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode,
    },
    scene::particle_system::ParticleSystem,
};

pub struct ParticleSystemPreviewControlPanel {
    pub window: Handle<UiNode>,
    play: Handle<UiNode>,
    pause: Handle<UiNode>,
    stop: Handle<UiNode>,
}

impl ParticleSystemPreviewControlPanel {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let play;
        let pause;
        let stop;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .open(false)
            .with_title(WindowTitle::text("Particle System"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            play = ButtonBuilder::new(
                                WidgetBuilder::new().on_row(0).on_column(0).with_width(80.0),
                            )
                            .with_text("Play")
                            .build(ctx);
                            play
                        })
                        .with_child({
                            pause = ButtonBuilder::new(
                                WidgetBuilder::new().on_row(0).on_column(1).with_width(80.0),
                            )
                            .with_text("Pause")
                            .build(ctx);
                            pause
                        })
                        .with_child({
                            stop = ButtonBuilder::new(
                                WidgetBuilder::new().on_row(0).on_column(2).with_width(80.0),
                            )
                            .with_text("Stop")
                            .build(ctx);
                            stop
                        }),
                )
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            play,
            pause,
            stop,
        }
    }

    pub fn handle_message(
        &self,
        message: &Message,
        editor_scene: &EditorScene,
        engine: &mut Engine,
    ) {
        if let Message::SelectionChanged { .. } = message {
            let scene = &engine.scenes[editor_scene.scene];
            if let Selection::Graph(ref selection) = editor_scene.selection {
                let any_particle_system_selected = selection
                    .nodes
                    .iter()
                    .any(|n| scene.graph.try_get_of_type::<ParticleSystem>(*n).is_some());
                if any_particle_system_selected {
                    engine.user_interface.send_message(WindowMessage::open(
                        self.window,
                        MessageDirection::ToWidget,
                        false,
                    ));
                } else {
                    engine.user_interface.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                }
            }
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut Engine,
    ) {
        if let Selection::Graph(ref selection) = editor_scene.selection {
            if let Some(ButtonMessage::Click) = message.data() {
                let scene = &mut engine.scenes[editor_scene.scene];

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
                        }
                    }
                }
            }
        }
    }
}
