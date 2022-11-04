#![warn(missing_docs)]

//! Contains all data structures and method to work with model resources.
//!
//! Model is an isolated scene that is used to create copies of its data - this
//! process is known as `instantiation`. Isolation in this context means that
//! such scene cannot be modified, rendered, etc. It just a data source.
//!
//! All instances will have references to resource they were created from - this
//! will help to get correct vertex and indices buffers when loading a save file,
//! loader will just take all needed data from resource so we don't need to store
//! such data in save file. Also this mechanism works perfectly when you changing
//! resource in external editor (3Ds max, Maya, Blender, etc.) engine will assign
//! correct visual data when loading a saved game.
//!
//! # Supported formats
//!
//! Currently only FBX (common format in game industry for storing complex 3d models)
//! and RGS (native Fyroxed format) formats are supported.
use crate::animation::AnimationHolder;
use crate::{
    animation::{Animation, AnimationContainer},
    asset::{define_new_resource, Resource, ResourceData},
    core::{
        pool::Handle,
        reflect::prelude::*,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::{
        resource_manager::{options::ImportOptions, ResourceManager},
        SerializationContext,
    },
    resource::fbx::{self, error::FbxError},
    scene::{
        graph::{map::NodeHandleMap, Graph},
        node::Node,
        Scene, SceneLoader,
    },
    utils::log::{Log, MessageKind},
};
use fyrox_core::variable::reset_inheritable_properties;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub(crate) enum NodeMapping {
    UseNames = 0,
    UseHandles = 1,
}

/// See module docs.
#[derive(Debug, Visit)]
pub struct ModelData {
    pub(crate) path: PathBuf,
    #[visit(skip)]
    pub(crate) mapping: NodeMapping,
    #[visit(skip)]
    scene: Scene,
}

define_new_resource!(
    /// See module docs.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    Model<ModelData, ModelLoadError>
);

impl Model {
    pub(crate) fn instantiate_from(
        model: Self,
        model_data: &ModelData,
        handle: Handle<Node>,
        dest_graph: &mut Graph,
    ) -> (Handle<Node>, NodeHandleMap) {
        let (root, old_to_new) =
            model_data
                .scene
                .graph
                .copy_node(handle, dest_graph, &mut |_, _| true);

        // Notify instantiated nodes about resource they were created from.
        let mut stack = vec![root];
        while let Some(node_handle) = stack.pop() {
            let node = &mut dest_graph[node_handle];

            node.resource = Some(model.clone());

            // Reset resource instance root flag, this is needed because a node after instantiation cannot
            // be a root anymore.
            node.is_resource_instance_root = false;

            // Reset inheritable properties, so property inheritance system will take properties
            // from parent objects on resolve stage.
            reset_inheritable_properties(node.as_reflect_mut());

            // Continue on children.
            stack.extend_from_slice(node.children());
        }

        // Fill original handles to instances.
        for (&old, &new) in old_to_new.inner().iter() {
            dest_graph[new].original_handle_in_resource = old;
        }

        (root, old_to_new)
    }

    /// Tries to instantiate model from given resource. Does not retarget available
    /// animations from model to its instance. Can be helpful if you only need geometry.
    pub fn instantiate_geometry(&self, dest_scene: &mut Scene) -> Handle<Node> {
        let data = self.data_ref();

        let instance_root = Self::instantiate_from(
            self.clone(),
            &*data,
            data.scene.graph.get_root(),
            &mut dest_scene.graph,
        )
        .0;
        dest_scene.graph[instance_root].is_resource_instance_root = true;

        // Embed navmeshes.
        // TODO: This also must provide a map which will make it possible to extract navmesh
        // from resource later on.

        for navmesh in data.scene.navmeshes.iter() {
            dest_scene.navmeshes.add(navmesh.clone());
        }

        std::mem::drop(data);

        instance_root
    }

    /// Tries to instantiate model from given resource.
    /// Returns root handle to node of model instance along with available animations
    pub fn instantiate(&self, dest_scene: &mut Scene) -> ModelInstance {
        let root = self.instantiate_geometry(dest_scene);
        ModelInstance {
            root,
            animations: self.retarget_animations(root, dest_scene),
        }
    }

    pub(crate) fn retarget_animations_internal(
        &self,
        root: Handle<Node>,
        graph: &Graph,
        animations: &mut AnimationContainer,
    ) -> Vec<Handle<Animation>> {
        let data = self.data_ref();
        let mut animation_handles = Vec::new();

        for ref_anim in data.scene.animations.iter() {
            let mut anim_copy = ref_anim.clone();

            anim_copy.set_root(root);

            // Keep reference to resource from which this animation was taken from. This will help
            // us to correctly reload keyframes for each track when we'll be loading a save file.
            anim_copy.resource = AnimationHolder::Model(Some(self.clone()));

            // Remap animation track nodes from resource to instance. This is required
            // because we've made a plain copy and it has tracks with node handles mapped
            // to nodes of internal scene.
            for (i, ref_track) in ref_anim.tracks().iter().enumerate() {
                let ref_node = &data.scene.graph[ref_track.target()];
                // Find instantiated node that corresponds to node in resource
                let instance_node = graph.find_by_name(root, ref_node.name());
                if instance_node.is_none() {
                    Log::writeln(
                        MessageKind::Error,
                        format!(
                            "Failed to retarget animation {:?} for node {}",
                            data.path(),
                            ref_node.name()
                        ),
                    );
                }
                // One-to-one track mapping so there is [i] indexing.
                anim_copy.tracks_mut()[i].set_target(instance_node);
            }

            animation_handles.push(animations.add(anim_copy));
        }

        animation_handles
    }

    /// Tries to retarget animations from given model resource to a node hierarchy starting
    /// from `root` on a given scene.
    ///
    /// Animation retargeting allows you to "transfer" animation from a model to a model
    /// instance on a scene. Imagine you have a character that should have multiple animations
    /// like idle, run, shoot, walk, etc. and you want to store each animation in a separate
    /// file. Then when you creating a character on a level you want to have all possible
    /// animations assigned to a character, this is where this function comes into play:
    /// you just load a model of your character with skeleton, but without any animations,
    /// then you load several "models" which have only skeleton with some animation (such
    /// "models" can be considered as "animation" resources). After this you need to
    /// instantiate model on your level and retarget all animations you need to that instance
    /// from other "models". All you have after this is a handle to a model and bunch of
    /// handles to specific animations. After this animations can be blended in any combinations
    /// you need to. For example idle animation can be blended with walk animation when your
    /// character starts walking.
    ///
    /// # Notes
    ///
    /// Most of the 3d model formats can contain only one animation, so in most cases
    /// this function will return vector with only one animation.
    pub fn retarget_animations(
        &self,
        root: Handle<Node>,
        dest_scene: &mut Scene,
    ) -> Vec<Handle<Animation>> {
        self.retarget_animations_internal(root, &dest_scene.graph, &mut dest_scene.animations)
    }
}

impl ResourceData for ModelData {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl Default for ModelData {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            mapping: NodeMapping::UseNames,
            scene: Scene::new(),
        }
    }
}

/// Defines a way of searching materials when loading a model resource from foreign file format such as FBX.
///
/// # Motivation
///
/// Most 3d model file formats store paths to external resources (textures and other things) as absolute paths,
/// which makes it impossible to use with "location-independent" application like games. To fix that issue, the
/// engine provides few ways of resolving paths to external resources. The engine starts resolving by stripping
/// everything but file name from an external resource's path, then it uses one of the following methods to find
/// a texture with the file name. It could look up on folders hierarchy by using [`MaterialSearchOptions::RecursiveUp`]
/// method, or even use global search starting from the working directory of your game
/// ([`MaterialSearchOptions::WorkingDirectory`])
#[derive(
    Clone,
    Debug,
    Visit,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Reflect,
    AsRefStr,
    EnumString,
    EnumVariantNames,
)]
pub enum MaterialSearchOptions {
    /// Search in specified materials directory. It is suitable for cases when
    /// your model resource use shared textures.
    ///
    /// # Platform specific
    ///
    /// Works on every platform.
    MaterialsDirectory(PathBuf),

    /// Recursive-up search. It is suitable for cases when textures are placed
    /// near your model resource. This is **default** option.
    ///
    /// # Platform specific
    ///
    /// Works on every platform.
    RecursiveUp,

    /// Global search starting from working directory. Slowest option with a lot of ambiguities -
    /// it may load unexpected file in cases when there are two or more files with same name but
    /// lying in different directories.
    ///
    /// # Platform specific
    ///
    /// WebAssembly - **not supported** due to lack of file system.
    WorkingDirectory,

    /// Try to use paths stored in the model resource directly. This options has limited usage,
    /// it is suitable to load animations, or any other model which does not have any materials.
    ///
    /// # Important notes
    ///
    /// RGS (native engine scenes) files should be loaded with this option by default, otherwise
    /// the engine won't be able to correctly find materials.
    UsePathDirectly,
}

impl Default for MaterialSearchOptions {
    fn default() -> Self {
        Self::RecursiveUp
    }
}

impl MaterialSearchOptions {
    /// A helper to create MaterialsDirectory variant.
    pub fn materials_directory<P: AsRef<Path>>(path: P) -> Self {
        Self::MaterialsDirectory(path.as_ref().to_path_buf())
    }
}

/// A set of options that will be applied to a model resource when loading it from external source.
///
/// # Details
///
/// The engine has a convenient way of storing import options in a `.options` files. For example you may
/// have a `foo.fbx` 3d model, to change import options create a new file with additional `.options`
/// extension: `foo.fbx.options`. The content of an options file could be something like this:
///
/// ```text
/// (
///     material_search_options: RecursiveUp
/// )
/// ```
///
/// Check documentation of the field of the structure for more info about each parameter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Reflect, Eq)]
pub struct ModelImportOptions {
    /// See [`MaterialSearchOptions`] docs for more info.
    #[serde(default)]
    pub material_search_options: MaterialSearchOptions,
}

impl ImportOptions for ModelImportOptions {}

/// Model instance is a combination of handle to root node of instance in a scene,
/// and list of all animations from model which were instantiated on a scene.
#[derive(Debug)]
pub struct ModelInstance {
    /// Handle of root node of instance.
    pub root: Handle<Node>,

    /// List of instantiated animations that were inside model resource.
    /// You must free them when you do not need model anymore
    pub animations: Vec<Handle<Animation>>,
}

/// All possible errors that may occur while trying to load model from some
/// data source.
#[derive(Debug)]
pub enum ModelLoadError {
    /// An error occurred while reading a data source.
    Visit(VisitError),
    /// Format is not supported.
    NotSupported(String),
    /// An error occurred while loading FBX file.
    Fbx(FbxError),
}

impl Display for ModelLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelLoadError::Visit(v) => {
                write!(f, "An error occurred while reading a data source {v:?}")
            }
            ModelLoadError::NotSupported(v) => {
                write!(f, "Model format is not supported: {v}")
            }
            ModelLoadError::Fbx(v) => v.fmt(f),
        }
    }
}

impl From<FbxError> for ModelLoadError {
    fn from(fbx: FbxError) -> Self {
        ModelLoadError::Fbx(fbx)
    }
}

impl From<VisitError> for ModelLoadError {
    fn from(e: VisitError) -> Self {
        ModelLoadError::Visit(e)
    }
}

impl ModelData {
    pub(crate) async fn load<P: AsRef<Path>>(
        path: P,
        serialization_context: Arc<SerializationContext>,
        resource_manager: ResourceManager,
        model_import_options: ModelImportOptions,
    ) -> Result<Self, ModelLoadError> {
        let extension = path
            .as_ref()
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref()
            .to_lowercase();
        let (scene, mapping) = match extension.as_ref() {
            "fbx" => {
                let mut scene = Scene::new();
                if let Some(filename) = path.as_ref().file_name() {
                    let root = scene.graph.get_root();
                    scene.graph[root].set_name(&filename.to_string_lossy());
                }
                fbx::load_to_scene(
                    &mut scene,
                    resource_manager,
                    path.as_ref(),
                    &model_import_options,
                )
                .await?;
                // Set NodeMapping::UseNames as mapping here because FBX does not have
                // any persistent unique ids, and we have to use names.
                (scene, NodeMapping::UseNames)
            }
            // Scene can be used directly as model resource. Such scenes can be created in
            // Fyroxed.
            "rgs" => (
                SceneLoader::from_file(path.as_ref(), serialization_context)
                    .await?
                    .finish(resource_manager)
                    .await,
                NodeMapping::UseHandles,
            ),
            // TODO: Add more formats.
            _ => {
                return Err(ModelLoadError::NotSupported(format!(
                    "Unsupported model resource format: {}",
                    extension
                )))
            }
        };

        Ok(Self {
            path: path.as_ref().to_owned(),
            scene,
            mapping,
        })
    }

    /// Returns shared reference to internal scene, there is no way to obtain
    /// mutable reference to inner scene because resource is immutable source
    /// of data.
    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    /// Tries to find node in resource by its name. Returns Handle::NONE if
    /// no node was found.
    pub fn find_node_by_name(&self, name: &str) -> Handle<Node> {
        self.scene.graph.find_by_name_from_root(name)
    }

    pub(crate) fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
}
