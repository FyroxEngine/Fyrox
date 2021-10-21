use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{do_command, inspector::SenderHelper, scene::commands::terrain::*};
use rg3d::{
    core::pool::Handle,
    gui::message::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::Graph, node::Node, terrain::Layer, terrain::Terrain},
};
use std::any::TypeId;

pub fn handle_terrain_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
    helper: &SenderHelper,
    graph: &Graph,
) -> Option<()> {
    match args.value {
        FieldKind::Collection(ref collection_changed) => {
            if let Terrain::LAYERS = args.name.as_ref() {
                match &**collection_changed {
                    CollectionChanged::Add => {
                        helper.do_scene_command(AddTerrainLayerCommand::new(handle, graph))
                    }
                    CollectionChanged::Remove(index) => {
                        helper.do_scene_command(DeleteTerrainLayerCommand::new(handle, *index))
                    }
                    CollectionChanged::ItemChanged { index, property } => {
                        assert_eq!(property.owner_type_id, TypeId::of::<Layer>());
                        if let FieldKind::Object(ref args) = property.value {
                            match property.name.as_ref() {
                                "mask_property_name" => helper.do_scene_command(
                                    SetTerrainLayerMaskPropertyNameCommand {
                                        handle,
                                        layer_index: *index,
                                        value: args.cast_value::<String>().cloned()?,
                                    },
                                ),
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
        FieldKind::Object(ref value) => {
            if let Terrain::DECAL_LAYER_INDEX = args.name.as_ref() {
                do_command!(helper, SetTerrainDecalLayerIndexCommand, handle, value)
            }
        }
        FieldKind::Inspectable(ref inner) => {
            if let Terrain::BASE = args.name.as_ref() {
                handle_base_property_changed(&inner, handle, node, helper)?
            }
        }
    }
    Some(())
}
