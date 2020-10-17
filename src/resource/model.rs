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
//! and RGS (native rusty-editor format) formats are supported.
use crate::{
    animation::Animation,
    core::{
        pool::Handle,
        visitor::{Visit, VisitError, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::{fbx, fbx::error::FbxError},
    scene::{node::Node, Scene},
    utils::log::Log,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
};

/// See module docs.
#[derive(Debug)]
pub struct Model {
    // enable_shared_from_this trick from C++
    pub(in crate) self_weak_ref: Option<Weak<Mutex<Model>>>,
    pub(in crate) path: PathBuf,
    scene: Scene,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            self_weak_ref: None,
            path: PathBuf::new(),
            scene: Scene::new(),
        }
    }
}

impl Visit for Model {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.self_weak_ref.visit("SelfWeakRef", visitor)?;
        self.path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

/// Model instance is a combination of handle to root node of instance in a scene,
/// and list of all animations from model which were instantiated on a scene.
pub struct ModelInstance {
    /// Handle of root node of instance.
    pub root: Handle<Node>,

    /// List of instantiated animations that were inside model resource.
    /// You must free them when you do not need model anymore
    pub animations: Vec<Handle<Animation>>,
}

fn upgrade_self_weak_ref(self_weak_ref: &Option<Weak<Mutex<Model>>>) -> Arc<Mutex<Model>> {
    // This .expect will never be triggered in normal conditions because there is only
    // one way to get resource - through resource manager which always returns Arc and
    // sets correct self ref.
    let self_weak_ref = self_weak_ref
        .as_ref()
        .expect("Model self weak ref cannot be None!");

    self_weak_ref
        .upgrade()
        .expect("Model self weak ref must be valid!")
}

/// All possible errors that may occur while trying to load model from some
/// data source.
#[derive(Debug)]
pub enum ModelLoadError {
    /// An error occurred while reading some data source.
    Visit(VisitError),
    /// Format is not supported.
    NotSupported(String),
    /// An error occurred while loading FBX file.
    Fbx(FbxError),
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
    pub(in crate) fn load<P: AsRef<Path>>(
        path: P,
        resource_manager: &mut ResourceManager,
    ) -> Result<Model, ModelLoadError> {
        let extension = path
            .as_ref()
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref()
            .to_lowercase();
        let scene = match extension.as_ref() {
            "fbx" => {
                let mut scene = Scene::new();
                fbx::load_to_scene(&mut scene, resource_manager, path.as_ref())?;
                scene
            }
            // Scene can be used directly as model resource. Such scenes can be created from
            // rusty-editor (https://github.com/mrDIMAS/rusty-editor) for example.
            "rgs" => Scene::from_file(path.as_ref(), resource_manager)?,
            // TODO: Add more formats.
            _ => {
                return Err(ModelLoadError::NotSupported(format!(
                    "Unsupported model resource format: {}",
                    extension
                )))
            }
        };

        Ok(Model {
            self_weak_ref: None,
            path: path.as_ref().to_owned(),
            scene,
        })
    }

    /// Tries to instantiate model from given resource. Does not retarget available
    /// animations from model to its instance. Can be helpful if you only need geometry.
    pub fn instantiate_geometry(&self, dest_scene: &mut Scene) -> Handle<Node> {
        let (root, _) = self.scene.graph.copy_node(
            self.scene.graph.get_root(),
            &mut dest_scene.graph,
            &mut |_, _| true,
        );
        dest_scene.graph[root].is_resource_instance = true;

        // Notify instantiated nodes about resource they were created from.
        let mut stack = Vec::new();
        stack.push(root);
        while let Some(node_handle) = stack.pop() {
            let node = &mut dest_scene.graph[node_handle];

            node.resource = Some(upgrade_self_weak_ref(&self.self_weak_ref));

            // Continue on children.
            stack.extend_from_slice(node.children());
        }

        root
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

    /// Tries to retarget animations from given model resource to a node hierarchy starting
    /// from `root` on a given scene.
    ///
    /// Animation retargetting allows you to "transfer" animation from a model to a model
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
        let mut animation_handles = Vec::new();

        for ref_anim in self.scene.animations.iter() {
            let mut anim_copy = ref_anim.clone();

            // Keep reference to resource from which this animation was taken from. This will help
            // us to correctly reload keyframes for each track when we'll be loading a save file.
            anim_copy.resource = Some(upgrade_self_weak_ref(&self.self_weak_ref));

            // Remap animation track nodes from resource to instance. This is required
            // because we've made a plain copy and it has tracks with node handles mapped
            // to nodes of internal scene.
            for (i, ref_track) in ref_anim.get_tracks().iter().enumerate() {
                let ref_node = &self.scene.graph[ref_track.get_node()];
                // Find instantiated node that corresponds to node in resource
                let instance_node = dest_scene.graph.find_by_name(root, ref_node.name());
                if instance_node.is_none() {
                    Log::writeln(format!(
                        "Failed to retarget animation {:?} for node {}",
                        self.path,
                        ref_node.name()
                    ));
                }
                // One-to-one track mapping so there is [i] indexing.
                anim_copy.get_tracks_mut()[i].set_node(instance_node);
            }

            animation_handles.push(dest_scene.animations.add(anim_copy));
        }

        animation_handles
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
}
