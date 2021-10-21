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
        FieldKind::Collection(ref collection_changed) => match args.name.as_ref() {
            Terrain::LAYERS => match &**collection_changed {
                CollectionChanged::Add => {
                    helper.do_scene_command(AddTerrainLayerCommand::new(handle, graph))
                }
                CollectionChanged::Remove(index) => {
                    helper.do_scene_command(DeleteTerrainLayerCommand::new(handle, *index))
                }
                CollectionChanged::ItemChanged { index, property } => {
                    assert_eq!(property.owner_type_id, TypeId::of::<Layer>());
                    match property.value {
                        FieldKind::Object(ref args) => match property.name.as_ref() {
                            Layer::MASK_PROPERTY_NAME => {
                                helper.do_scene_command(SetTerrainLayerMaskPropertyNameCommand {
                                    handle,
                                    layer_index: *index,
                                    value: args.cast_value::<String>().cloned()?,
                                });
                                Some(())
                            }
                            _ => None,
                        },
                        _ => None,
                    }
                }
            },
            _ => None,
        },
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Terrain::DECAL_LAYER_INDEX => {
                do_command!(helper, SetTerrainDecalLayerIndexCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Terrain::BASE => handle_base_property_changed(&inner, handle, node, helper),
            _ => None,
        },
    }
}
