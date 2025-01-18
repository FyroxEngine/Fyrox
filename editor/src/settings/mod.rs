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

use crate::{
    fyrox::core::{log::Log, reflect::prelude::*},
    settings::{
        build::BuildSettings, camera::CameraSettings, debugging::DebuggingSettings,
        general::GeneralSettings, graphics::GraphicsSettings, keys::KeyBindings,
        model::ModelSettings, move_mode::MoveInteractionModeSettings, navmesh::NavmeshSettings,
        recent::RecentFiles, rotate_mode::RotateInteractionModeSettings, scene::SceneSettings,
        selection::SelectionSettings, windows::WindowsSettings,
    },
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::mpsc::Sender,
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

#[derive(Deserialize, Serialize, PartialEq, Clone, Default, Debug, Reflect)]
pub struct SettingsData {
    #[reflect(tag = "Group.Selection")]
    pub selection: SelectionSettings,
    #[reflect(tag = "Group.Graphics")]
    pub graphics: GraphicsSettings,
    #[reflect(tag = "Group.Build")]
    #[serde(default)]
    pub build: BuildSettings,
    #[reflect(tag = "Group.General")]
    #[serde(default)]
    pub general: GeneralSettings,
    #[reflect(tag = "Group.Debugging")]
    pub debugging: DebuggingSettings,
    #[reflect(tag = "Group.MoveMode")]
    pub move_mode_settings: MoveInteractionModeSettings,
    #[reflect(tag = "Group.RotateMode")]
    pub rotate_mode_settings: RotateInteractionModeSettings,
    #[reflect(tag = "Group.Model")]
    pub model: ModelSettings,
    #[reflect(tag = "Group.Camera")]
    pub camera: CameraSettings,
    #[reflect(tag = "Group.Navmesh")]
    pub navmesh: NavmeshSettings,
    #[reflect(tag = "Group.KeyBindings")]
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

    pub fn try_save(&mut self) -> bool {
        if self.need_save {
            self.need_save = false;
            Log::verify(self.settings.save());
            true
        } else {
            false
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
}
