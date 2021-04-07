use crate::STARTUP_WORKING_DIR;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::EditorScene,
    GameEngine, Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        button::ButtonBuilder,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, CheckBoxMessage, ColorFieldMessage, MessageDirection,
            NumericUpDownMessage, UiMessageData, WindowMessage,
        },
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    renderer::QualitySettings,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs::File, sync::mpsc::Sender};

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ssao: Handle<UiNode>,
    point_shadows: Handle<UiNode>,
    spot_shadows: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    sender: Sender<Message>,
    ambient_color: Handle<UiNode>,
    light_scatter: Handle<UiNode>,
    near_plane: Handle<UiNode>,
    far_plane: Handle<UiNode>,
    parallax_mapping: Handle<UiNode>,
    show_physics: Handle<UiNode>,
    show_bounds: Handle<UiNode>,
    show_tbn: Handle<UiNode>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct Settings {
    pub graphics: QualitySettings,
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
            z_near: 0.025,
            z_far: 128.0,
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Ron(ron::Error),
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ron::Error> for SettingsError {
    fn from(e: ron::Error) -> Self {
        Self::Ron(e)
    }
}

impl Settings {
    const FILE_NAME: &'static str = "settings.ron";

    fn full_path() -> PathBuf {
        STARTUP_WORKING_DIR.lock().unwrap().join(Self::FILE_NAME)
    }

    pub fn load() -> Result<Self, SettingsError> {
        let file = File::open(Self::full_path())?;
        Ok(ron::de::from_reader(file)?)
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        let file = File::create(Self::full_path())?;
        ron::ser::to_writer_pretty(file, self, PrettyConfig::default())?;
        Ok(())
    }
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

impl SettingsWindow {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>, settings: &Settings) -> Self {
        let ssao;
        let ok;
        let default;
        let ambient_color;
        let point_shadows;
        let spot_shadows;
        let light_scatter;
        let near_plane;
        let far_plane;
        let parallax_mapping;
        let show_physics;
        let show_bounds;
        let show_tbn;
        let ctx = &mut engine.user_interface.build_ctx();
        let text =
            "Here you can select graphics settings to improve performance and/or to understand how \
            you scene will look like with different graphics settings. Please note that these settings won't be saved \
            with scene!";
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .open(false)
            .with_title(WindowTitle::Text("Settings".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text(text)
                            .with_wrap(true)
                            .build(ctx),
                        )
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_child(make_text_mark(ctx, "SSAO", 0))
                                    .with_child({
                                        ssao = make_bool_input_field(
                                            ctx,
                                            0,
                                            settings.graphics.use_ssao,
                                        );
                                        ssao
                                    })
                                    .with_child(make_text_mark(ctx, "Ambient Color", 1))
                                    .with_child({
                                        ambient_color = ColorFieldBuilder::new(
                                            WidgetBuilder::new().on_column(1).on_row(1),
                                        )
                                        .build(ctx);
                                        ambient_color
                                    })
                                    .with_child(make_text_mark(ctx, "Point Shadows", 2))
                                    .with_child({
                                        point_shadows = make_bool_input_field(
                                            ctx,
                                            2,
                                            settings.graphics.point_shadows_enabled,
                                        );
                                        point_shadows
                                    })
                                    .with_child(make_text_mark(ctx, "Spot Shadows", 3))
                                    .with_child({
                                        spot_shadows = make_bool_input_field(
                                            ctx,
                                            3,
                                            settings.graphics.spot_shadows_enabled,
                                        );
                                        spot_shadows
                                    })
                                    .with_child(make_text_mark(ctx, "Light Scatter", 4))
                                    .with_child({
                                        light_scatter = make_bool_input_field(
                                            ctx,
                                            4,
                                            settings.graphics.light_scatter_enabled,
                                        );
                                        light_scatter
                                    })
                                    .with_child(make_text_mark(ctx, "Near Plane", 5))
                                    .with_child({
                                        near_plane = NumericUpDownBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(1)
                                                .on_row(5)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_value(settings.z_near)
                                        .with_min_value(0.0)
                                        .build(ctx);
                                        near_plane
                                    })
                                    .with_child(make_text_mark(ctx, "Far Plane", 6))
                                    .with_child({
                                        far_plane = NumericUpDownBuilder::new(
                                            WidgetBuilder::new()
                                                .on_column(1)
                                                .on_row(6)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_min_value(0.0)
                                        .with_value(settings.z_far)
                                        .build(ctx);
                                        far_plane
                                    })
                                    .with_child(make_text_mark(ctx, "Parallax Mapping", 7))
                                    .with_child({
                                        parallax_mapping = make_bool_input_field(
                                            ctx,
                                            7,
                                            settings.graphics.use_parallax_mapping,
                                        );
                                        parallax_mapping
                                    })
                                    .with_child(make_text_mark(ctx, "Show Physics", 8))
                                    .with_child({
                                        show_physics =
                                            make_bool_input_field(ctx, 8, settings.show_physics);
                                        show_physics
                                    })
                                    .with_child(make_text_mark(ctx, "Show Bounds", 9))
                                    .with_child({
                                        show_bounds =
                                            make_bool_input_field(ctx, 9, settings.show_bounds);
                                        show_bounds
                                    })
                                    .with_child(make_text_mark(ctx, "Show TBN", 10))
                                    .with_child({
                                        show_tbn =
                                            make_bool_input_field(ctx, 10, settings.show_tbn);
                                        show_tbn
                                    }),
                            )
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::strict(25.0))
                            .add_row(Row::stretch())
                            .add_row(Row::stretch())
                            .add_column(Column::strict(120.0))
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
            .build(ctx);

        Self {
            window,
            ssao,
            sender,
            ok,
            default,
            ambient_color,
            point_shadows,
            spot_shadows,
            light_scatter,
            near_plane,
            far_plane,
            parallax_mapping,
            show_physics,
            show_bounds,
            show_tbn,
        }
    }

    pub fn open(&self, ui: &Ui, settings: &Settings) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        self.sync_to_model(ui, settings);
    }

    fn sync_to_model(&self, ui: &Ui, settings: &Settings) {
        ui.send_message(NumericUpDownMessage::value(
            self.near_plane,
            MessageDirection::ToWidget,
            settings.z_near,
        ));
        ui.send_message(NumericUpDownMessage::value(
            self.far_plane,
            MessageDirection::ToWidget,
            settings.z_far,
        ));

        let sync_check_box = |handle: Handle<UiNode>, value: bool| {
            ui.send_message(CheckBoxMessage::checked(
                handle,
                MessageDirection::ToWidget,
                Some(value),
            ));
        };

        sync_check_box(self.ssao, settings.graphics.use_ssao);
        sync_check_box(self.point_shadows, settings.graphics.point_shadows_enabled);
        sync_check_box(self.spot_shadows, settings.graphics.spot_shadows_enabled);
        sync_check_box(self.light_scatter, settings.graphics.light_scatter_enabled);
        sync_check_box(
            self.parallax_mapping,
            settings.graphics.use_parallax_mapping,
        );
        sync_check_box(self.show_physics, settings.show_physics);
        sync_check_box(self.show_tbn, settings.show_tbn);
        sync_check_box(self.show_bounds, settings.show_bounds);
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        in_settings: &mut Settings,
    ) {
        scope_profile!();

        let mut settings = in_settings.clone();

        match message.data() {
            UiMessageData::CheckBox(CheckBoxMessage::Check(check)) => {
                let value = check.unwrap_or(false);
                if message.destination() == self.ssao {
                    settings.graphics.use_ssao = value;
                } else if message.destination() == self.point_shadows {
                    settings.graphics.point_shadows_enabled = value;
                } else if message.destination() == self.spot_shadows {
                    settings.graphics.spot_shadows_enabled = value;
                } else if message.destination() == self.light_scatter {
                    settings.graphics.light_scatter_enabled = value;
                } else if message.destination() == self.parallax_mapping {
                    settings.graphics.use_parallax_mapping = value;
                } else if message.destination() == self.show_bounds {
                    settings.show_bounds = value;
                } else if message.destination() == self.show_tbn {
                    settings.show_tbn = value;
                } else if message.destination() == self.show_physics {
                    settings.show_physics = value;
                }
            }
            UiMessageData::ColorField(msg)
                if message.direction() == MessageDirection::FromWidget =>
            {
                // TODO: Should not be here!
                if message.destination() == self.ambient_color {
                    if let ColorFieldMessage::Color(color) = *msg {
                        engine.scenes[editor_scene.scene].ambient_lighting_color = color;
                    }
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.ok {
                    engine.user_interface.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                } else if message.destination() == self.default {
                    settings = Default::default();
                    self.sync_to_model(&engine.user_interface, &settings);
                }
            }
            &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if message.destination() == self.near_plane {
                    settings.z_near = value;
                } else if message.destination() == self.far_plane {
                    settings.z_far = value;
                }
            }
            _ => {}
        }

        // Apply only if anything changed.
        if &settings != in_settings {
            *in_settings = settings.clone();

            if settings.graphics != engine.renderer.get_quality_settings() {
                if let Err(e) = engine.renderer.set_quality_settings(&settings.graphics) {
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

            // Save config
            match settings.save() {
                Ok(_) => {
                    println!("Settings were successfully saved!");
                }
                Err(e) => {
                    println!("Unable to save settings! Reason: {:?}!", e);
                }
            };
        }
    }
}
