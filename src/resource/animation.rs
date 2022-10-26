//! Animation resource is a container for animation data.

use crate::{
    animation::{definition::AnimationDefinition, Animation},
    asset::{define_new_resource, Resource, ResourceData},
    core::{io::FileLoadError, pool::Handle, reflect::Reflect, visitor::prelude::*},
    engine::resource_manager::options::ImportOptions,
    scene::{node::Node, Scene},
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// An error that may occur during animation resource loading.
#[derive(Debug, thiserror::Error)]
pub enum AnimationResourceError {
    /// An i/o error has occurred.
    #[error("A file load error has occurred {0:?}")]
    Io(FileLoadError),

    /// An error that may occur due to version incompatibilities.
    #[error("An error that may occur due to version incompatibilities. {0:?}")]
    Visit(VisitError),
}

impl From<FileLoadError> for AnimationResourceError {
    fn from(e: FileLoadError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for AnimationResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

/// State of the [`AnimationResource`]
#[derive(Debug, Visit, Default)]
pub struct AnimationResourceState {
    pub(crate) path: PathBuf,
    /// Actual animation definition.
    pub animation_definition: AnimationDefinition,
}

impl ResourceData for AnimationResourceState {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl AnimationResourceState {
    /// Load an animation resource from the specific file path.
    pub async fn from_file(path: &Path) -> Result<Self, AnimationResourceError> {
        let mut visitor = Visitor::load_binary(path).await?;
        let mut animation_definition = AnimationDefinition::default();
        animation_definition.visit("Definition", &mut visitor)?;
        Ok(Self {
            animation_definition,
            path: path.to_path_buf(),
        })
    }
}

define_new_resource!(
    /// See module docs.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    AnimationResource<AnimationResourceState, AnimationResourceError>
);

/// Import options for animation resource.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AnimationImportOptions {}

impl ImportOptions for AnimationImportOptions {}

impl AnimationResource {
    /// Creates an instance of animation resource.
    pub fn instantiate(&self, root: Handle<Node>, scene: &mut Scene) -> Handle<Animation> {
        self.data_ref()
            .animation_definition
            .instantiate(root, scene)
    }
}
