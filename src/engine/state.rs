use std::{
    rc::Rc,
    cell::RefCell,
    path::{Path, PathBuf},
    any::TypeId,
};

use crate::{
    resource::{
        texture::Texture,
        Resource,
        ResourceKind,
        model::Model,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        Scene,
        node::NodeKind,
        node::Node,
    },
    utils::{
        pool::{Pool, Handle},
        visitor::{Visit, Visitor, VisitResult},
    },
};

pub struct State {
    scenes: Pool<Scene>,
    resource_manager: ResourceManager,
}

impl State {
    #[inline]
    pub fn new() -> Self {
        State {
            scenes: Pool::new(),
            resource_manager: ResourceManager::new(),
        }
    }

    pub fn request_resource(&mut self, path: &Path) -> Option<Rc<RefCell<Resource>>> {
        match self.resource_manager.find_resource(path) {
            Some(resource) => Some(resource),
            None => {
                // No such resource, try to load it.
                let extension = path.extension().
                    and_then(|os| os.to_str()).
                    map_or(String::from(""), |s| s.to_ascii_lowercase());

                match extension.as_str() {
                    "jpg" | "jpeg" | "png" | "tif" | "tiff" | "tga" | "bmp" => match Texture::load(path) {
                        Ok(texture) => {
                            let resource = Rc::new(RefCell::new(Resource::new(path, ResourceKind::Texture(texture))));
                            self.resource_manager.add_resource(resource.clone());
                            println!("Texture {} is loaded!", path.display());
                            Some(resource)
                        }
                        Err(e) => {
                            println!("Unable to load texture {}! Reason {}", path.display(), e);
                            None
                        }
                    }
                    "fbx" => match Model::load(path, self) {
                        Ok(model) => {
                            let resource = Rc::new(RefCell::new(Resource::new(path, ResourceKind::Model(model))));
                            self.resource_manager.add_resource(resource.clone());
                            println!("Model {} is loaded!", path.display());
                            Some(resource)
                        }
                        Err(e) => {
                            println!("Unable to load model from {}! Reason {}", path.display(), e);
                            None
                        }
                    },
                    _ => {
                        println!("Unknown resource type {}!", path.display());
                        None
                    }
                }
            }
        }
    }

    fn clear(&mut self) {
        for i in 0..self.scenes.get_capacity() {
            if let Some(mut scene) = self.scenes.take_at(i) {
                self.destroy_scene_internal(&mut scene);
            }
        }
    }

    fn find_model_root(scene: &Scene, from: Handle<Node>) -> Handle<Node> {
        let mut model_root_handle = from;
        while let Some(model_node) = scene.get_nodes().borrow(model_root_handle) {
            if let Some(model_node_resource) = model_node.get_resource() {
                if let Some(parent) = scene.get_nodes().borrow(model_node.get_parent()) {
                    if let Some(parent_resource) = parent.get_resource() {
                        if !Rc::ptr_eq(&parent_resource, &model_node_resource) {
                            // Parent node uses different resource than current root node.
                            return model_root_handle;
                        }
                    } else {
                        return model_root_handle;
                    }
                } else {
                    // We have no parent on node, then it must be root.
                    return model_root_handle;
                }
            }
            // Continue searching up on hierarchy.
            model_root_handle = model_node.get_parent();
        }
        model_root_handle
    }

    pub(in crate::engine) fn resolve(&mut self) {
        // Reload resources first.
        let resources_to_reload = self.resource_manager.get_resources().to_vec();
        for resource in resources_to_reload {
            let path = PathBuf::from(resource.borrow().get_path());
            let id = resource.borrow().get_kind_id();

            if id == TypeId::of::<Model>() {
                let new_model = match Model::load(path.as_path(), self) {
                    Ok(new_model) => new_model,
                    Err(e) => {
                        println!("Unable to reload {:?} model! Reason: {}", path, e);
                        continue;
                    }
                };

                if let ResourceKind::Model(model) = resource.borrow_mut().borrow_kind_mut() {
                    *model = new_model;
                }
            } else if id == TypeId::of::<Texture>() {
                let new_texture = match Texture::load(path.as_path()) {
                    Ok(texture) => texture,
                    Err(e) => {
                        println!("Unable to reload {:?} texture! Reason: {}", path, e);
                        continue;
                    }
                };

                if let ResourceKind::Texture(texture) = resource.borrow_mut().borrow_kind_mut() {
                    *texture = new_texture;
                }
            }
        }

        // Resolve original handles. Original handle is a handle to a node in resource from which
        // a node was instantiated from. We can resolve it only by names of nodes, but this is not
        // reliable way of doing this, because some editors allow nodes to have same names for
        // objects, but here we'll assume that modellers will not create models with duplicated
        // names.
        for scene in self.scenes.iter_mut() {
            for node in scene.get_nodes_mut().iter_mut() {
                if node.get_original_handle().is_none() {
                    if let Some(resource) = node.get_resource() {
                        if let ResourceKind::Model(model) = resource.borrow().borrow_kind() {
                            for i in 0..model.get_scene().get_nodes().get_capacity() {
                                if let Some(resource_node) = model.get_scene().get_nodes().at(i) {
                                    if resource_node.get_name() == node.get_name() {
                                        node.set_original_handle(model.get_scene().get_nodes().handle_from_index(i));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Then iterate over all scenes and resolve changes in surface data, remap bones, etc.
        // This step is needed to take correct graphical data from resource, we do not store
        // meshes in save files, just references to resource this data was taken from. So on
        // resolve stage we just copying surface from resource, do bones remapping. Bones remapping
        // is required stage because we copied surface from resource and bones are mapped to nodes
        // in resource, but we must have them mapped to instantiated nodes on scene. To do that
        // we'll try to find a root for each node, and starting from it we'll find corresponding
        // bone nodes. I know that this sounds too confusing but try to understand it.
        for scene in self.scenes.iter_mut() {
            for i in 0..scene.get_nodes().get_capacity() {
                let node_handle = scene.get_nodes().handle_from_index(i);

                // TODO HACK: Fool borrow checker for now.
                let mscene = unsafe { &mut *(scene as *mut Scene) };

                let root_handle = Self::find_model_root(scene, node_handle);

                if let Some(node) = scene.get_nodes_mut().at_mut(i) {
                    let node_name = String::from(node.get_name());
                    if let Some(resource) = node.get_resource() {
                        if let NodeKind::Mesh(mesh) = node.borrow_kind_mut() {
                            if let ResourceKind::Model(model) = resource.borrow().borrow_kind() {
                                let resource_node_handle = model.find_node_by_name(node_name.as_str());
                                if let Some(resource_node) = model.get_scene().get_node(resource_node_handle) {
                                    if let NodeKind::Mesh(resource_mesh) = resource_node.borrow_kind() {
                                        // Copy surfaces from resource and assign to meshes.
                                        let surfaces = mesh.get_surfaces_mut();
                                        surfaces.clear();
                                        for resource_surface in resource_mesh.get_surfaces() {
                                            surfaces.push(resource_surface.make_copy());
                                        }

                                        // Remap bones
                                        for surface in mesh.get_surfaces_mut() {
                                            for bone_handle in surface.bones.iter_mut() {
                                                *bone_handle = mscene.find_copy_of(root_handle, *bone_handle);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn get_scenes(&self) -> &Pool<Scene> {
        &self.scenes
    }

    #[inline]
    pub fn get_scenes_mut(&mut self) -> &mut Pool<Scene> {
        &mut self.scenes
    }

    #[inline]
    pub fn get_resource_manager_mut(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    #[inline]
    pub fn get_resource_manager(&self) -> &ResourceManager {
        &self.resource_manager
    }

    #[inline]
    pub fn add_scene(&mut self, scene: Scene) -> Handle<Scene> {
        self.scenes.spawn(scene)
    }

    #[inline]
    pub fn get_scene(&self, handle: Handle<Scene>) -> Option<&Scene> {
        if let Some(scene) = self.scenes.borrow(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn get_scene_mut(&mut self, handle: Handle<Scene>) -> Option<&mut Scene> {
        if let Some(scene) = self.scenes.borrow_mut(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    fn destroy_scene_internal(&mut self, scene: &mut Scene) {
        scene.remove_node(scene.get_root(), self);
    }

    #[inline]
    pub fn destroy_scene(&mut self, handle: Handle<Scene>) {
        if let Some(mut scene) = self.scenes.take(handle) {
            self.destroy_scene_internal(&mut scene);
        }
    }
}

impl Visit for State {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.resource_manager.visit("ResourceManager", visitor)?;
        self.scenes.visit("Scenes", visitor)?;

        visitor.leave_region()
    }
}