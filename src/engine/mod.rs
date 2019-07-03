use crate::scene::*;
use crate::utils::pool::*;
use crate::renderer::renderer::*;
use crate::resource::*;
use std::rc::*;
use std::cell::*;
use std::path::*;
use crate::resource::texture::*;
use serde::{Serialize, Deserialize};
use crate::utils::rcpool::{RcPool, RcHandle};

pub struct ResourceManager {
    resources: RcPool<Resource>,
    /// Path to textures, extensively used for resource files
    /// which stores path in weird format (either relative or absolute) which
    /// is obviously not good for engine.
    textures_path: PathBuf,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self {
            resources: RcPool::new(),
            textures_path: PathBuf::from("data/textures/"),
        }
    }
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager::default()
    }

    pub fn request_texture(&mut self, path: &Path) -> Option<RcHandle<Resource>> {
        for i in 0..self.resources.get_capacity() {
            if let Some(resource) = self.resources.at_mut(i) {
                if resource.path == path {
                    if let ResourceKind::Texture(_) = resource.borrow_kind() {
                        return Some(self.resources.handle_from_index(i).unwrap());
                    } else {
                        println!("Resource with path {:?} found but it is not a texture!", path);
                        return None;
                    }
                }
            }
        }

        // Texture was not loaded before, try to load and register
        if let Ok(texture) = Texture::load(path) {
            println!("Texture {:?} loaded", path);
            return Some(self.resources.spawn(
                Resource::new(path, ResourceKind::Texture(texture))));
        }

        // Fail
        None
    }

    pub fn for_each_texture_mut<Func>(&mut self, mut func: Func) where Func: FnMut(&mut Texture) {
        for resource in self.resources.iter_mut() {
            if let ResourceKind::Texture(texture) = resource.borrow_kind_mut() {
                func(texture);
            }
        }
    }

    pub fn borrow_resource(&self, resource_handle: &RcHandle<Resource>) -> Option<&Resource> {
        self.resources.borrow(resource_handle)
    }

    pub fn borrow_resource_mut(&mut self, resource_handle: &RcHandle<Resource>) -> Option<&mut Resource> {
        self.resources.borrow_mut(resource_handle)
    }

    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    scenes: Pool<Scene>,
}

impl State {
    pub fn new() -> Self {
        State {
            scenes: Pool::new(),
        }
    }
}

pub struct Engine {
    renderer: Renderer,
    state: State,
    events: Vec<glutin::Event>,
    running: bool,
    resource_manager: ResourceManager,
}

impl Engine {
    #[inline]
    pub fn new() -> Engine {
        Engine {
            state: State::new(),
            renderer: Renderer::new(),
            events: Vec::new(),
            running: true,
            resource_manager: ResourceManager::new(),
        }
    }

    #[inline]
    pub fn add_scene(&mut self, scene: Scene) -> Handle<Scene> {
        self.state.scenes.spawn(scene)
    }

    #[inline]
    pub fn borrow_scene(&self, handle: &Handle<Scene>) -> Option<&Scene> {
        if let Some(scene) = self.state.scenes.borrow(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn borrow_scene_mut(&mut self, handle: &Handle<Scene>) -> Option<&mut Scene> {
        if let Some(scene) = self.state.scenes.borrow_mut(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn get_state(&self) -> &State {
        &self.state
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    #[inline]
    pub fn get_resource_manager(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    pub fn update(&mut self, dt: f64) {
        let client_size = self.renderer.context.get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        for scene in self.state.scenes.iter_mut() {
            scene.update(aspect_ratio, dt);
        }
    }

    pub fn poll_events(&mut self) {
        // Gather events
        let events = &mut self.events;
        events.clear();
        self.renderer.events_loop.poll_events(|event| {
            events.push(event);
        });
    }

    pub fn render(&mut self) {
        self.renderer.upload_resources(&mut self.resource_manager);
        self.renderer.render(&self.state.scenes, &self.resource_manager);
    }

    #[inline]
    pub fn stop(&mut self) {
        self.running = false;
    }

    #[inline]
    pub fn pop_event(&mut self) -> Option<glutin::Event> {
        self.events.pop()
    }
}