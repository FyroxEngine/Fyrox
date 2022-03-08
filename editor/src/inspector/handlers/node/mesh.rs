use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::mesh::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{mesh::Mesh, node::Node},
};

pub fn handle_mesh_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &mut Node,
) -> Option<SceneCommand> {
    if node.is_mesh() {
        match args.value {
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    Mesh::RENDER_PATH => SetMeshRenderPathCommand,
                    Mesh::DECAL_LAYER_INDEX => SetMeshDecalLayerIndexCommand
                )
            }
            FieldKind::Collection(ref args) => match **args {
                CollectionChanged::Add => {
                    // TODO
                    None
                }
                CollectionChanged::Remove(_) => {
                    // TODO
                    None
                }
                CollectionChanged::ItemChanged { .. } => {
                    // TODO
                    None
                }
            },
            FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
                Mesh::BASE => handle_base_property_changed(inner, handle, node),
                _ => None,
            },
        }
    } else {
        None
    }
}
