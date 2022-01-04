use rg3d::{
    core::inspect::Inspect, engine::resource_manager::ResourceManager,
    gui::inspector::PropertyChanged,
};

pub mod model;
pub mod texture;

pub trait ImportOptionsHandler {
    fn apply(&self, resource_manager: ResourceManager);
    fn revert(&mut self);
    fn value(&self) -> &dyn Inspect;
    fn handle_property_changed(&mut self, property_changed: &PropertyChanged);
}
