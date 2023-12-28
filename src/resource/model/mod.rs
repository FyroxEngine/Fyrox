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

use crate::{
    asset::{
        manager::ResourceManager, options::ImportOptions, Resource, ResourceData,
        MODEL_RESOURCE_UUID,
    },
    core::{
        algebra::{UnitQuaternion, Vector3},
        log::{Log, MessageKind},
        pool::Handle,
        reflect::prelude::*,
        uuid::Uuid,
        visitor::{Visit, VisitError, VisitResult, Visitor},
        TypeUuidProvider,
    },
    engine::SerializationContext,
    resource::fbx::{self, error::FbxError},
    scene::{
        animation::{Animation, AnimationPlayer},
        graph::{map::NodeHandleMap, Graph},
        node::Node,
        Scene, SceneLoader,
    },
};
use fyrox_core::uuid_provider;
use fyrox_resource::io::ResourceIo;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{
    any::Any,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

pub mod loader;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Reflect)]
#[repr(u32)]
pub(crate) enum NodeMapping {
    UseNames = 0,
    UseHandles = 1,
}

/// See module docs.
#[derive(Debug, Visit, Reflect)]
pub struct Model {
    #[visit(skip)]
    pub(crate) mapping: NodeMapping,
    #[visit(skip)]
    pub(crate) scene: Scene,
}

impl TypeUuidProvider for Model {
    fn type_uuid() -> Uuid {
        MODEL_RESOURCE_UUID
    }
}

/// Type alias for model resources.
pub type ModelResource = Resource<Model>;

/// Extension trait for model resources.
pub trait ModelResourceExtension: Sized {
    /// Tries to instantiate model from given resource.
    fn instantiate_from(
        model: ModelResource,
        model_data: &Model,
        handle: Handle<Node>,
        dest_graph: &mut Graph,
    ) -> (Handle<Node>, NodeHandleMap);

    /// Tries to instantiate model from given resource.
    fn instantiate(&self, dest_scene: &mut Scene) -> Handle<Node>;

    /// Instantiates a prefab and places it at specified position and orientation in global coordinates.
    fn instantiate_at(
        &self,
        scene: &mut Scene,
        position: Vector3<f32>,
        orientation: UnitQuaternion<f32>,
    ) -> Handle<Node>;

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
    fn retarget_animations_directly(&self, root: Handle<Node>, graph: &Graph) -> Vec<Animation>;

    /// Tries to retarget animations from given model resource to a node hierarchy starting
    /// from `root` on a given scene. Unlike [`Self::retarget_animations_directly`], it automatically
    /// adds retargetted animations to the specified animation player in the hierarchy of given `root`.
    ///
    /// # Panic
    ///
    /// Panics if `dest_animation_player` is invalid handle, or the node does not have [`AnimationPlayer`]
    /// component.
    fn retarget_animations_to_player(
        &self,
        root: Handle<Node>,
        dest_animation_player: Handle<Node>,
        graph: &mut Graph,
    ) -> Vec<Handle<Animation>>;

    /// Tries to retarget animations from given model resource to a node hierarchy starting
    /// from `root` on a given scene. Unlike [`Self::retarget_animations_directly`], it automatically
    /// adds retargetted animations to a first animation player in the hierarchy of given `root`.
    ///
    /// # Panic
    ///
    /// Panics if there's no animation player in the given hierarchy (descendant nodes of `root`).
    fn retarget_animations(&self, root: Handle<Node>, graph: &mut Graph) -> Vec<Handle<Animation>>;
}

impl ModelResourceExtension for ModelResource {
    fn instantiate_from(
        model: ModelResource,
        model_data: &Model,
        handle: Handle<Node>,
        dest_graph: &mut Graph,
    ) -> (Handle<Node>, NodeHandleMap) {
        let (root, old_to_new) = model_data.scene.graph.copy_node(
            handle,
            dest_graph,
            &mut |_, _| true,
            &mut |_, original_handle, node| {
                node.set_inheritance_data(original_handle, model.clone());
            },
        );

        dest_graph.update_hierarchical_data_for_descendants(root);

        (root, old_to_new)
    }

    fn instantiate(&self, dest_scene: &mut Scene) -> Handle<Node> {
        let data = self.data_ref();

        let instance_root = Self::instantiate_from(
            self.clone(),
            &data,
            data.scene.graph.get_root(),
            &mut dest_scene.graph,
        )
        .0;

        // Explicitly mark as root node.
        dest_scene.graph[instance_root].is_resource_instance_root = true;

        std::mem::drop(data);

        instance_root
    }

    fn instantiate_at(
        &self,
        scene: &mut Scene,
        position: Vector3<f32>,
        orientation: UnitQuaternion<f32>,
    ) -> Handle<Node> {
        let root = self.instantiate(scene);

        scene.graph[root]
            .local_transform_mut()
            .set_position(position)
            .set_rotation(orientation);

        scene.graph.update_hierarchical_data_for_descendants(root);

        root
    }

    fn retarget_animations_directly(&self, root: Handle<Node>, graph: &Graph) -> Vec<Animation> {
        let mut retargetted_animations = Vec::new();

        let mut header = self.state();
        let self_kind = header.kind().clone();
        if let Some(model) = header.data() {
            for src_node_ref in model.scene.graph.linear_iter() {
                if let Some(src_player) = src_node_ref.query_component_ref::<AnimationPlayer>() {
                    for src_anim in src_player.animations().iter() {
                        let mut anim_copy = src_anim.clone();

                        // Remap animation track nodes from resource to instance. This is required
                        // because we've made a plain copy and it has tracks with node handles mapped
                        // to nodes of internal scene.
                        for (i, ref_track) in src_anim.tracks().iter().enumerate() {
                            let ref_node = &model.scene.graph[ref_track.target()];
                            let track = &mut anim_copy.tracks_mut()[i];
                            // Find instantiated node that corresponds to node in resource
                            match graph.find_by_name(root, ref_node.name()) {
                                Some((instance_node, _)) => {
                                    // One-to-one track mapping so there is [i] indexing.
                                    track.set_target(instance_node);
                                }
                                None => {
                                    track.set_target(Default::default());
                                    Log::writeln(
                                        MessageKind::Error,
                                        format!(
                                            "Failed to retarget animation {:?} for node {}",
                                            self_kind,
                                            ref_node.name()
                                        ),
                                    );
                                }
                            }
                        }

                        retargetted_animations.push(anim_copy);
                    }
                }
            }
        }

        retargetted_animations
    }

    fn retarget_animations_to_player(
        &self,
        root: Handle<Node>,
        dest_animation_player: Handle<Node>,
        graph: &mut Graph,
    ) -> Vec<Handle<Animation>> {
        let mut animation_handles = Vec::new();

        let animations = self.retarget_animations_directly(root, graph);

        let dest_animation_player = graph[dest_animation_player]
            .query_component_mut::<AnimationPlayer>()
            .unwrap();

        for animation in animations {
            animation_handles.push(dest_animation_player.animations_mut().add(animation));
        }

        animation_handles
    }

    fn retarget_animations(&self, root: Handle<Node>, graph: &mut Graph) -> Vec<Handle<Animation>> {
        if let Some((animation_player, _)) = graph.find(root, &mut |n| {
            n.query_component_ref::<AnimationPlayer>().is_some()
        }) {
            self.retarget_animations_to_player(root, animation_player, graph)
        } else {
            Default::default()
        }
    }
}

impl ResourceData for Model {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.scene.save("Scene", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

impl Default for Model {
    fn default() -> Self {
        Self {
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

uuid_provider!(MaterialSearchOptions = "11634aa0-cf8f-4532-a8cd-c0fa6ef804f1");

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

impl Model {
    pub(crate) async fn load<P: AsRef<Path>>(
        path: P,
        io: &dyn ResourceIo,
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
                    io,
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
                SceneLoader::from_file(
                    path.as_ref(),
                    io,
                    serialization_context,
                    resource_manager.clone(),
                )
                .await?
                .0
                .finish(&resource_manager)
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

        Ok(Self { scene, mapping })
    }

    /// Returns shared reference to internal scene, there is no way to obtain
    /// mutable reference to inner scene because resource is immutable source
    /// of data.
    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    /// Searches for a node in the model, starting from specified node using the specified closure. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    pub fn find_node_by_name(&self, name: &str) -> Option<(Handle<Node>, &Node)> {
        self.scene.graph.find_by_name_from_root(name)
    }

    pub(crate) fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
}
