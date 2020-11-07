use crate::core::algebra::{Vector2, Vector3};
use crate::{
    core::pool::Handle,
    renderer::surface::{VertexWeight, VertexWeightSet},
    resource::{
        fbx::scene,
        fbx::{
            document::{FbxNode, FbxNodeContainer},
            error::FbxError,
            scene::{FbxComponent, FbxContainer, FbxScene},
        },
    },
};

pub struct FbxGeometry {
    // Only vertices and indices are required.
    pub vertices: Vec<Vector3<f32>>,
    pub indices: Vec<i32>,

    // Normals, UVs, etc. are optional.
    pub normals: Option<FbxContainer<Vector3<f32>>>,
    pub uvs: Option<FbxContainer<Vector2<f32>>>,
    pub materials: Option<FbxContainer<i32>>,
    pub tangents: Option<FbxContainer<Vector3<f32>>>,
    pub binormals: Option<FbxContainer<Vector3<f32>>>,

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
    geom_node_handle: Handle<FbxNode>,
    nodes: &FbxNodeContainer,
) -> Result<Vec<i32>, FbxError> {
    let indices_node_handle = nodes.find(geom_node_handle, "PolygonVertexIndex")?;
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
) -> Result<Option<FbxContainer<Vector3<f32>>>, FbxError> {
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
) -> Result<Option<FbxContainer<Vector3<f32>>>, FbxError> {
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
) -> Result<Option<FbxContainer<Vector3<f32>>>, FbxError> {
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
) -> Result<Option<FbxContainer<Vector2<f32>>>, FbxError> {
    if let Ok(layer_element_uv) = nodes.find(geom_node_handle, "LayerElementUV") {
        Ok(Some(FbxContainer::new(
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
) -> Result<Option<FbxContainer<i32>>, FbxError> {
    if let Ok(layer_element_material_node_handle) =
        nodes.find(geom_node_handle, "LayerElementMaterial")
    {
        Ok(Some(FbxContainer::new(
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

impl FbxGeometry {
    pub(in crate::resource::fbx) fn read(
        geom_node_handle: Handle<FbxNode>,
        nodes: &FbxNodeContainer,
    ) -> Result<FbxGeometry, FbxError> {
        Ok(FbxGeometry {
            vertices: read_vertices(geom_node_handle, nodes)?,
            indices: read_indices(geom_node_handle, nodes)?,
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
                let sub_deformer = scene.get(sub_deformer_handle).as_sub_deformer()?;
                for (index, weight) in sub_deformer.weights.iter() {
                    let bone_set = out
                        .get_mut(*index as usize)
                        .ok_or(FbxError::IndexOutOfBounds)?;
                    if !bone_set.push(VertexWeight {
                        value: *weight,
                        effector: sub_deformer.model.into(),
                    }) {
                        // Re-normalize weights if there are more than 4 bones per vertex.
                        bone_set.normalize();
                    }
                }
            }
        }
        Ok(out)
    }
}
