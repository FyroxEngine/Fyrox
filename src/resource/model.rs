use crate::{
    scene::{Scene, node::Node, animation::Animation},
    resource::{fbx, fbx::error::FbxError},
    engine::resource_manager::ResourceManager,
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use rg3d_core::{
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor},
};

pub struct Model {
    pub(in crate) path: PathBuf,
    scene: Scene,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            scene: Scene::new(),
        }
    }
}

impl Visit for Model {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

pub struct ModelInstance {
    pub root: Handle<Node>,
    pub animations: Vec<Handle<Animation>>,
}

impl Model {
    pub(in crate) fn load(path: &Path, resource_manager: &mut ResourceManager) -> Result<Model, FbxError> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, resource_manager, path)?;
        Ok(Model {
            path: PathBuf::from(path),
            scene,
        })
    }

    /// Tries to instantiate model from given resource. Does not retarget available
    /// animations from model to its instance. Can be helpful if you only need geometry.
    pub fn instantiate_geometry(model_rc: Arc<Mutex<Model>>, dest_scene: &mut Scene) -> Handle<Node> {
        let model = model_rc.lock().unwrap();
        let root = model.scene.copy_node(model.scene.get_root(), dest_scene);

        if let Some(root) = dest_scene.get_node_mut(root) {
            root.is_resource_instance = true;
        }

        // Notify instantiated nodes about resource they were created from.
        let mut stack = Vec::new();
        stack.push(root);
        while let Some(node_handle) = stack.pop() {
            if let Some(node) = dest_scene.get_nodes_mut().borrow_mut(node_handle) {
                node.set_resource(Arc::clone(&model_rc));
                // Continue on children.
                for child_handle in node.get_children() {
                    stack.push(child_handle.clone());
                }
            }
        }

        root
    }

    /// Tries to instantiate model from given resource.
    /// Returns root handle to node of model instance along with available animations
    pub fn instantiate(model: Arc<Mutex<Model>>, dest_scene: &mut Scene) -> ModelInstance {
        let root = Self::instantiate_geometry(model.clone(), dest_scene);
        ModelInstance {
            root,
            animations: Self::retarget_animations(model, root, dest_scene),
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
    /// Most of the 3d model formats can contain only one animation, so in most cases
    /// this function will return vector with only one animation.
    pub fn retarget_animations(model_rc: Arc<Mutex<Model>>, root: Handle<Node>, dest_scene: &mut Scene) -> Vec<Handle<Animation>> {
        let model = model_rc.lock().unwrap();

        let mut animations = Vec::new();

        for ref_anim in model.scene.get_animations().iter() {
            let mut anim_copy = ref_anim.clone();

            // Keep reference to resource from which this animation was taken from. This will help
            // us to correctly reload keyframes for each track when we'll be loading a save file.
            anim_copy.resource = Some(model_rc.clone());

            // Remap animation track nodes from resource to instance. This is required
            // because we've made a plain copy and it has tracks with node handles mapped
            // to nodes of internal scene.
            for (i, ref_track) in ref_anim.get_tracks().iter().enumerate() {
                if let Some(ref_node) = model.scene.get_node(ref_track.get_node()) {
                    // Find instantiated node that corresponds to node in resource
                    let instance_node = dest_scene.find_node_by_name(root, ref_node.get_name());
                    if instance_node.is_none() {
                        println!("Failed to retarget animation for node {}", ref_node.get_name())
                    }
                    // One-to-one track mapping so there is [i] indexing.
                    anim_copy.get_tracks_mut()[i].set_node(instance_node);
                }
            }

            animations.push(dest_scene.add_animation(anim_copy));
        }

        animations
    }

    /// Returns internal scene
    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    pub fn find_node_by_name(&self, name: &str) -> Handle<Node> {
        self.scene.find_node_by_name(self.scene.get_root(), name)
    }
}