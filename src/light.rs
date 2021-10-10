use crate::{scene::EditorScene, GameEngine};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::UiNode;
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::ButtonMessage,
        message::{MessageDirection, UiMessageData},
        numeric::NumericUpDownBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness, VerticalAlignment,
    },
    utils::lightmap::Lightmap,
};

pub struct LightPanel {
    pub window: Handle<UiNode>,
    nud_texels_per_unit: Handle<UiNode>,
    nud_spacing: Handle<UiNode>,
    generate: Handle<UiNode>,
    texels_per_unit: u32,
    spacing: f32,
}

impl LightPanel {
    pub fn new(engine: &mut GameEngine) -> Self {
        let generate;
        let nud_texels_per_unit;
        let nud_spacing;
        let ctx = &mut engine.user_interface.build_ctx();
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_title(WindowTitle::Text("Light Settings".to_owned()))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(0)
                                    .with_vertical_alignment(VerticalAlignment::Center),
                            )
                            .with_text("Texels Per Unit")
                            .build(ctx),
                        )
                        .with_child({
                            nud_texels_per_unit = NumericUpDownBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_min_value(16.0)
                            .with_max_value(256.0)
                            .with_step(4.0)
                            .with_precision(0)
                            .with_value(128.0)
                            .build(ctx);
                            nud_texels_per_unit
                        })
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(0)
                                    .with_vertical_alignment(VerticalAlignment::Center),
                            )
                            .with_text("Spacing")
                            .build(ctx),
                        )
                        .with_child({
                            nud_spacing = NumericUpDownBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_min_value(0.0)
                            .with_max_value(0.1)
                            .with_step(0.001)
                            .with_precision(3)
                            .with_value(0.02)
                            .build(ctx);
                            nud_spacing
                        })
                        .with_child({
                            generate = ButtonBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text("Generate Lightmap")
                            .build(ctx);
                            generate
                        }),
                )
                .add_column(Column::strict(100.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            generate,
            nud_texels_per_unit,
            texels_per_unit: 128,
            nud_spacing,
            spacing: 0.02,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.generate {
                    let scene = &mut engine.scenes[editor_scene.scene];

                    let lightmap = Lightmap::new(
                        scene,
                        self.texels_per_unit,
                        Default::default(),
                        Default::default(),
                    )
                    .unwrap();
                    lightmap
                        .save("./", engine.resource_manager.clone())
                        .unwrap();
                    scene.set_lightmap(lightmap).unwrap();
                }
            }
            UiMessageData::User(msg) if message.direction() == MessageDirection::FromWidget => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if message.destination() == self.nud_texels_per_unit {
                        self.texels_per_unit = value as u32;
                    } else if message.destination() == self.nud_spacing {
                        self.spacing = value;
                    }
                }
            }
            _ => {}
        }
    }
}
