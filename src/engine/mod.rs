use crate::scene::*;
use crate::utils::pool::*;
use crate::renderer::renderer::*;
use crate::resource::*;
use std::rc::*;
use std::cell::*;
use std::path::*;
use crate::resource::texture::*;

pub struct ResourceManager {
    resources: Vec<Rc<RefCell<Resource>>>,
    /// Path to textures, extensively used for resource files
    /// which stores path in weird format (either relative or absolute) which
    /// is obviously not good for engine.
    textures_path: PathBuf
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        ResourceManager {
            resources: Vec::new(),
            textures_path: PathBuf::from("data/textures/")
        }
    }

    pub fn request_texture(&mut self, path: &Path) -> Option<Rc<RefCell<Resource>>> {
        for existing in self.resources.iter() {
            let resource = existing.borrow_mut();
            if resource.path == path {
                if let ResourceKind::Texture(_) = resource.borrow_kind() {
                    return Some(existing.clone());
                } else {
                    println!("Resource with path {:?} found but it is not a texture!", path);
                    return None;
                }
            }
        }

        // Texture was not loaded before, try to load and register
        if let Ok(texture) = Texture::load(path) {
            println!("Texture {:?} loaded", path);
            let resource = Rc::new(RefCell::new(
                Resource::new(path, ResourceKind::Texture(texture))));
            self.resources.push(resource.clone());
            return Some(resource.clone());
        }

        // Fail
        None
    }

    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }
}

pub struct Engine {
    renderer: Renderer,
    scenes: Pool<Scene>,
    events: Vec<glutin::Event>,
    running: bool,
    resource_manager: ResourceManager,
}

impl Engine {
    #[inline]
    pub fn new() -> Engine {
        Engine {
            scenes: Pool::new(),
            renderer: Renderer::new(),
            events: Vec::new(),
            running: true,
            resource_manager: ResourceManager::new()
        }
    }

    #[inline]
    pub fn add_scene(&mut self, scene: Scene) -> Handle<Scene> {
        self.scenes.spawn(scene)
    }

    #[inline]
    pub fn borrow_scene(&self, handle: &Handle<Scene>) -> Option<&Scene> {
        if let Some(scene) = self.scenes.borrow(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn borrow_scene_mut(&mut self, handle: &Handle<Scene>) -> Option<&mut Scene> {
        if let Some(scene) = self.scenes.borrow_mut(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    #[inline]
    pub fn get_resource_manager(&mut self) -> &mut ResourceManager {
        &mut self.resource_manager
    }

    pub fn update(&mut self) {
        let client_size = self.renderer.context.get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        for i in 0..self.scenes.capacity() {
            if let Some(scene) = self.scenes.at_mut(i) {
                scene.update(aspect_ratio);
            }
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
        self.renderer.upload_resources(&mut self.resource_manager.resources);

        let mut alive_scenes: Vec<&Scene> = Vec::new();
        for i in 0..self.scenes.capacity() {
            if let Some(scene) = self.scenes.at(i) {
                alive_scenes.push(scene);
            }
        }
        self.renderer.render(alive_scenes.as_slice());
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
