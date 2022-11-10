use fyrox::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder, button::ButtonBuilder, numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder, text::TextBuilder, widget::WidgetBuilder, BuildContext,
        Orientation, Thickness, UiNode, VerticalAlignment, BRUSH_LIGHT,
    },
};

pub struct Toolbar {
    pub panel: Handle<UiNode>,
    pub play_pause: Handle<UiNode>,
    pub stop: Handle<UiNode>,
    pub speed: Handle<UiNode>,
}

impl Toolbar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let play_pause;
        let stop;
        let speed;
        let panel = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_foreground(BRUSH_LIGHT)
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(1.0))
                            .with_child({
                                play_pause = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Play/Pause")
                                .build(ctx);
                                play_pause
                            })
                            .with_child({
                                stop = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Stop")
                                .build(ctx);
                                stop
                            })
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new()
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness {
                                            left: 10.0,
                                            top: 1.0,
                                            right: 1.0,
                                            bottom: 1.0,
                                        }),
                                )
                                .with_text("Playback Speed")
                                .build(ctx),
                            )
                            .with_child({
                                speed = NumericUpDownBuilder::<f32>::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .build(ctx);
                                speed
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                ),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        Self {
            panel,
            play_pause,
            stop,
            speed,
        }
    }
}
