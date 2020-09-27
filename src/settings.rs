use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    GameEngine, Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, CheckBoxMessage, ColorFieldMessage, MessageDirection, UiMessageData,
            WindowMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

pub struct Settings {
    pub window: Handle<UiNode>,
    ssao: Handle<UiNode>,
    point_shadows: Handle<UiNode>,
    spot_shadows: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    sender: Sender<Message>,
    ambient_color: Handle<UiNode>,
    light_scatter: Handle<UiNode>,
}

fn make_text_mark(ctx: &mut BuildContext, text: &str, row: usize) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(Thickness::left(4.0))
            .on_row(row)
            .on_column(0),
    )
    .with_text(text)
    .build(ctx)
}

fn make_bool_input_field(ctx: &mut BuildContext, row: usize, value: bool) -> Handle<UiNode> {
    CheckBoxBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .checked(Some(value))
    .build(ctx)
}

impl Settings {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>) -> Self {
        let ssao;
        let ok;
        let default;
        let ambient_color;
        let point_shadows;
        let spot_shadows;
        let light_scatter;
        let ctx = &mut engine.user_interface.build_ctx();
        let settings = engine.renderer.get_quality_settings();
        Self {
            window: WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Settings".to_owned()))
                .with_content(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(TextBuilder::new(WidgetBuilder::new().on_row(0).with_margin(Thickness::uniform(1.0)))
                                .with_text("Here you can select graphics settings to improve performance and/or to understand how \
                                you scene will look like with different graphics settings. Please note that these settings won't be saved \
                                with scene!")
                                .with_wrap(true)
                                .build(ctx))
                            .with_child(
                                GridBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(1)
                                        .with_child(make_text_mark(ctx, "SSAO", 0))
                                        .with_child({
                                            ssao = make_bool_input_field(ctx, 0, settings.use_ssao);
                                            ssao
                                        })
                                        .with_child(make_text_mark(ctx, "Ambient Color", 1))
                                        .with_child( {
                                            ambient_color = ColorFieldBuilder::new(WidgetBuilder::new().on_column(1).on_row(1)).build(ctx);
                                            ambient_color
                                        })
                                        .with_child(make_text_mark(ctx, "Point Shadows", 2))
                                        .with_child({
                                            point_shadows = make_bool_input_field(ctx, 2, settings.point_shadows_enabled);
                                            point_shadows
                                        })
                                        .with_child(make_text_mark(ctx, "Spot Shadows", 3))
                                        .with_child({
                                            spot_shadows = make_bool_input_field(ctx, 3, settings.spot_shadows_enabled);
                                            spot_shadows
                                        })
                                        .with_child(make_text_mark(ctx, "Light Scatter", 4))
                                        .with_child({
                                            light_scatter = make_bool_input_field(ctx, 4, settings.light_scatter_enabled);
                                            light_scatter
                                        }),
                                )
                                .add_row(Row::strict(25.0))
                                .add_row(Row::strict(25.0))
                                .add_row(Row::strict(25.0))
                                .add_row(Row::strict(25.0))
                                .add_row(Row::strict(25.0))
                                .add_row(Row::stretch())
                                .add_row(Row::stretch())
                                .add_column(Column::strict(100.0))
                                .add_column(Column::stretch())
                                .build(ctx),
                            )
                            .with_child(
                                StackPanelBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(2)
                                        .with_horizontal_alignment(HorizontalAlignment::Right)
                                        .with_child({
                                            default = ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(80.0)
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_text("Default")
                                            .build(ctx);
                                            default
                                        })
                                        .with_child({
                                            ok = ButtonBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_width(80.0)
                                                    .with_margin(Thickness::uniform(1.0)),
                                            )
                                            .with_text("OK")
                                            .build(ctx);
                                            ok
                                        }),
                                )
                                .with_orientation(Orientation::Horizontal)
                                .build(ctx),
                            ),
                    )
                        .add_row(Row::auto())
                    .add_row(Row::stretch())
                    .add_row(Row::strict(25.0))
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build(ctx),
            ssao,
            sender,
            ok,
            default,
            ambient_color,
            point_shadows,
            spot_shadows,
            light_scatter
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        let mut settings = engine.renderer.get_quality_settings();

        match message.data() {
            UiMessageData::CheckBox(msg) => {
                if let CheckBoxMessage::Check(check) = msg {
                    let value = check.unwrap_or(false);
                    if message.destination() == self.ssao {
                        settings.use_ssao = value;
                    } else if message.destination() == self.point_shadows {
                        settings.point_shadows_enabled = value;
                    } else if message.destination() == self.spot_shadows {
                        settings.spot_shadows_enabled = value;
                    } else if message.destination() == self.light_scatter {
                        settings.light_scatter_enabled = value;
                    }
                }
            }
            UiMessageData::ColorField(msg)
                if message.direction() == MessageDirection::FromWidget =>
            {
                if message.destination() == self.ambient_color {
                    if let &ColorFieldMessage::Color(color) = msg {
                        engine.renderer.set_ambient_color(color);
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination() == self.ok {
                        engine.user_interface.send_message(WindowMessage::close(
                            self.window,
                            MessageDirection::ToWidget,
                        ));
                    } else if message.destination() == self.default {
                        settings = Default::default();

                        let sync_check_box = |handle: Handle<UiNode>, value: bool| {
                            engine.user_interface.send_message(CheckBoxMessage::checked(
                                handle,
                                MessageDirection::ToWidget,
                                Some(value),
                            ));
                        };

                        sync_check_box(self.ssao, settings.use_ssao);
                        sync_check_box(self.point_shadows, settings.point_shadows_enabled);
                        sync_check_box(self.spot_shadows, settings.spot_shadows_enabled);
                        sync_check_box(self.light_scatter, settings.light_scatter_enabled);
                    }
                }
            }
            _ => {}
        }

        if settings != engine.renderer.get_quality_settings() {
            if let Err(e) = engine.renderer.set_quality_settings(&settings) {
                self.sender
                    .send(Message::Log(format!(
                        "An error occurred at attempt to set new graphics settings: {:?}",
                        e
                    )))
                    .unwrap();
            } else {
                self.sender
                    .send(Message::Log(
                        "New graphics quality settings were successfully set!".to_owned(),
                    ))
                    .unwrap();
            }
        }
    }
}
