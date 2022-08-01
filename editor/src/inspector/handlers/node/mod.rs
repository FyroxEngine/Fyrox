use std::any::TypeId;

use crate::{
    inspector::handlers::node::terrain::handle_terrain_property_changed,
    scene::commands::make_set_node_property_command, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::PropertyChanged,
    scene::{node::Node, terrain::Terrain},
};

pub mod base;
pub mod terrain;
pub mod transform;

pub struct SceneNodePropertyChangedHandler;

impl SceneNodePropertyChangedHandler {
    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &mut Node,
    ) -> Option<SceneCommand> {
        if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, handle, node)
        } else {
            Some(make_set_node_property_command(handle, args))
        }
    }
}
