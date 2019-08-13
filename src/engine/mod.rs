use crate::{
    scene::*,
    utils::{
        pool::*,
        rcpool::{
            RcPool,
            RcHandle
        }
    },
    renderer::{
        renderer::*,
        surface::SurfaceSharedData
    },
    resource::{
        *,
        texture::*,
        model::Model,
        ttf::Font
    },
    gui::UserInterface,
    math::vec2::Vec2
};
use std::{
    path::*,
    collections::VecDeque,
    time::Duration
};
use serde::{
    Serialize,
    Deserialize
};
use std::any::{Any, TypeId};

#[derive(Serialize, Deserialize)]
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

    #[inline]
    pub fn for_each_texture_mut<Func>(&mut self, mut func: Func) where Func: FnMut(&mut Texture) {
        for resource in self.resources.iter_mut() {
            if let ResourceKind::Texture(texture) = resource.borrow_kind_mut() {
                func(texture);
            }
        }
    }

    #[inline]
    fn add_resource(&mut self, resource: Resource) -> RcHandle<Resource> {
        self.resources.spawn(resource)
    }

    /// Searches for a resource of specified path, if found - returns handle to resource
    /// and increases reference count of resource.
    #[inline]
    fn find_resource(&mut self, path: &Path) -> RcHandle<Resource> {
        for i in 0..self.resources.get_capacity() {
            if let Some(resource) = self.resources.at(i) {
                if resource.get_path() == path {
                    return self.resources.handle_from_index(i);
                }
            }
        }
        RcHandle::none()
    }

    #[inline]
    pub fn borrow_resource(&self, resource_handle: &RcHandle<Resource>) -> Option<&Resource> {
        self.resources.borrow(resource_handle)
    }

    #[inline]
    pub fn borrow_resource_mut(&mut self, resource_handle: &RcHandle<Resource>) -> Option<&mut Resource> {
        self.resources.borrow_mut(resource_handle)
    }

    #[inline]
    pub fn share_resource_handle(&self, resource_handle: &RcHandle<Resource>) -> RcHandle<Resource> {
        self.resources.share_handle(resource_handle)
    }

    #[inline]
    #[must_use]
    pub fn release_resource(&mut self, resource_handle: &RcHandle<Resource>) -> Option<Resource> {
        self.resources.release(resource_handle)
    }

    #[inline]
    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    scenes: Pool<Scene>,
    #[serde(skip)]
    surf_data_storage: RcPool<SurfaceSharedData>,
    resource_manager: ResourceManager,
}

impl State {
    #[inline]
    pub fn new() -> Self {
        State {
            scenes: Pool::new(),
            resource_manager: ResourceManager::new(),
            surf_data_storage: RcPool::new(),
        }
    }

    /// Returns handle of existing resource, or if resource is not loaded yet,
    /// loads it and returns it handle. If resource could not be loaded, returns
    /// none handle.
    pub fn request_resource(&mut self, path: &Path) -> RcHandle<Resource> {
        let mut resource_handle = self.resource_manager.find_resource(path);

        if resource_handle.is_none() {
            // No such resource, try to load it.
            let extension = path.extension().
                and_then(|os| os.to_str()).
                map_or(String::from(""), |s| s.to_ascii_lowercase());

            resource_handle = match extension.as_str() {
                "jpg" | "jpeg" | "png" | "tif" | "tiff" | "tga" | "bmp" => {
                    match Texture::load(path) {
                        Ok(texture) => {
                            self.resource_manager.add_resource(Resource::new(path, ResourceKind::Texture(texture)))
                        }
                        Err(_) => {
                            println!("Unable to load texture {}!", path.display());
                            RcHandle::none()
                        }
                    }
                }
                "fbx" => {
                    match Model::load(path, self) {
                        Ok(model) => {
                           self.resource_manager.add_resource(Resource::new(path, ResourceKind::Model(model)))
                        }
                        Err(_) => {
                            println!("Unable to load model from {}!", path.display());
                            RcHandle::none()
                        }
                    }
                }
                _ => {
                    println!("Unknown resource type {}!", path.display());
                    RcHandle::none()
                }
            }
        }

        if resource_handle.is_some() {
            println!("Resource {} is loaded!", path.display());
        }

        resource_handle
    }

    #[inline]
    pub fn release_resource(&mut self, handle: &RcHandle<Resource>) {
        if let Some(mut resource) = self.resource_manager.release_resource(handle) {
            match resource.borrow_kind_mut() {
                ResourceKind::Model(model) => {
                    self.destroy_scene_internal(model.get_scene_mut());
                }
                ResourceKind::Texture(texture) => ()
            }
            println!("Resource destroyed: {}!", resource.get_path().display());
        }
    }

    fn clear(&mut self) {
        for i in 0..self.scenes.get_capacity() {
            if let Some(mut scene) = self.scenes.take_at(i) {
                self.destroy_scene_internal(&mut scene);
            }
        }

        if self.surf_data_storage.alive_count() != 0 {
            println!("Not all shared surface data was freed! {} left alive!", self.surf_data_storage.alive_count());
        }
    }

    fn resolve(&mut self) {
        // Reload all resources first.
        for i in 0..self.resource_manager.resources.get_capacity() {
            let path;
            let id;
            if let Some(resource) = self.resource_manager.resources.at(i) {
                path = PathBuf::from(resource.get_path());
                match resource.borrow_kind() {
                    ResourceKind::Model(model) => id = model.type_id(),
                    ResourceKind::Texture(texture) => id = texture.type_id()
                }
            } else {
                continue
            }

            let handle = self.resource_manager.resources.handle_from_index(i);
            if id == TypeId::of::<Model>() {
                let model = Model::load(path.as_path(), self).unwrap();
                let resource = Resource::new(path.as_path(), ResourceKind::Model(model));
                self.resource_manager.resources.replace(&handle, resource);
            } else if id == TypeId::of::<Texture>() {

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
    pub fn get_surface_data_storage(&self) -> &RcPool<SurfaceSharedData> {
        &self.surf_data_storage
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
    pub fn get_surface_data_storage_mut(&mut self) -> &mut RcPool<SurfaceSharedData> {
        &mut self.surf_data_storage
    }

    #[inline]
    pub fn add_scene(&mut self, scene: Scene) -> Handle<Scene> {
        self.scenes.spawn(scene)
    }

    #[inline]
    pub fn get_scene(&self, handle: &Handle<Scene>) -> Option<&Scene> {
        if let Some(scene) = self.scenes.borrow(handle) {
            return Some(scene);
        }
        None
    }

    #[inline]
    pub fn get_scene_mut(&mut self, handle: &Handle<Scene>) -> Option<&mut Scene> {
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
    pub fn destroy_scene(&mut self, handle: &Handle<Scene>) {
        if let Some(mut scene) = self.scenes.take(handle) {
            self.destroy_scene_internal(&mut scene);
        }
    }
}

pub struct Engine {
    renderer: Renderer,
    pub state: State,
    events: VecDeque<glutin::Event>,
    running: bool,
    font_cache: Pool<Font>,
    default_font: Handle<Font>,
    user_interface: UserInterface,
}

impl Engine {
    #[inline]
    pub fn new() -> Engine {
        let mut font_cache = Pool::new();
        let default_font = font_cache.spawn(Font::load(
            Path::new("data/fonts/font.ttf"),
            20.0,
            (0..255).collect()).unwrap());
        let mut renderer = Renderer::new();
        renderer.upload_font_cache(&mut font_cache);
        Engine {
            state: State::new(),
            renderer,
            events: VecDeque::new(),
            running: true,
            user_interface: UserInterface::new(default_font.clone()),
            default_font,
            font_cache,
        }
    }

    #[inline]
    pub fn get_state(&self) -> &State {
        &self.state
    }

    #[inline]
    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn update(&mut self, dt: f64) {
        let client_size = self.renderer.context.get_inner_size().unwrap();
        let aspect_ratio = (client_size.width / client_size.height) as f32;

        for scene in self.state.scenes.iter_mut() {
            scene.update(aspect_ratio, dt);
        }

        self.user_interface.update(&Vec2::make(client_size.width as f32, client_size.height as f32));
    }

    pub fn poll_events(&mut self) {
        // Gather events
        let events = &mut self.events;
        events.clear();
        self.renderer.events_loop.poll_events(|event| {
            events.push_back(event);
        });
    }

    #[inline]
    pub fn get_font(&self, font_handle: &Handle<Font>) -> Option<&Font> {
        self.font_cache.borrow(font_handle)
    }

    #[inline]
    pub fn get_default_font(&self) -> Handle<Font> {
        self.default_font.clone()
    }

    #[inline]
    pub fn get_ui(&self) -> &UserInterface {
        &self.user_interface
    }

    #[inline]
    pub fn get_ui_mut(&mut self) -> &mut UserInterface {
        &mut self.user_interface
    }

    pub fn get_rendering_statisting(&self) -> &Statistics {
        self.renderer.get_statistics()
    }

    pub fn save<W>(&self, writer: W) where W: std::io::Write {
        serde_json::to_writer_pretty(writer, &self.state).unwrap();
    }

    pub fn load<R>(&mut self, reader: R) where R: std::io::Read {
        self.state.clear();
        self.state = serde_json::from_reader(reader).unwrap();
        self.state.resolve();
    }

    pub fn render(&mut self) {
        self.renderer.upload_font_cache(&mut self.font_cache);
        self.renderer.upload_resources(&mut self.state);
        self.user_interface.draw(&self.font_cache);
        self.renderer.render(&self.state, &self.user_interface.get_drawing_context());
    }

    #[inline]
    pub fn stop(&mut self) {
        self.running = false;
    }

    #[inline]
    pub fn pop_event(&mut self) -> Option<glutin::Event> {
        self.events.pop_front()
    }
}

pub fn duration_to_seconds_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0
}

pub fn duration_to_seconds_f32(duration: Duration) -> f32 {
    duration_to_seconds_f64(duration) as f32
}
