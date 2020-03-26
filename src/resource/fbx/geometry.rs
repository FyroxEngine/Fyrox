use crate::{
    core::{
        math::{
            vec3::Vec3,
            vec2::Vec2
        },
        pool::{Pool, Handle},
    },
    resource::fbx::{
        find_and_borrow_node,
        find_node,
        FbxContainer,
        FbxComponent,
        FbxNode,
        FbxReference,
        string_to_reference,
        string_to_mapping,
        error::FbxError,
    },
    renderer::surface::{
        VertexWeightSet,
        VertexWeight
    },
};

pub struct FbxGeometry {
    pub vertices: Vec<Vec3>,
    pub indices: Vec<i32>,
    pub normals: FbxContainer<Vec3>,
    pub uvs: FbxContainer<Vec2>,
    pub materials: FbxContainer<i32>,
    pub tangents: FbxContainer<Vec3>,
    pub binormals: FbxContainer<Vec3>,
    pub(in crate::resource::fbx) deformers: Vec<Handle<FbxComponent>>,
}

impl FbxGeometry {
    pub(in crate::resource::fbx) fn read(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<FbxGeometry, String> {
        Ok(FbxGeometry {
            vertices: Self::read_vertices(geom_node_handle, nodes)?,
            indices: Self::read_indices(geom_node_handle, nodes)?,
            normals: Self::read_normals(geom_node_handle, nodes)?,
            uvs: Self::read_uvs(geom_node_handle, nodes)?,
            materials: Self::read_materials(geom_node_handle, nodes)?,
            tangents: FbxContainer::default(),
            binormals: FbxContainer::default(),
            deformers: Vec::new(),
        })
    }

    fn read_vertices(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Vec<Vec3>, String> {
        let vertices_node_handle = find_node(nodes, geom_node_handle, "Vertices")?;
        let vertices_array_node = find_and_borrow_node(nodes, vertices_node_handle, "a")?;
        let vertex_count = vertices_array_node.attrib_count() / 3;
        let mut vertices = Vec::with_capacity(vertex_count);
        for i in 0..vertex_count {
            vertices.push(vertices_array_node.get_vec3_at(i * 3)?);
        }
        Ok(vertices)
    }

    fn read_indices(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<Vec<i32>, String> {
        let indices_node_handle = find_node(nodes, geom_node_handle, "PolygonVertexIndex")?;
        let indices_array_node = find_and_borrow_node(nodes, indices_node_handle, "a")?;
        let index_count = indices_array_node.attrib_count();
        let mut indices = Vec::with_capacity(index_count);
        for i in 0..index_count {
            let index = indices_array_node.get_attrib(i)?.as_i32()?;
            indices.push(index);
        }
        Ok(indices)
    }

    fn read_normals(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<FbxContainer<Vec3>, String> {
        if let Ok(layer_element_normal_node_handle) = find_node(nodes, geom_node_handle, "LayerElementNormal") {
            let map_type_node = find_and_borrow_node(nodes, layer_element_normal_node_handle, "MappingInformationType")?;
            let mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, layer_element_normal_node_handle, "ReferenceInformationType")?;
            let reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let normals_node_handle = find_node(nodes, layer_element_normal_node_handle, "Normals")?;
            let normals_array_node = find_and_borrow_node(nodes, normals_node_handle, "a")?;
            let count = normals_array_node.attrib_count() / 3;
            let mut normals = Vec::with_capacity(count);
            for i in 0..count {
                normals.push(normals_array_node.get_vec3_at(i * 3)?);
            }

            Ok(FbxContainer {
                elements: normals,
                mapping,
                reference,
                ..Default::default()
            })
        } else {
            Ok(Default::default())
        }
    }

    fn read_uvs(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<FbxContainer<Vec2>, String> {
        if let Ok(layer_element_uv_node_handle) = find_node(nodes, geom_node_handle, "LayerElementUV") {
            let map_type_node = find_and_borrow_node(nodes, layer_element_uv_node_handle, "MappingInformationType")?;
            let mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, layer_element_uv_node_handle, "ReferenceInformationType")?;
            let reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let uvs_node_handle = find_node(nodes, layer_element_uv_node_handle, "UV")?;
            let uvs_array_node = find_and_borrow_node(nodes, uvs_node_handle, "a")?;
            let count = uvs_array_node.attrib_count() / 2;
            let mut uvs = Vec::with_capacity(count);
            for i in 0..count {
                let uv = uvs_array_node.get_vec2_at(i * 2)?;
                uvs.push(Vec2 { x: uv.x, y: -uv.y }); // Hack FIXME
            }

            let mut index = Vec::new();
            if reference == FbxReference::IndexToDirect {
                let uv_index_node = find_node(nodes, layer_element_uv_node_handle, "UVIndex")?;
                let uv_index_array_node = find_and_borrow_node(nodes, uv_index_node, "a")?;
                for i in 0..uv_index_array_node.attrib_count() {
                    index.push(uv_index_array_node.get_attrib(i)?.as_i32()?);
                }
            }

            Ok(FbxContainer {
                elements: uvs,
                index,
                mapping,
                reference,
            })
        } else {
            Ok(Default::default())
        }
    }

    fn read_materials(geom_node_handle: Handle<FbxNode>, nodes: &Pool<FbxNode>) -> Result<FbxContainer<i32>, String> {
        if let Ok(layer_element_material_node_handle) = find_node(nodes, geom_node_handle, "LayerElementMaterial") {
            let map_type_node = find_and_borrow_node(nodes, layer_element_material_node_handle, "MappingInformationType")?;
            let mapping = string_to_mapping(&map_type_node.get_attrib(0)?.as_string());

            let ref_type_node = find_and_borrow_node(nodes, layer_element_material_node_handle, "ReferenceInformationType")?;
            let reference = string_to_reference(&ref_type_node.get_attrib(0)?.as_string());

            let materials_node_handle = find_node(nodes, layer_element_material_node_handle, "Materials")?;
            let materials_array_node = find_and_borrow_node(nodes, materials_node_handle, "a")?;
            let count = materials_array_node.attrib_count();
            let mut materials = Vec::with_capacity(count);
            for i in 0..count {
                materials.push(materials_array_node.get_attrib(i)?.as_i32()?);
            }

            Ok(FbxContainer {
                elements: materials,
                mapping,
                reference,
                ..Default::default()
            })
        } else {
            Ok(Default::default())
        }
    }

    pub(in crate::resource::fbx) fn get_skin_data(&self, components: &Pool<FbxComponent>) -> Result<Vec<VertexWeightSet>, FbxError> {
        let mut out = vec![VertexWeightSet::default(); self.vertices.len()];
        for deformer_handle in self.deformers.iter() {
            for sub_deformer_handle in components.borrow(*deformer_handle)
                .as_deformer()?.sub_deformers.iter() {
                let sub_deformer = components.borrow(*sub_deformer_handle)
                    .as_sub_deformer()?;
                for (index, weight) in sub_deformer.weights.iter() {
                    let bone_set = out.get_mut(*index as usize)
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