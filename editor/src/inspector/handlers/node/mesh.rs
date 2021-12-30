use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::mesh::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{mesh::Mesh, node::Node},
};

pub fn handle_mesh_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    node: &Node,
) -> Option<SceneCommand> {
    if let Node::Mesh(_) = node {
        match args.value {
            FieldKind::Object(ref value) => {
                handle_properties!(args.name.as_ref(), handle, value,
                    Mesh::CAST_SHADOWS => SetMeshCastShadowsCommand,
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
