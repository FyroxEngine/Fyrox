use crate::{
    utils::pool::Handle,
    scene::{
        node::Node,
        Scene
    },
    engine::State,
    resource::ResourceKind,
    math::vec3::Vec3,
    game::GameTime
};
use std::path::Path;

pub enum WeaponKind {
    M4,
    Ak47,
}

pub struct Weapon {
    kind: WeaponKind,
    model: Handle<Node>,
    offset: Vec3,
    dest_offset: Vec3,
    last_shot_time: f64
}

impl Weapon {
    pub fn new(kind: WeaponKind, state: &mut State, scene: &mut Scene) -> Weapon {
        let model_path = match kind {
            WeaponKind::Ak47 => Path::new("data/models/ak47.fbx"),
            WeaponKind::M4 => Path::new("data/models/m4.fbx"),
        };

        let mut weapon_model = Handle::none();
        let model_resource_handle = state.request_resource(model_path);
        if let Some(resource) = state.get_resource_manager().borrow_resource(&model_resource_handle) {
            if let ResourceKind::Model(model) = resource.borrow_kind() {
                weapon_model = model.instantiate(state, scene);
            }
        }

        Weapon {
            kind,
            model: weapon_model,
            offset: Vec3::new(),
            dest_offset: Vec3::new(),
            last_shot_time: 0.0,
        }
    }

    #[inline]
    pub fn get_model(&self) -> Handle<Node> {
        self.model.clone()
    }

    pub fn update(&mut self, scene: &mut Scene) {
        self.offset.x += (self.dest_offset.x - self.offset.x) * 0.2;
        self.offset.y += (self.dest_offset.y - self.offset.y) * 0.2;
        self.offset.z += (self.dest_offset.z - self.offset.z) * 0.2;

        if let Some(node) = scene.get_node_mut(&self.model) {
            node.set_local_position(self.offset);
        }
    }

    pub fn shoot(&mut self, time: &GameTime) {
        if time.elapsed - self.last_shot_time >= 0.1 {
            self.offset = Vec3::make(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;
        }
    }
}