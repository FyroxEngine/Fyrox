use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::material::shader::SamplerFallback;
use rg3d::material::PropertyValue;
use rg3d::{
    core::pool::Handle,
    resource::texture::Texture,
    scene::{
        graph::Graph,
        mesh::{Mesh, RenderPath},
        node::Node,
    },
};

#[derive(Debug)]
enum TextureSet {
    Single(Texture),
    Multiple(Vec<Option<Texture>>),
}

#[derive(Debug)]
pub struct SetMeshTextureCommand {
    node: Handle<Node>,
    set: TextureSet,
}

impl SetMeshTextureCommand {
    pub fn new(node: Handle<Node>, texture: Texture) -> Self {
        Self {
            node,
            set: TextureSet::Single(texture),
        }
    }
}

impl<'a> Command<'a> for SetMeshTextureCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set Texture".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        if let TextureSet::Single(texture) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let old_set = mesh
                .surfaces_mut()
                .iter()
                .map(|s| {
                    s.material()
                        .lock()
                        .unwrap()
                        .property_ref("diffuseTexture")
                        .and_then(|p| {
                            if let PropertyValue::Sampler { value, .. } = p {
                                value.clone()
                            } else {
                                None
                            }
                        })
                })
                .collect();
            for surface in mesh.surfaces_mut() {
                surface
                    .material()
                    .lock()
                    .unwrap()
                    .set_property(
                        "diffuseTexture",
                        PropertyValue::Sampler {
                            value: Some(texture.clone()),
                            fallback: SamplerFallback::White,
                        },
                    )
                    .unwrap();
            }
            self.set = TextureSet::Multiple(old_set);
        } else {
            unreachable!()
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        if let TextureSet::Multiple(set) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let new_value = mesh.surfaces_mut()[0]
                .material()
                .lock()
                .unwrap()
                .property_ref("diffuseTexture")
                .and_then(|p| {
                    if let PropertyValue::Sampler { value, .. } = p {
                        value.clone()
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(mesh.surfaces_mut().len(), set.len());
            for (surface, old_texture) in mesh.surfaces_mut().iter_mut().zip(set) {
                surface
                    .material()
                    .lock()
                    .unwrap()
                    .set_property(
                        "diffuseTexture",
                        PropertyValue::Sampler {
                            value: old_texture.clone(),
                            fallback: SamplerFallback::White,
                        },
                    )
                    .unwrap();
            }
            self.set = TextureSet::Single(new_value);
        } else {
            unreachable!()
        }
    }
}

define_node_command!(SetMeshCastShadowsCommand("Set Mesh Cast Shadows", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_mesh_mut(), cast_shadows, set_cast_shadows);
});

define_node_command!(SetMeshRenderPathCommand("Set Mesh Render Path", RenderPath) where fn swap(self, node) {
    get_set_swap!(self, node.as_mesh_mut(), render_path, set_render_path);
});

define_node_command!(SetMeshDecalLayerIndexCommand("Set Mesh Decal Layer Index", u8) where fn swap(self, node) {
    get_set_swap!(self, node.as_mesh_mut(), decal_layer_index, set_decal_layer_index);
});
