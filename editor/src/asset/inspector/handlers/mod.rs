use fyrox::{
    core::reflect::prelude::*, engine::resource_manager::ResourceManager,
    gui::inspector::PropertyChanged,
};

pub mod model;
pub mod sound;
pub mod texture;

pub trait ImportOptionsHandler {
    fn apply(&self, resource_manager: ResourceManager);
    fn revert(&mut self);
    fn value(&self) -> &dyn Reflect;
    fn handle_property_changed(&mut self, property_changed: &PropertyChanged);
}
