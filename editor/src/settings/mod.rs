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
    fyrox::core::{
        dyntype::DynTypeContainer, futures::executor::block_on, log::Log, reflect::prelude::*,
        visitor::prelude::*,
    },
    settings::{
        build::BuildSettings, camera::CameraSettings, debugging::DebuggingSettings,
        general::GeneralSettings, graphics::GraphicsSettings, keys::KeyBindings, log::LogSettings,
        model::ModelSettings, move_mode::MoveInteractionModeSettings, recent::RecentFiles,
        rotate_mode::RotateInteractionModeSettings, scene::SceneSettings,
        selection::SelectionSettings, windows::WindowsSettings,
    },
};
use fyrox::core::dyntype::{DynType, DynTypeConstructorContainer};
use std::cell::{RefCell, RefMut};
use std::sync::Arc;
use std::{
    collections::HashMap,
    fmt::Display,
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
pub mod log;
pub mod model;
pub mod move_mode;
pub mod recent;
pub mod rotate_mode;
pub mod scene;
pub mod selection;
pub mod windows;

#[derive(PartialEq, Clone, Default, Debug, Reflect, Visit)]
#[reflect(type_uuid = "8b5064fc-e25b-442c-af34-67b12b6f9e66")]
pub struct SettingsData {
    #[reflect(tag = "Group.Selection")]
    pub selection: SelectionSettings,
    #[reflect(tag = "Group.Graphics")]
    pub graphics: GraphicsSettings,
    #[reflect(tag = "Group.Build")]
    pub build: BuildSettings,
    #[reflect(tag = "Group.General")]
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
    #[reflect(tag = "Group.KeyBindings")]
    pub key_bindings: KeyBindings,
    #[reflect(tag = "Group.Log")]
    #[reflect(hidden)]
    pub log: LogSettings,
    #[reflect(hidden)]
    pub scene_settings: HashMap<PathBuf, SceneSettings>,
    #[reflect(hidden)]
    pub recent: RecentFiles,
    #[reflect(hidden)]
    pub windows: WindowsSettings,
    #[reflect(immutable_collection, tag = "Group.Plugins")]
    #[visit(optional)]
    plugin_settings: RefCell<Vec<DynTypeContainer>>,
}

impl SettingsData {
    /// Tries to fetch arbitrary data type from the settings container. If the instance is not found,
    /// then this method creates a new instance and adds it to the container. This method may fail
    /// only on
    pub fn get<T>(&self) -> Option<RefMut<'_, T>>
    where
        T: DynType + Default,
    {
        let settings = self.plugin_settings.try_borrow_mut().ok()?;
        RefMut::filter_map(settings, |settings| {
            // SAFETY: This is safe, because there's only one branch is used to return the value.
            let settings2 = unsafe { &mut *(settings as *mut Vec<DynTypeContainer>) };
            if let Some(ret) = settings
                .iter_mut()
                .find_map(|container| container.data_mut().ok())
            {
                Some(ret)
            } else {
                settings2.push(DynTypeContainer::new(T::default()));
                settings2
                    .last_mut()
                    .and_then(|container| container.data_mut().ok())
            }
        })
        .ok()
    }
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
    pub fn load(
        dyn_type_constructors: Arc<DynTypeConstructorContainer>,
    ) -> Result<Self, SettingsError> {
        let settings = SettingsData::load(dyn_type_constructors)?;
        Log::set_log_info(settings.log.log_info);
        Log::set_log_warning(settings.log.log_warning);
        Log::set_log_error(settings.log.log_error);
        Ok(Settings {
            settings,
            need_save: false,
            subscribers: Default::default(),
        })
    }

    pub fn data_mut(&mut self) -> &mut SettingsData {
        &mut self.settings
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
    Visit(VisitError),
}

impl std::error::Error for SettingsError {}

impl Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(error) => Display::fmt(error, f),
            SettingsError::Visit(error) => Display::fmt(error, f),
        }
    }
}

impl From<VisitError> for SettingsError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

impl SettingsData {
    const FILE_NAME: &'static str = "settings.cfg";

    fn full_path() -> PathBuf {
        Self::FILE_NAME.into()
    }

    pub fn load(
        dyn_type_constructors: Arc<DynTypeConstructorContainer>,
    ) -> Result<Self, SettingsError> {
        let mut visitor = block_on(Visitor::load_from_file(Self::full_path()))?;
        visitor.blackboard.register(dyn_type_constructors);
        let mut data = Self::default();
        data.visit("Settings", &mut visitor)?;
        Ok(data)
    }

    fn save(&mut self) -> Result<(), SettingsError> {
        self.recent.deduplicate_and_refresh();
        let mut visitor = Visitor::new();
        self.visit("Settings", &mut visitor)?;
        visitor.save_ascii_to_file(Self::full_path())?;
        Ok(())
    }
}
