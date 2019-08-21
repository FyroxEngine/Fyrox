use crate::{
    scene::*,
    utils::{
        pool::*,
        visitor::{
            Visitor,
            VisitResult,
            Visit,
        },
    },
    renderer::render::*,
    resource::{
        *,
        texture::*,
        model::Model,
        ttf::Font,
    },
    gui::UserInterface,
    math::vec2::Vec2,
};
use std::{
    path::*,
    collections::VecDeque,
    time::Duration,
    cell::RefCell,
    rc::Rc,
};
use crate::scene::node::NodeKind;
use std::any::TypeId;

pub struct ResourceManager {
    resources: Vec<Rc<RefCell<Resource>>>,
    /// Path to textures, extensively used for resource files
    /// which stores path in weird format (either relative or absolute) which
    /// is obviously not good for engine.
    textures_path: PathBuf,
}

impl ResourceManager {
    pub fn new() -> ResourceManager {
        Self {
            resources: Vec::new(),
            textures_path: PathBuf::from("data/textures/"),
        }
    }

    #[inline]
    pub fn for_each_texture_mut<Func>(&self, mut func: Func) where Func: FnMut(&mut Texture) {
        for resource in self.resources.iter() {
            if let ResourceKind::Texture(texture) = resource.borrow_mut().borrow_kind_mut() {
                func(texture);
            }
        }
    }

    #[inline]
    fn add_resource(&mut self, resource: Rc<RefCell<Resource>>) {
        self.resources.push(resource)
    }

    /// Searches for a resource of specified path, if found - returns handle to resource
    /// and increases reference count of resource.
    #[inline]
    fn find_resource(&mut self, path: &Path) -> Option<Rc<RefCell<Resource>>> {
        for resource in self.resources.iter() {
            if resource.borrow().get_path() == path {
                return Some(resource.clone());
            }
        }
        None
    }

    #[inline]
    pub fn get_textures_path(&self) -> &Path {
        self.textures_path.as_path()
    }

    pub fn update(&mut self) {
        self.resources.retain(|resource| {
            Rc::strong_count(resource) > 1
        })
    }
}

impl Visit for ResourceManager {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.resources.visit("Resources", visitor)?;

        visitor.leave_region()
    }
}

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
                        Err(_) => {
                            println!("Unable to load texture {}!", path.display());
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
                        Err(_) => {
                            println!("Unable to load model from {}!", path.display());
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

    fn resolve(&mut self) {
        let resources_to_reload = self.resource_manager.resources.clone();

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

        for scene in self.scenes.iter_mut() {
            for node in scene.nodes.iter_mut() {
                let node_name = String::from(node.get_name());
                if let Some(resource) = node.get_resource() {
                    if let NodeKind::Mesh(mesh) = node.borrow_kind_mut() {
                        if let ResourceKind::Model(model) = resource.borrow().borrow_kind() {
                            let resource_node_handle = model.find_node_by_name(node_name.as_str());
                            if let Some(resource_node) = model.get_scene().get_node(&resource_node_handle) {
                                if let NodeKind::Mesh(resource_mesh) = resource_node.borrow_kind() {
                                    let surfaces = mesh.get_surfaces_mut();
                                    surfaces.clear();
                                    for resource_surface in resource_mesh.get_surfaces() {
                                        surfaces.push(resource_surface.make_copy());
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

impl Visit for State {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.resource_manager.visit("ResourceManager", visitor)?;
        self.scenes.visit("Scenes", visitor)?;

        visitor.leave_region()
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

        self.state.resource_manager.update();

        for scene in self.state.scenes.iter_mut() {
            scene.update(aspect_ratio, dt);
        }

        self.user_interface.update(Vec2::make(client_size.width as f32, client_size.height as f32));
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

impl Visit for Engine {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Make sure to delete unused resources.
        if visitor.is_reading() {
            self.state.resource_manager.update();
            self.state.scenes.clear();
        }

        self.state.visit("State", visitor)?;

        if visitor.is_reading() {
            self.state.resolve();
        }

        visitor.leave_region()
    }
}

pub fn duration_to_seconds_f64(duration: Duration) -> f64 {
    duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000.0
}

pub fn duration_to_seconds_f32(duration: Duration) -> f32 {
    duration_to_seconds_f64(duration) as f32
}
