use crate::{
    inspector::editors::make_property_editors_container,
    settings::{
        debugging::DebuggingSettings, graphics::GraphicsSettings, model::ModelSettings,
        move_mode::MoveInteractionModeSettings, rotate_mode::RotateInteractionModeSettings,
        selection::SelectionSettings,
    },
    GameEngine, Message, MSG_SYNC_FLAG,
};
use fyrox::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        scope_profile,
    },
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{
            editors::{
                enumeration::EnumPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            FieldKind, InspectorBuilder, InspectorContext, InspectorMessage, PropertyChanged,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    renderer::{CsmSettings, QualitySettings, ShadowMapPrecision},
    utils::log::Log,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf, rc::Rc, sync::mpsc::Sender};

pub mod debugging;
pub mod graphics;
pub mod model;
pub mod move_mode;
pub mod rotate_mode;
pub mod selection;

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default, Debug, Inspect)]
pub struct Settings {
    pub selection: SelectionSettings,
    pub graphics: GraphicsSettings,
    pub debugging: DebuggingSettings,
    pub move_mode_settings: MoveInteractionModeSettings,
    pub rotate_mode_settings: RotateInteractionModeSettings,
    pub model: ModelSettings,
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
        Self::FILE_NAME.into()
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

    fn make_property_editors_container(
        sender: Sender<Message>,
    ) -> Rc<PropertyEditorDefinitionContainer> {
        let container = make_property_editors_container(sender);

        container.insert(InspectablePropertyEditorDefinition::<GraphicsSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<SelectionSettings>::new());
        container.insert(EnumPropertyEditorDefinition::<ShadowMapPrecision>::new());
        container.insert(InspectablePropertyEditorDefinition::<DebuggingSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<CsmSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<QualitySettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<
            MoveInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<
            RotateInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<ModelSettings>::new());

        Rc::new(container)
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        if let FieldKind::Inspectable(ref inner) = property_changed.value {
            return match property_changed.name.as_ref() {
                Self::SELECTION => self.selection.handle_property_changed(&**inner),
                Self::GRAPHICS => self.graphics.handle_property_changed(&**inner),
                Self::DEBUGGING => self.debugging.handle_property_changed(&**inner),
                Self::MOVE_MODE_SETTINGS => {
                    self.move_mode_settings.handle_property_changed(&**inner)
                }
                Self::ROTATE_MODE_SETTINGS => {
                    self.rotate_mode_settings.handle_property_changed(&**inner)
                }
                Self::MODEL => self.model.handle_property_changed(&**inner),
                _ => false,
            };
        }
        false
    }
}

impl SettingsWindow {
    pub fn new(engine: &mut GameEngine) -> Self {
        let ok;
        let default;

        let ctx = &mut engine.user_interface.build_ctx();

        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::Text("Settings".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(2.0))
                                    .on_row(0),
                            )
                            .with_content(inspector)
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
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
                .add_row(Row::stretch())
                .add_row(Row::strict(25.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            ok,
            default,
            inspector,
        }
    }

    pub fn open(&self, ui: &mut UserInterface, settings: &Settings, sender: &Sender<Message>) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        self.sync_to_model(ui, settings, sender);
    }

    fn sync_to_model(&self, ui: &mut UserInterface, settings: &Settings, sender: &Sender<Message>) {
        let context = InspectorContext::from_object(
            settings,
            &mut ui.build_ctx(),
            Settings::make_property_editors_container(sender.clone()),
            None,
            MSG_SYNC_FLAG,
            0,
        );
        ui.send_message(InspectorMessage::context(
            self.inspector,
            MessageDirection::ToWidget,
            context,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        engine: &mut GameEngine,
        settings: &mut Settings,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        let old_settings = settings.clone();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                engine.user_interface.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.default {
                *settings = Default::default();
                self.sync_to_model(&mut engine.user_interface, settings, sender);
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector
                && !settings.handle_property_changed(property_changed)
            {
                Log::err(format!(
                    "Unhandled property change: {}",
                    property_changed.path()
                ))
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
