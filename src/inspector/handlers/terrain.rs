use crate::{
    inspector::SenderHelper,
    scene::commands::terrain::{AddTerrainLayerCommand, DeleteTerrainLayerCommand},
};
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::Graph, node::Node},
};

pub fn handle_terrain_property_changed(
    args: &PropertyChanged,
    node_handle: Handle<Node>,
    helper: &SenderHelper,
    graph: &Graph,
) {
    match args.name.as_ref() {
        "layers" => {
            if let FieldKind::Collection(ref collection_changed) = args.value {
                match &**collection_changed {
                    CollectionChanged::Add => {
                        helper.do_scene_command(AddTerrainLayerCommand::new(node_handle, graph))
                    }
                    CollectionChanged::Remove(index) => {
                        helper.do_scene_command(DeleteTerrainLayerCommand::new(node_handle, *index))
                    }
                    CollectionChanged::ItemChanged { .. } => {
                        // Nothing to do.
                    }
                }
            }
        }
        _ => println!("Unhandled property of Camera: {:?}", args),
    }
}
