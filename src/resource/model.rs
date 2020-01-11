use crate::{
    scene::{
        Scene,
        node::Node,
        SceneInterface,
        SceneInterfaceMut,
        base::AsBase
    },
    animation::Animation,
    resource::{fbx, fbx::error::FbxError},
    engine::resource_manager::ResourceManager,
    core::{
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    utils::log::Log
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        Mutex,
        Weak
    }
};

pub struct Model {
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

pub struct ModelInstance {
    pub root: Handle<Node>,
    pub animations: Vec<Handle<Animation>>,
}

fn upgrade_self_weak_ref(self_weak_ref: &Option<Weak<Mutex<Model>>>) -> Arc<Mutex<Model>> {
    // This .expect will never be triggered in normal conditions because there is only
    // one way to get resource - through resource manager which always returns Arc and
    // sets correct self ref.
    let self_weak_ref = self_weak_ref
        .as_ref()
        .expect("Model self weak ref cannon be None!");

    self_weak_ref
        .upgrade()
        .expect("Model self weak ref must be valid!")
}

impl Model {
    pub(in crate) fn load<P: AsRef<Path>>(path: P, resource_manager: &mut ResourceManager) -> Result<Model, FbxError> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, resource_manager, path.as_ref())?;
        Ok(Model {
            self_weak_ref: None,
            path: path.as_ref().to_path_buf(),
            scene,
        })
    }

    /// Tries to instantiate model from given resource. Does not retarget available
    /// animations from model to its instance. Can be helpful if you only need geometry.
    pub fn instantiate_geometry(&self, dest_scene: &mut Scene) -> Handle<Node> {
        let SceneInterfaceMut { graph: dest_graph, .. } = dest_scene.interface_mut();
        let SceneInterface { graph: resource_graph, .. } = self.scene.interface();

        let root = resource_graph.copy_node(resource_graph.get_root(), dest_graph);
        dest_graph.get_mut(root).base_mut().is_resource_instance = true;

        // Notify instantiated nodes about resource they were created from.
        let mut stack = Vec::new();
        stack.push(root);
        while let Some(node_handle) = stack.pop() {
            let node = dest_graph.get_mut(node_handle);

            node.base_mut().resource = Some(upgrade_self_weak_ref(&self.self_weak_ref));

            // Continue on children.
            for child_handle in node.base().get_children() {
                stack.push(child_handle.clone());
            }
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
    pub fn retarget_animations(&self, root: Handle<Node>, dest_scene: &mut Scene) -> Vec<Handle<Animation>> {
        let mut animation_handles = Vec::new();

        let SceneInterface {
            animations: resource_animations,
            graph: resource_graph, ..
        } = self.scene.interface();

        for ref_anim in resource_animations.iter() {
            let mut anim_copy = ref_anim.clone();

            // Keep reference to resource from which this animation was taken from. This will help
            // us to correctly reload keyframes for each track when we'll be loading a save file.
            anim_copy.resource = Some(upgrade_self_weak_ref(&self.self_weak_ref));

            let SceneInterfaceMut {
                animations: dest_animations,
                graph: dest_graph, ..
            } = dest_scene.interface_mut();

            // Remap animation track nodes from resource to instance. This is required
            // because we've made a plain copy and it has tracks with node handles mapped
            // to nodes of internal scene.
            for (i, ref_track) in ref_anim.get_tracks().iter().enumerate() {
                let ref_node = resource_graph.get(ref_track.get_node());
                // Find instantiated node that corresponds to node in resource
                let instance_node = dest_graph.find_by_name(root, ref_node.base().get_name());
                if instance_node.is_none() {
                    Log::writeln(format!("Failed to retarget animation for node {}", ref_node.base().get_name()));
                }
                // One-to-one track mapping so there is [i] indexing.
                anim_copy.get_tracks_mut()[i].set_node(instance_node);
            }

            animation_handles.push(dest_animations.add(anim_copy));
        }

        animation_handles
    }

    /// Returns internal scene
    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    pub fn find_node_by_name(&self, name: &str) -> Handle<Node> {
        let SceneInterface { graph, .. } = self.scene.interface();
        graph.find_by_name_from_root(name)
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        if self.path.exists() {
            Log::writeln(format!("Model resource {:?} destroyed!", self.path));
        }
    }
}