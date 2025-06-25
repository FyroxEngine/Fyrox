// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use directories::ProjectDirs;
use fyrox::gui::inspector::InspectorContextArgs;
use fyrox::{
    core::{log::Log, pool::Handle, reflect::prelude::*},
    fxhash::FxHashSet,
    graph::BaseSceneGraph,
    gui::{
        inspector::{
            editors::{
                collection::VecCollectionPropertyEditorDefinition,
                inspectable::InspectablePropertyEditorDefinition,
                PropertyEditorDefinitionContainer,
            },
            Inspector, InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction,
        },
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, UiNode, UserInterface,
    },
};
use fyrox_build_tools::{CommandDescriptor, EnvironmentVariable};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, LazyLock},
};

pub const MANIFEST_PATH_VAR: &str = "%MANIFEST_PATH%";
pub const MANIFEST_DIR_VAR: &str = "%MANIFEST_DIR%";

pub static PROJECT_DIRS: LazyLock<Option<ProjectDirs>> =
    LazyLock::new(|| ProjectDirs::from("", "Fyrox", "Fyrox Project Manager"));

pub static CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let config_dir = PROJECT_DIRS
        .as_ref()
        .map(|dirs| dirs.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            eprintln!("Unable to fetch project dirs! Using current folder instead...");
            PathBuf::from("./")
        });
    if !config_dir.exists() {
        if let Err(err) = std::fs::create_dir_all(&config_dir) {
            eprintln!("Unable to create config dir: {err:?}",);
        }
    }
    println!("Config dir: {:?}", config_dir);
    config_dir
});

pub static DATA_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let data_dir = PROJECT_DIRS
        .as_ref()
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            eprintln!("Unable to fetch project dirs! Using current folder instead...");
            PathBuf::from("./")
        });
    if !data_dir.exists() {
        if let Err(err) = std::fs::create_dir_all(&data_dir) {
            eprintln!("Unable to create data dir: {err:?}",);
        }
    }
    println!("Data dir: {:?}", data_dir);
    data_dir
});

#[derive(Default, Serialize, Deserialize, Reflect, Clone, Debug)]
pub struct SettingsData {
    #[serde(default = "default_open_ide_command")]
    #[reflect(
        description = "Defines a command to run an IDE in a project folder. This command \
    should use either %MANIFEST_PATH% or %MANIFEST_DIR% built-in variable to provide the selected project path to the \
    chosen IDE."
    )]
    pub open_ide_command: CommandDescriptor,
    #[reflect(hidden)]
    pub projects: Vec<Project>,
}

fn default_open_ide_command() -> CommandDescriptor {
    CommandDescriptor {
        command: "rustrover64".to_string(),
        args: vec![MANIFEST_PATH_VAR.to_string()],
        environment_variables: vec![],
        skip_passthrough_marker: false,
    }
}

impl SettingsData {
    pub const FILE_NAME: &'static str = "pm_settings.ron";

    fn actual_path() -> PathBuf {
        CONFIG_DIR.join(Self::FILE_NAME)
    }

    pub fn load() -> Self {
        match File::open(Self::actual_path()) {
            Ok(file) => {
                let mut settings: SettingsData = ron::de::from_reader(file).unwrap_or_default();
                settings.remove_non_existent_projects();
                settings.remove_duplicates();
                settings
            }
            Err(err) => {
                eprintln!("Unable to load project manager settings! Reason: {err:?}");
                Default::default()
            }
        }
    }

    pub fn save(&self) {
        match File::create(Self::actual_path()) {
            Ok(mut file) => {
                let pretty_string =
                    ron::ser::to_string_pretty(self, PrettyConfig::default()).unwrap_or_default();
                if let Err(err) = file.write_all(pretty_string.as_bytes()) {
                    eprintln!("Unable to save project manager settings! Reason: {err:?}");
                }
            }
            Err(err) => {
                eprintln!("Unable to create project manager settings file! Reason: {err:?}");
            }
        }
    }

    fn remove_non_existent_projects(&mut self) {
        self.projects
            .retain(|project| project.manifest_path.exists())
    }

    fn remove_duplicates(&mut self) {
        let mut existing = FxHashSet::default();
        self.projects.retain(|project| {
            if existing.contains(&project.manifest_path) {
                false
            } else {
                existing.insert(project.manifest_path.clone());
                true
            }
        });
    }
}

pub struct Settings {
    data: SettingsData,
    need_save: bool,
}

impl Deref for Settings {
    type Target = SettingsData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Settings {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.need_save = true;
        &mut self.data
    }
}

impl Settings {
    pub fn load() -> Self {
        Self {
            data: SettingsData::load(),
            need_save: false,
        }
    }

    pub fn try_save(&mut self) {
        if self.need_save {
            self.need_save = false;
            self.data.save();
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub manifest_path: PathBuf,
    pub name: String,
    pub hot_reload: bool,
}

pub struct SettingsWindow {
    window: Handle<UiNode>,
    inspector: Handle<UiNode>,
    clipboard: Option<Box<dyn Reflect>>,
}

impl SettingsWindow {
    pub fn new(settings: &Settings, ctx: &mut BuildContext) -> Self {
        let property_editors = PropertyEditorDefinitionContainer::with_default_editors();
        property_editors.insert(InspectablePropertyEditorDefinition::<CommandDescriptor>::new());
        property_editors
            .insert(VecCollectionPropertyEditorDefinition::<EnvironmentVariable>::new());
        property_editors.insert(InspectablePropertyEditorDefinition::<EnvironmentVariable>::new());
        let property_editors = Arc::new(property_editors);
        let context = InspectorContext::from_object(InspectorContextArgs {
            object: settings.deref(),
            ctx,
            definition_container: property_editors,
            environment: None,
            sync_flag: 1,
            layer_index: 0,
            generate_property_string_values: true,
            filter: Default::default(),
            name_column_width: 170.0,
            base_path: Default::default(),
        });
        let inspector = InspectorBuilder::new(WidgetBuilder::new())
            .with_context(context)
            .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
            .with_content(
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content(inspector)
                    .build(ctx),
            )
            .open(false)
            .with_title(WindowTitle::text("Settings"))
            .with_remove_on_close(true)
            .build(ctx);

        ctx.send_message(WindowMessage::open_modal(
            window,
            MessageDirection::ToWidget,
            true,
            true,
        ));

        Self {
            window,
            inspector,
            clipboard: None,
        }
    }

    pub fn handle_ui_message(
        mut self,
        settings: &mut Settings,
        message: &UiMessage,
        ui: &mut UserInterface,
    ) -> Option<Self> {
        Inspector::handle_context_menu_message(
            self.inspector,
            message,
            ui,
            settings.deref_mut(),
            &mut self.clipboard,
        );

        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                return None;
            }
        } else if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                PropertyAction::from_field_kind(&args.value).apply(
                    &args.path(),
                    settings.deref_mut(),
                    &mut |result| {
                        Log::verify(result);
                    },
                );

                let ctx = ui
                    .node(self.inspector)
                    .cast::<Inspector>()
                    .unwrap()
                    .context()
                    .clone();

                Log::verify(ctx.sync(
                    &**settings,
                    ui,
                    0,
                    true,
                    Default::default(),
                    Default::default(),
                ));
            }
        }

        Some(self)
    }
}
