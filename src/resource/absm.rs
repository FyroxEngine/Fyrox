//! Animation blending state machine resource.

use crate::{
    animation::machine::{MachineDefinition, MachineInstantiationError},
    asset::{define_new_resource, Resource, ResourceData},
    core::{reflect::prelude::*, visitor::prelude::*},
    engine::resource_manager::options::ImportOptions,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// State of the [`AbsmResource`]
#[derive(Debug, Visit, Default)]
pub struct AbsmResourceState {
    /// A path to source.
    pub path: PathBuf,
    /// Animation blending state machine definition.
    pub absm_definition: MachineDefinition,
}

impl ResourceData for AbsmResourceState {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl AbsmResourceState {
    /// Load a ABSM resource from the specific file path.
    pub async fn from_file(path: &Path) -> Result<Self, MachineInstantiationError> {
        let mut visitor = Visitor::load_binary(path).await?;
        let mut absm_definition = MachineDefinition::default();
        absm_definition.visit("Machine", &mut visitor)?;
        Ok(Self {
            absm_definition,
            path: path.to_path_buf(),
        })
    }
}

define_new_resource!(
    /// See module docs.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    AbsmResource<AbsmResourceState, MachineInstantiationError>
);

/// Import options for ABSM resource.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AbsmImportOptions {}

impl ImportOptions for AbsmImportOptions {}
