use crate::settings::build::{BuildCommand, BuildProfile, EnvironmentVariable};
use crate::{
    fyrox::{
        core::{log::Log, pool::Handle, reflect::prelude::*, scope_profile},
        gui::{
            button::{ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::{
                    enumeration::EnumPropertyEditorDefinition,
                    inspectable::InspectablePropertyEditorDefinition,
                    key::HotKeyPropertyEditorDefinition, PropertyEditorDefinitionContainer,
                },
                InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
                PropertyChanged,
            },
            message::{MessageDirection, UiMessage},
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            widget::WidgetBuilder,
            window::{WindowBuilder, WindowMessage, WindowTitle},
            HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
        renderer::{CsmSettings, QualitySettings, ShadowMapPrecision},
    },
    inspector::editors::make_property_editors_container,
    message::MessageSender,
    settings::{
        build::BuildSettings,
        camera::CameraSettings,
        debugging::DebuggingSettings,
        general::{GeneralSettings, ScriptEditor},
        graphics::GraphicsSettings,
        keys::{KeyBindings, TerrainKeyBindings},
        model::ModelSettings,
        move_mode::MoveInteractionModeSettings,
        navmesh::NavmeshSettings,
        recent::RecentFiles,
        rotate_mode::RotateInteractionModeSettings,
        scene::SceneSettings,
        selection::SelectionSettings,
        windows::WindowsSettings,
    },
    Engine, MSG_SYNC_FLAG,
};
use fyrox::gui::inspector::editors::collection::VecCollectionPropertyEditorDefinition;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};

pub mod build;
pub mod camera;
pub mod debugging;
pub mod general;
pub mod graphics;
pub mod keys;
pub mod model;
pub mod move_mode;
pub mod navmesh;
pub mod recent;
pub mod rotate_mode;
pub mod scene;
pub mod selection;
pub mod windows;

pub struct SettingsWindow {
    window: Handle<UiNode>,
    ok: Handle<UiNode>,
    default: Handle<UiNode>,
    inspector: Handle<UiNode>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Default, Debug, Reflect)]
pub struct SettingsData {
    pub selection: SelectionSettings,
    pub graphics: GraphicsSettings,
    #[serde(default)]
    pub build: BuildSettings,
    #[serde(default)]
    pub general: GeneralSettings,
    pub debugging: DebuggingSettings,
    pub move_mode_settings: MoveInteractionModeSettings,
    pub rotate_mode_settings: RotateInteractionModeSettings,
    pub model: ModelSettings,
    pub camera: CameraSettings,
    pub navmesh: NavmeshSettings,
    pub key_bindings: KeyBindings,
    #[reflect(hidden)]
    pub scene_settings: HashMap<PathBuf, SceneSettings>,
    #[reflect(hidden)]
    pub recent: RecentFiles,
    #[serde(default)]
    #[reflect(hidden)]
    pub windows: WindowsSettings,
}

pub enum SettingsMessage {
    Changed,
}

#[derive(Default)]
pub struct Settings {
    settings: SettingsData,
    need_save: bool,
    pub subscribers: Vec<Sender<SettingsMessage>>,
}

impl Deref for Settings {
    type Target = SettingsData;

    fn deref(&self) -> &Self::Target {
        &self.settings
    }
}

impl DerefMut for Settings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.need_save = true;

        self.subscribers
            .retain_mut(|subscriber| subscriber.send(SettingsMessage::Changed).is_ok());

        &mut self.settings
    }
}

impl Settings {
    pub fn load() -> Result<Self, SettingsError> {
        Ok(Settings {
            settings: SettingsData::load()?,
            need_save: false,
            subscribers: Default::default(),
        })
    }

    pub fn force_save(&mut self) {
        self.need_save = false;
        Log::verify(self.settings.save());
    }

    pub fn update(&mut self) {
        if self.need_save {
            self.need_save = false;
            Log::verify(self.settings.save());
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    RonSpanned(ron::error::SpannedError),
    Ron(ron::Error),
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ron::error::SpannedError> for SettingsError {
    fn from(e: ron::error::SpannedError) -> Self {
        Self::RonSpanned(e)
    }
}

impl From<ron::Error> for SettingsError {
    fn from(e: ron::Error) -> Self {
        Self::Ron(e)
    }
}

impl SettingsData {
    const FILE_NAME: &'static str = "settings.ron";

    fn full_path() -> PathBuf {
        Self::FILE_NAME.into()
    }

    pub fn load() -> Result<Self, SettingsError> {
        let file = File::open(Self::full_path())?;
        Ok(ron::de::from_reader(file)?)
    }

    fn save(&mut self) -> Result<(), SettingsError> {
        let mut file = File::create(Self::full_path())?;
        self.recent.deduplicate_and_refresh();

        file.write_all(ron::ser::to_string_pretty(self, PrettyConfig::default())?.as_bytes())?;

        Log::info("Settings were successfully saved!");
        Ok(())
    }

    fn make_property_editors_container(
        sender: MessageSender,
    ) -> Arc<PropertyEditorDefinitionContainer> {
        let container = make_property_editors_container(sender);

        container.insert(InspectablePropertyEditorDefinition::<GeneralSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<GraphicsSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<SelectionSettings>::new());
        container.insert(EnumPropertyEditorDefinition::<ShadowMapPrecision>::new());
        container.insert(EnumPropertyEditorDefinition::<ScriptEditor>::new());
        container.insert(InspectablePropertyEditorDefinition::<DebuggingSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<CsmSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<QualitySettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<CameraSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<
            MoveInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<
            RotateInteractionModeSettings,
        >::new());
        container.insert(InspectablePropertyEditorDefinition::<ModelSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<NavmeshSettings>::new());
        container.insert(InspectablePropertyEditorDefinition::<KeyBindings>::new());
        container.insert(InspectablePropertyEditorDefinition::<TerrainKeyBindings>::new());
        container.insert(InspectablePropertyEditorDefinition::<BuildSettings>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<EnvironmentVariable>::new());
        container.insert(InspectablePropertyEditorDefinition::<EnvironmentVariable>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<BuildProfile>::new());
        container.insert(InspectablePropertyEditorDefinition::<BuildProfile>::new());
        container.insert(VecCollectionPropertyEditorDefinition::<BuildCommand>::new());
        container.insert(InspectablePropertyEditorDefinition::<BuildCommand>::new());
        container.insert(HotKeyPropertyEditorDefinition);

        Arc::new(container)
    }

    fn handle_property_changed(&mut self, property_changed: &PropertyChanged) {
        PropertyAction::from_field_kind(&property_changed.value).apply(
            &property_changed.path(),
            self,
            &mut Log::verify,
        );
    }
}

impl SettingsWindow {
    pub fn new(engine: &mut Engine) -> Self {
        let ok;
        let default;

        let ctx = &mut engine.user_interfaces.first_mut().build_ctx();

        let inspector = InspectorBuilder::new(WidgetBuilder::new()).build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text("Settings"))
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

    pub fn open(&self, ui: &mut UserInterface, settings: &Settings, sender: &MessageSender) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        self.sync_to_model(ui, settings, sender);
    }

    fn sync_to_model(&self, ui: &mut UserInterface, settings: &Settings, sender: &MessageSender) {
        let context = InspectorContext::from_object(
            &**settings,
            &mut ui.build_ctx(),
            SettingsData::make_property_editors_container(sender.clone()),
            None,
            MSG_SYNC_FLAG,
            0,
            true,
            Default::default(),
            150.0,
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
        engine: &mut Engine,
        settings: &mut Settings,
        sender: &MessageSender,
    ) {
        scope_profile!();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.default {
                **settings = Default::default();

                self.sync_to_model(engine.user_interfaces.first_mut(), settings, sender);
            }
        } else if let Some(InspectorMessage::PropertyChanged(property_changed)) = message.data() {
            if message.destination() == self.inspector {
                settings.handle_property_changed(property_changed);
            }
        }

        let graphics_context = engine.graphics_context.as_initialized_mut();

        if settings.graphics.quality != graphics_context.renderer.get_quality_settings() {
            if let Err(e) = graphics_context
                .renderer
                .set_quality_settings(&settings.graphics.quality)
            {
                Log::err(format!(
                    "An error occurred at attempt to set new graphics settings: {:?}",
                    e
                ));
            } else {
                Log::info("New graphics quality settings were successfully set!");
            }
        }
    }
}
