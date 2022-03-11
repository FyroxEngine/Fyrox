use crate::{
    scene::EditorScene,
    settings::{
        debugging::{DebuggingSection, DebuggingSettings},
        graphics::{GraphicsSection, GraphicsSettings},
        move_mode::{MoveInteractionModeSettings, MoveModeSection},
        rotate_mode::{RotateInteractionModeSettings, RotateModeSection},
    },
    GameEngine, CONFIG_DIR,
};
use fyrox::{
    core::{pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        check_box::CheckBoxBuilder,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{TreeBuilder, TreeRootBuilder, TreeRootMessage},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
    utils::log::Log,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};

pub mod debugging;
pub mod graphics;
pub mod move_mode;
pub mod rotate_mode;

struct SwitchEntry {
    tree_item: Handle<UiNode>,
    section: Handle<UiNode>,
    kind: SettingsSectionKind,
}

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    graphics_section: GraphicsSection,
    move_mode_section: MoveModeSection,
    rotate_mode_section: RotateModeSection,
    debugging_section: DebuggingSection,
    section_switches: Vec<SwitchEntry>,
    sections_root: Handle<UiNode>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SettingsSectionKind {
    Graphics,
    Debugging,
    MoveModeSettings,
    RotateModeSettings,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default)]
pub struct Settings {
    pub graphics: GraphicsSettings,
    pub debugging: DebuggingSettings,
    pub move_mode_settings: MoveInteractionModeSettings,
    pub rotate_mode_settings: RotateInteractionModeSettings,
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
        CONFIG_DIR.lock().join(Self::FILE_NAME)
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

fn make_section(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    TreeBuilder::new(WidgetBuilder::new())
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(name)
                .build(ctx),
        )
        .build(ctx)
}

impl SettingsWindow {
    pub fn new(engine: &mut GameEngine, settings: &Settings) -> Self {
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
        let rotate_mode_section = RotateModeSection::new(ctx, &settings.rotate_mode_settings);

        let sections_root;
        let graphics_section_item;
        let debugging_section_item;
        let move_mode_section_item;
        let rotate_mode_section_item;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child({
                    sections_root =
                        TreeRootBuilder::new(WidgetBuilder::new().on_column(0).on_row(0))
                            .with_items(vec![
                                {
                                    graphics_section_item = make_section(ctx, "Graphics");
                                    graphics_section_item
                                },
                                {
                                    debugging_section_item = make_section(ctx, "Debugging");
                                    debugging_section_item
                                },
                                {
                                    move_mode_section_item =
                                        make_section(ctx, "Move Interaction Mode");
                                    move_mode_section_item
                                },
                                {
                                    rotate_mode_section_item =
                                        make_section(ctx, "Rotate Interaction Mode");
                                    rotate_mode_section_item
                                },
                            ])
                            .build(ctx);
                    sections_root
                })
                .with_child(
                    BorderBuilder::new(WidgetBuilder::new().on_row(0).on_column(1).with_children(
                        [
                            graphics_section.section,
                            debugging_section.section,
                            move_mode_section.section,
                            rotate_mode_section.section,
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
                kind: SettingsSectionKind::Graphics,
            },
            SwitchEntry {
                tree_item: debugging_section_item,
                section: debugging_section.section,
                kind: SettingsSectionKind::Debugging,
            },
            SwitchEntry {
                tree_item: move_mode_section_item,
                section: move_mode_section.section,
                kind: SettingsSectionKind::MoveModeSettings,
            },
            SwitchEntry {
                tree_item: rotate_mode_section_item,
                section: rotate_mode_section.section,
                kind: SettingsSectionKind::RotateModeSettings,
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
                            .with_wrap(WrapMode::Word)
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
            sections_root,
            section_switches,
            window,
            ok,
            default,
            graphics_section,
            move_mode_section,
            rotate_mode_section,
            debugging_section,
        }
    }

    pub fn open(
        &self,
        ui: &UserInterface,
        settings: &Settings,
        section: Option<SettingsSectionKind>,
    ) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        if let Some(section) = section {
            for entry in self.section_switches.iter() {
                if entry.kind == section {
                    ui.send_message(TreeRootMessage::select(
                        self.sections_root,
                        MessageDirection::ToWidget,
                        vec![entry.tree_item],
                    ));
                }
            }
        }

        self.sync_to_model(ui, settings);
    }

    fn sync_to_model(&self, ui: &UserInterface, settings: &Settings) {
        self.graphics_section.sync_to_model(ui, &settings.graphics);
        self.move_mode_section
            .sync_to_model(ui, &settings.move_mode_settings);
        self.rotate_mode_section
            .sync_to_model(ui, &settings.rotate_mode_settings);
        self.debugging_section
            .sync_to_model(ui, &settings.debugging);
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        settings: &mut Settings,
    ) {
        scope_profile!();

        let old_settings = settings.clone();

        self.graphics_section
            .handle_message(message, editor_scene, engine, &mut settings.graphics);
        self.debugging_section
            .handle_message(message, &mut settings.debugging);
        self.move_mode_section
            .handle_message(message, &mut settings.move_mode_settings);
        self.rotate_mode_section
            .handle_message(message, &mut settings.rotate_mode_settings);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                engine.user_interface.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.default {
                *settings = Default::default();
                self.sync_to_model(&engine.user_interface, settings);
            }
        } else if let Some(TreeRootMessage::Selected(items)) = message.data::<TreeRootMessage>() {
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

        // Apply only if anything changed.
        if settings != &old_settings {
            if settings.graphics.quality != engine.renderer.get_quality_settings() {
                if let Err(e) = engine
                    .renderer
                    .set_quality_settings(&settings.graphics.quality)
                {
                    Log::err(format!(
                        "An error occurred at attempt to set new graphics settings: {:?}",
                        e
                    ));
                } else {
                    Log::info("New graphics quality settings were successfully set!".to_owned());
                }
            }

            // Save config
            match settings.save() {
                Ok(_) => {
                    Log::info("Settings were successfully saved!".to_owned());
                }
                Err(e) => {
                    Log::err(format!("Unable to save settings! Reason: {:?}!", e));
                }
            };
        }
    }
}
