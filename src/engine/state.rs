use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use crate::{
    resource::{
        texture::Texture,
        model::Model,
    },
    engine::resource_manager::ResourceManager,
    scene::Scene,
};
use rg3d_core::{
    pool::{Pool, Handle},
    visitor::{Visit, Visitor, VisitResult},
};
use rg3d_sound::buffer::{Buffer, BufferKind};

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

    pub fn request_texture(&mut self, path: &Path) -> Option<Arc<Mutex<Texture>>> {
        if let Some(texture) = self.resource_manager.find_texture(path) {
            return Some(texture);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "jpg" | "jpeg" | "png" | "tif" | "tiff" | "tga" | "bmp" => match Texture::load(path) {
                Ok(texture) => {
                    let shared_texture = Arc::new(Mutex::new(texture));
                    self.resource_manager.add_texture(shared_texture.clone());
                    println!("Texture {} is loaded!", path.display());
                    Some(shared_texture)
                }
                Err(e) => {
                    println!("Unable to load texture {}! Reason {}", path.display(), e);
                    None
                }
            }
            _ => {
                println!("Unsupported texture type {}!", path.display());
                None
            }
        }
    }

    pub fn request_model(&mut self, path: &Path) -> Option<Arc<Mutex<Model>>> {
        if let Some(model) = self.resource_manager.find_model(path) {
            return Some(model);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "fbx" => match Model::load(path, self) {
                Ok(model) => {
                    let model = Arc::new(Mutex::new(model));
                    self.resource_manager.add_model(model.clone());
                    println!("Model {} is loaded!", path.display());
                    Some(model)
                }
                Err(e) => {
                    println!("Unable to load model from {}! Reason {}", path.display(), e);
                    None
                }
            },
            _ => {
                println!("Unsupported model type {}!", path.display());
                None
            }
        }
    }

    pub fn request_sound_buffer(&mut self, path: &Path, kind: BufferKind) -> Option<Arc<Mutex<Buffer>>> {
        if let Some(sound_buffer) = self.resource_manager.find_sound_buffer(path) {
            return Some(sound_buffer);
        }

        let extension = path.extension().
            and_then(|os| os.to_str()).
            map_or(String::from(""), |s| s.to_ascii_lowercase());

        match extension.as_str() {
            "wav" => match Buffer::new(path, kind) {
                Ok(sound_buffer) => {
                    let sound_buffer = Arc::new(Mutex::new(sound_buffer));
                    self.resource_manager.add_sound_buffer(sound_buffer.clone());
                    println!("Model {} is loaded!", path.display());
                    Some(sound_buffer)
                }
                Err(e) => {
                    println!("Unable to load sound buffer from {}! Reason {}", path.display(), e);
                    None
                }
            },
            _ => {
                println!("Unsupported sound buffer type {}!", path.display());
                None
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

    pub(in crate::engine) fn resolve(&mut self) {
        println!("Starting resolve stage...\nReloading resources...");
        // Reload resources first.
        for old_texture in self.resource_manager.get_textures() {
            let mut old_texture = old_texture.lock().unwrap();
            let new_texture = match Texture::load(old_texture.path.as_path()) {
                Ok(texture) => texture,
                Err(e) => {
                    println!("Unable to reload {:?} texture! Reason: {}", old_texture.path, e);
                    continue;
                }
            };

            *old_texture = new_texture;
        }

        for old_model in self.resource_manager.get_models().to_vec() {
            let mut old_model = old_model.lock().unwrap();
            let new_model = match Model::load(old_model.path.as_path(), self) {
                Ok(new_model) => new_model,
                Err(e) => {
                    println!("Unable to reload {:?} model! Reason: {}", old_model.path, e);
                    continue;
                }
            };

            *old_model = new_model;
        }

        for old_sound_buffer in self.resource_manager.get_sound_buffers() {
            let mut old_sound_buffer = old_sound_buffer.lock().unwrap();
            let new_sound_buffer = match Buffer::new(old_sound_buffer.get_source_path(), old_sound_buffer.get_kind()) {
                Ok(new_sound_buffer) => new_sound_buffer,
                Err(e) => {
                    println!("Unable to reload {:?} sound buffer! Reason: {}", old_sound_buffer.get_source_path(), e);
                    continue;
                }
            };

            *old_sound_buffer = new_sound_buffer;
        }

        println!("Resolving scenes...");

        for scene in self.scenes.iter_mut() {
            scene.resolve();
        }

        println!("Resolve successful!");
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