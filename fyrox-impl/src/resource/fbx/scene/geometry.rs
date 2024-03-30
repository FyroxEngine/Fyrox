use crate::resource::fbx::scene::FbxBlendShapeChannel;
use crate::{
    core::{
        algebra::{Vector2, Vector3},
        pool::Handle,
    },
    resource::fbx::{
        document::{FbxNode, FbxNodeContainer},
        error::FbxError,
        scene::{self, attributes_to_vec3_array, FbxComponent, FbxLayerElement, FbxScene},
    },
    scene::mesh::surface::{VertexWeight, VertexWeightSet},
};
use fxhash::FxHashMap;

pub struct FbxMeshGeometry {
    // Only vertices and indices are required.
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<i32>,

    // Normals, UVs, etc. are optional.
    pub normals: Option<FbxLayerElement<Vector3<f32>>>,
    pub uvs: Option<FbxLayerElement<Vector2<f32>>>,
    pub materials: Option<FbxLayerElement<i32>>,
    pub tangents: Option<FbxLayerElement<Vector3<f32>>>,
    #[allow(dead_code)] // TODO: Use binormals.
    pub binormals: Option<FbxLayerElement<Vector3<f32>>>,

    pub deformers: Vec<Handle<FbxComponent>>,
}

fn read_vertices(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Vec<Vector3<f32>>, FbxError> {
    let vertices_node_handle = nodes.find(geom_node_handle, "Vertices")?;
    let vertices_array_node = nodes.get_by_name(vertices_node_handle, "a")?;
    let mut vertices = Vec::with_capacity(vertices_array_node.attrib_count() / 3);
    for vertex in vertices_array_node.attributes().chunks_exact(3) {
        vertices.push(Vector3::new(
            vertex[0].as_f32()?,
            vertex[1].as_f32()?,
            vertex[2].as_f32()?,
        ));
    }

    Ok(vertices)
}

fn read_indices(
    name: &str,
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Vec<i32>, FbxError> {
    let indices_node_handle = nodes.find(geom_node_handle, name)?;
    let indices_array_node = nodes.get_by_name(indices_node_handle, "a")?;
    let mut indices = Vec::with_capacity(indices_array_node.attrib_count());
    for index in indices_array_node.attributes() {
        indices.push(index.as_i32()?);
    }
    Ok(indices)
}

fn read_normals(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<FbxLayerElement<Vector3<f32>>>, FbxError> {
    if let Ok(layer_element_normal) = nodes.find(geom_node_handle, "LayerElementNormal") {
        Ok(Some(scene::make_vec3_container(
            nodes,
            layer_element_normal,
            "Normals",
        )?))
    } else {
        Ok(None)
    }
}

fn read_tangents(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<FbxLayerElement<Vector3<f32>>>, FbxError> {
    if let Ok(layer_element_tangent) = nodes.find(geom_node_handle, "LayerElementTangent") {
        Ok(Some(scene::make_vec3_container(
            nodes,
            layer_element_tangent,
            "Tangents",
        )?))
    } else {
        Ok(None)
    }
}

fn read_binormals(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<FbxLayerElement<Vector3<f32>>>, FbxError> {
    if let Ok(layer_element_tangent) = nodes.find(geom_node_handle, "LayerElementBinormal") {
        Ok(Some(scene::make_vec3_container(
            nodes,
            layer_element_tangent,
            "Binormals",
        )?))
    } else {
        Ok(None)
    }
}

fn read_uvs(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<FbxLayerElement<Vector2<f32>>>, FbxError> {
    if let Ok(layer_element_uv) = nodes.find(geom_node_handle, "LayerElementUV") {
        Ok(Some(FbxLayerElement::new(
            nodes,
            layer_element_uv,
            "UV",
            |attributes| {
                let mut uvs = Vec::with_capacity(attributes.len() / 2);
                for uv in attributes.chunks_exact(2) {
                    uvs.push(Vector2::new(uv[0].as_f32()?, uv[1].as_f32()?));
                }
                Ok(uvs)
            },
        )?))
    } else {
        Ok(None)
    }
}

fn read_materials(
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<FbxLayerElement<i32>>, FbxError> {
    if let Ok(layer_element_material_node_handle) =
        nodes.find(geom_node_handle, "LayerElementMaterial")
    {
        Ok(Some(FbxLayerElement::new(
            nodes,
            layer_element_material_node_handle,
            "Materials",
            |attributes| {
                let mut materials = Vec::with_capacity(attributes.len());
                for attribute in attributes {
                    materials.push(attribute.as_i32()?);
                }
                Ok(materials)
            },
        )?))
    } else {
        Ok(None)
    }
}

impl FbxMeshGeometry {
    pub(in crate::resource::fbx) fn read(
        geom_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, FbxError> {
        Ok(Self {
            vertices: read_vertices(geom_node_handle, nodes)?,
            indices: read_indices("PolygonVertexIndex", geom_node_handle, nodes)?,
            normals: read_normals(geom_node_handle, nodes)?,
            uvs: read_uvs(geom_node_handle, nodes)?,
            materials: read_materials(geom_node_handle, nodes)?,
            tangents: read_tangents(geom_node_handle, nodes)?,
            binormals: read_binormals(geom_node_handle, nodes)?,
            deformers: Vec::new(),
        })
    }

    pub(in crate::resource::fbx) fn get_skin_data(
        &self,
        scene: &FbxScene,
    ) -> Result<Vec<VertexWeightSet>, FbxError> {
        let mut out = vec![VertexWeightSet::default(); self.vertices.len()];
        for &deformer_handle in self.deformers.iter() {
            for &sub_deformer_handle in scene
                .get(deformer_handle)
                .as_deformer()?
                .sub_deformers
                .iter()
            {
                // We must check for cluster here, because skinned meshes can also have blend shape deformers.
                if let FbxComponent::Cluster(cluster) = scene.get(sub_deformer_handle) {
                    for (index, weight) in cluster.weights.iter() {
                        let bone_set = out
                            .get_mut(*index as usize)
                            .ok_or(FbxError::IndexOutOfBounds)?;
                        if !bone_set.push(VertexWeight {
                            value: *weight,
                            effector: cluster.model.into(),
                        }) {
                            // Re-normalize weights if there are more than 4 bones per vertex.
                            bone_set.normalize();
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn collect_blend_shapes_refs<'a>(
        &self,
        scene: &'a FbxScene,
    ) -> Result<Vec<&'a FbxBlendShapeChannel>, FbxError> {
        let mut blend_shapes = Vec::new();
        for &deformer_handle in &self.deformers {
            let deformer = scene.get(deformer_handle).as_deformer()?;
            for &sub_deformer in &deformer.sub_deformers {
                if let FbxComponent::BlendShapeChannel(channel) = scene.get(sub_deformer) {
                    blend_shapes.push(channel);
                }
            }
        }
        Ok(blend_shapes)
    }
}

// According to the docs, shape geometry is similar to mesh geometry, but all containers have "index to direct" mapping
// (plain arrays), so we'll store all data in simple Vecs. Shape geometry is used a source of "delta" data for blend
// shapes, so we'll only use some common data here: vertex positions, normals, tangents and binormals.
pub struct FbxShapeGeometry {
    // Only vertices and indices are required.
    pub vertices: Vec<Vector3<f32>>,
    pub indices: FxHashMap<i32, i32>,
    // The rest is optional.
    pub normals: Option<Vec<Vector3<f32>>>,
    pub tangents: Option<Vec<Vector3<f32>>>,
    #[allow(dead_code)] // TODO: Use binormals.
    pub binormals: Option<Vec<Vector3<f32>>>,
}

fn read_vec3_plain_array(
    name: &str,
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Option<Vec<Vector3<f32>>>, FbxError> {
    if let Ok(layer_element_normal) = nodes.find(geom_node_handle, name) {
        let array_node = nodes.get_by_name(layer_element_normal, "a")?;
        Ok(Some(attributes_to_vec3_array(array_node.attributes())?))
    } else {
        Ok(None)
    }
}

impl FbxShapeGeometry {
    pub(in crate::resource::fbx) fn read(
        geom_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<Self, FbxError> {
        Ok(Self {
            vertices: read_vec3_plain_array("Vertices", geom_node_handle, nodes)?
                .ok_or_else(|| "No vertices element!".to_string())?,
            indices: read_indices("Indexes", geom_node_handle, nodes)?
                .into_iter()
                .enumerate()
                .map(|(k, i)| (i, k as i32))
                .collect(),
            normals: read_vec3_plain_array("Normals", geom_node_handle, nodes)?,
            tangents: read_vec3_plain_array("Tangents", geom_node_handle, nodes)?,
            binormals: read_vec3_plain_array("Binormals", geom_node_handle, nodes)?,
        })
    }
}
