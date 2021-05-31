use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::EditorScene,
    GameEngine, Message, STARTUP_WORKING_DIR,
};
use rg3d::gui::message::{TreeRootMessage, WidgetMessage};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
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
        tree::{TreeBuilder, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    renderer::QualitySettings,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, sync::mpsc::Sender};

struct SwitchEntry {
    tree_item: Handle<UiNode>,
    section: Handle<UiNode>,
}

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    sender: Sender<Message>,
    graphics_section: GraphicsSection,
    move_mode_section: MoveModeSection,
    debugging_section: DebuggingSection,
    section_switches: Vec<SwitchEntry>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct MoveInteractionModeSettings {
    pub grid_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for MoveInteractionModeSettings {
    fn default() -> Self {
        Self {
            grid_snapping: false,
            x_snap_step: 0.05,
            y_snap_step: 0.05,
            z_snap_step: 0.05,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct DebuggingSettings {
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
}

impl Default for DebuggingSettings {
    fn default() -> Self {
        Self {
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct GraphicsSettings {
    pub quality: QualitySettings,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            z_near: 0.025,
            z_far: 128.0,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default)]
pub struct Settings {
    pub graphics: GraphicsSettings,
    pub debugging: DebuggingSettings,
    pub move_mode_settings: MoveInteractionModeSettings,
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

fn make_f32_input_field(
    ctx: &mut BuildContext,
    row: usize,
    value: f32,
    min: f32,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_column(1)
            .on_row(row)
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_value(value)
    .with_min_value(min)
    .with_min_value(min)
    .build(ctx)
}

struct MoveModeSection {
    section: Handle<UiNode>,
    snapping: Handle<UiNode>,
    x_snap_step: Handle<UiNode>,
    y_snap_step: Handle<UiNode>,
    z_snap_step: Handle<UiNode>,
}

impl MoveModeSection {
    pub fn new(ctx: &mut BuildContext, settings: &MoveInteractionModeSettings) -> Self {
        let snapping;
        let x_snap_step;
        let y_snap_step;
        let z_snap_step;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(make_text_mark(ctx, "Snapping", 0))
                .with_child({
                    snapping = make_bool_input_field(ctx, 0, settings.grid_snapping);
                    snapping
                })
                .with_child(make_text_mark(ctx, "X Snap Step", 1))
                .with_child({
                    x_snap_step = make_f32_input_field(ctx, 1, settings.x_snap_step, 0.001);
                    x_snap_step
                })
                .with_child(make_text_mark(ctx, "Y Snap Step", 2))
                .with_child({
                    y_snap_step = make_f32_input_field(ctx, 2, settings.y_snap_step, 0.001);
                    y_snap_step
                })
                .with_child(make_text_mark(ctx, "Z Snap Step", 3))
                .with_child({
                    z_snap_step = make_f32_input_field(ctx, 3, settings.z_snap_step, 0.001);
                    z_snap_step
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            snapping,
            x_snap_step,
            y_snap_step,
            z_snap_step,
        }
    }

    fn sync_to_model(&self, ui: &Ui, settings: &MoveInteractionModeSettings) {
        for &(node, value) in &[
            (self.x_snap_step, settings.x_snap_step),
            (self.y_snap_step, settings.y_snap_step),
            (self.z_snap_step, settings.z_snap_step),
        ] {
            ui.send_message(NumericUpDownMessage::value(
                node,
                MessageDirection::ToWidget,
                value,
            ));
        }

        ui.send_message(CheckBoxMessage::checked(
            self.snapping,
            MessageDirection::ToWidget,
            Some(settings.grid_snapping),
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        settings: &mut MoveInteractionModeSettings,
    ) {
        match message.data() {
            &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if message.destination() == self.x_snap_step {
                    settings.x_snap_step = value;
                } else if message.destination() == self.y_snap_step {
                    settings.y_snap_step = value;
                } else if message.destination() == self.z_snap_step {
                    settings.z_snap_step = value;
                }
            }
            &UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) => {
                if message.destination() == self.snapping {
                    settings.grid_snapping = value;
                }
            }
            _ => {}
        }
    }
}

struct DebuggingSection {
    section: Handle<UiNode>,
    show_physics: Handle<UiNode>,
    show_bounds: Handle<UiNode>,
    show_tbn: Handle<UiNode>,
}

impl DebuggingSection {
    pub fn new(ctx: &mut BuildContext, settings: &DebuggingSettings) -> Self {
        let show_physics;
        let show_bounds;
        let show_tbn;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(make_text_mark(ctx, "Show Physics", 0))
                .with_child({
                    show_physics = make_bool_input_field(ctx, 0, settings.show_physics);
                    show_physics
                })
                .with_child(make_text_mark(ctx, "Show Bounds", 1))
                .with_child({
                    show_bounds = make_bool_input_field(ctx, 1, settings.show_bounds);
                    show_bounds
                })
                .with_child(make_text_mark(ctx, "Show TBN", 2))
                .with_child({
                    show_tbn = make_bool_input_field(ctx, 2, settings.show_tbn);
                    show_tbn
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            show_bounds,
            show_physics,
            show_tbn,
        }
    }

    fn sync_to_model(&self, ui: &Ui, settings: &DebuggingSettings) {
        ui.send_message(CheckBoxMessage::checked(
            self.show_tbn,
            MessageDirection::ToWidget,
            Some(settings.show_tbn),
        ));

        ui.send_message(CheckBoxMessage::checked(
            self.show_physics,
            MessageDirection::ToWidget,
            Some(settings.show_physics),
        ));

        ui.send_message(CheckBoxMessage::checked(
            self.show_bounds,
            MessageDirection::ToWidget,
            Some(settings.show_bounds),
        ));
    }

    pub fn handle_message(&mut self, message: &UiMessage, settings: &mut DebuggingSettings) {
        if let &UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) = message.data() {
            if message.destination() == self.show_bounds {
                settings.show_bounds = value;
            } else if message.destination() == self.show_tbn {
                settings.show_tbn = value;
            } else if message.destination() == self.show_physics {
                settings.show_tbn = value;
            }
        }
    }
}

pub struct GraphicsSection {
    section: Handle<UiNode>,
    ssao: Handle<UiNode>,
    point_shadows: Handle<UiNode>,
    spot_shadows: Handle<UiNode>,
    ambient_color: Handle<UiNode>,
    light_scatter: Handle<UiNode>,
    near_plane: Handle<UiNode>,
    far_plane: Handle<UiNode>,
    parallax_mapping: Handle<UiNode>,
}

impl GraphicsSection {
    pub fn new(ctx: &mut BuildContext, settings: &GraphicsSettings) -> Self {
        let ssao;
        let ambient_color;
        let point_shadows;
        let spot_shadows;
        let light_scatter;
        let near_plane;
        let far_plane;
        let parallax_mapping;

        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "SSAO", 0))
                .with_child({
                    ssao = make_bool_input_field(ctx, 0, settings.quality.use_ssao);
                    ssao
                })
                .with_child(make_text_mark(ctx, "Ambient Color", 1))
                .with_child({
                    ambient_color =
                        ColorFieldBuilder::new(WidgetBuilder::new().on_column(1).on_row(1))
                            .build(ctx);
                    ambient_color
                })
                .with_child(make_text_mark(ctx, "Point Shadows", 2))
                .with_child({
                    point_shadows =
                        make_bool_input_field(ctx, 2, settings.quality.point_shadows_enabled);
                    point_shadows
                })
                .with_child(make_text_mark(ctx, "Spot Shadows", 3))
                .with_child({
                    spot_shadows =
                        make_bool_input_field(ctx, 3, settings.quality.spot_shadows_enabled);
                    spot_shadows
                })
                .with_child(make_text_mark(ctx, "Light Scatter", 4))
                .with_child({
                    light_scatter =
                        make_bool_input_field(ctx, 4, settings.quality.light_scatter_enabled);
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
                    .with_min_value(0.001)
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
                    .with_min_value(1.0)
                    .with_value(settings.z_far)
                    .build(ctx);
                    far_plane
                })
                .with_child(make_text_mark(ctx, "Parallax Mapping", 7))
                .with_child({
                    parallax_mapping =
                        make_bool_input_field(ctx, 7, settings.quality.use_parallax_mapping);
                    parallax_mapping
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
        .add_row(Row::stretch())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            ssao,
            ambient_color,
            point_shadows,
            spot_shadows,
            light_scatter,
            near_plane,
            far_plane,
            parallax_mapping,
        }
    }

    fn sync_to_model(&self, ui: &Ui, settings: &GraphicsSettings) {
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

        sync_check_box(self.ssao, settings.quality.use_ssao);
        sync_check_box(self.point_shadows, settings.quality.point_shadows_enabled);
        sync_check_box(self.spot_shadows, settings.quality.spot_shadows_enabled);
        sync_check_box(self.light_scatter, settings.quality.light_scatter_enabled);
        sync_check_box(self.parallax_mapping, settings.quality.use_parallax_mapping);
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        settings: &mut GraphicsSettings,
    ) {
        match message.data() {
            UiMessageData::CheckBox(CheckBoxMessage::Check(check)) => {
                let value = check.unwrap_or(false);
                if message.destination() == self.ssao {
                    settings.quality.use_ssao = value;
                } else if message.destination() == self.point_shadows {
                    settings.quality.point_shadows_enabled = value;
                } else if message.destination() == self.spot_shadows {
                    settings.quality.spot_shadows_enabled = value;
                } else if message.destination() == self.light_scatter {
                    settings.quality.light_scatter_enabled = value;
                } else if message.destination() == self.parallax_mapping {
                    settings.quality.use_parallax_mapping = value;
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
            &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if message.destination() == self.near_plane {
                    settings.z_near = value;
                } else if message.destination() == self.far_plane {
                    settings.z_far = value;
                }
            }
            _ => {}
        }
    }
}

impl SettingsWindow {
    pub fn new(engine: &mut GameEngine, sender: Sender<Message>, settings: &Settings) -> Self {
        let ok;
        let default;

        let ctx = &mut engine.user_interface.build_ctx();
        let text =
            "Here you can select graphics settings to improve performance and/or to understand how \
            you scene will look like with different graphics settings. Please note that these settings won't be saved \
            with scene!";

        let graphics_section = GraphicsSection::new(ctx, &settings.graphics);
        let debugging_section = DebuggingSection::new(ctx, &settings.debugging);
        let move_mode_section = MoveModeSection::new(ctx, &settings.move_mode_settings);

        let graphics_section_item;
        let debugging_section_item;
        let move_mode_section_item;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(
                    TreeRootBuilder::new(WidgetBuilder::new().on_column(0).on_row(0))
                        .with_items(vec![
                            {
                                graphics_section_item = TreeBuilder::new(WidgetBuilder::new())
                                    .with_content(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text("Graphics")
                                            .build(ctx),
                                    )
                                    .build(ctx);
                                graphics_section_item
                            },
                            {
                                debugging_section_item = TreeBuilder::new(WidgetBuilder::new())
                                    .with_content(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text("Debugging")
                                            .build(ctx),
                                    )
                                    .build(ctx);
                                debugging_section_item
                            },
                            {
                                move_mode_section_item = TreeBuilder::new(WidgetBuilder::new())
                                    .with_content(
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text("Move Interaction Mode")
                                            .build(ctx),
                                    )
                                    .build(ctx);
                                move_mode_section_item
                            },
                        ])
                        .build(ctx),
                )
                .with_child(
                    BorderBuilder::new(WidgetBuilder::new().on_row(0).on_column(1).with_children(
                        &[
                            graphics_section.section,
                            debugging_section.section,
                            move_mode_section.section,
                        ],
                    ))
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .build(ctx);

        let section_switches = vec![
            SwitchEntry {
                tree_item: graphics_section_item,
                section: graphics_section.section,
            },
            SwitchEntry {
                tree_item: debugging_section_item,
                section: debugging_section.section,
            },
            SwitchEntry {
                tree_item: move_mode_section_item,
                section: move_mode_section.section,
            },
        ];

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
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
                        .with_child(section)
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
            section_switches,
            window,
            sender,
            ok,
            default,
            graphics_section,
            move_mode_section,
            debugging_section,
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
        self.graphics_section.sync_to_model(ui, &settings.graphics);
        self.move_mode_section
            .sync_to_model(ui, &settings.move_mode_settings);
        self.debugging_section
            .sync_to_model(ui, &settings.debugging);
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

        self.graphics_section.handle_message(
            message,
            editor_scene,
            engine,
            &mut in_settings.graphics,
        );
        self.debugging_section
            .handle_message(message, &mut in_settings.debugging);
        self.move_mode_section
            .handle_message(message, &mut in_settings.move_mode_settings);

        match message.data() {
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
            UiMessageData::TreeRoot(TreeRootMessage::Selected(items)) => {
                if let Some(selected) = items.first().cloned() {
                    for entry in self.section_switches.iter() {
                        engine
                            .user_interface
                            .send_message(WidgetMessage::visibility(
                                entry.section,
                                MessageDirection::ToWidget,
                                entry.tree_item == selected,
                            ))
                    }
                }
            }
            _ => {}
        }

        // Apply only if anything changed.
        if &settings != in_settings {
            *in_settings = settings.clone();

            if settings.graphics.quality != engine.renderer.get_quality_settings() {
                if let Err(e) = engine
                    .renderer
                    .set_quality_settings(&settings.graphics.quality)
                {
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
