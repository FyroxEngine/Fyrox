use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
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
            FieldKind::Object(ref value) => match args.name.as_ref() {
                Mesh::CAST_SHADOWS => {
                    make_command!(SetMeshCastShadowsCommand, handle, value)
                }
                Mesh::RENDER_PATH => {
                    make_command!(SetMeshRenderPathCommand, handle, value)
                }
                Mesh::DECAL_LAYER_INDEX => {
                    make_command!(SetMeshDecalLayerIndexCommand, handle, value)
                }
                _ => None,
            },
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
