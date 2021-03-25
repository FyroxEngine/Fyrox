//! Contains all methods to load and convert FBX model format.
//!
//! FBX is most flexible format to store and distribute 3D models, it has lots of useful features
//! such as skeletal animation, keyframe animation, support tangents, binormals, materials, etc.
//!
//! Normally you should never use methods from this module directly, use resource manager to load
//! models and create their instances.

mod document;
pub mod error;
mod scene;

use crate::{
    animation::{Animation, AnimationContainer, KeyFrame, Track},
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3, Vector4},
        math::{self, triangulator::triangulate, RotationOrder},
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    renderer::surface::{Surface, SurfaceSharedData, Vertex, VertexWeightSet},
    resource::fbx::{
        document::FbxDocument,
        error::FbxError,
        scene::{
            animation::FbxAnimationCurveNodeType, geometry::FbxGeometry, model::FbxModel,
            FbxComponent, FbxMapping, FbxScene,
        },
    },
    scene::{
        base::BaseBuilder, graph::Graph, mesh::MeshBuilder, node::Node,
        transform::TransformBuilder, Scene,
    },
    utils::{
        log::{Log, MessageKind},
        raw_mesh::RawMeshBuilder,
    },
};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, RwLock},
    time::Instant,
};

/// Input angles in degrees
fn quat_from_euler(euler: Vector3<f32>) -> UnitQuaternion<f32> {
    math::quat_from_euler(
        Vector3::new(
            euler.x.to_radians(),
            euler.y.to_radians(),
            euler.z.to_radians(),
        ),
        RotationOrder::XYZ,
    )
}

/// Fixes index that is used as indicator of end of a polygon
/// FBX stores array of indices like so 0,1,-3,... where -3
/// is actually index 2 but it xor'ed using -1.
fn fix_index(index: i32) -> usize {
    if index < 0 {
        (index ^ -1) as usize
    } else {
        index as usize
    }
}

/// Triangulates polygon face if needed.
/// Returns number of processed indices.
fn prepare_next_face(
    vertices: &[Vector3<f32>],
    indices: &[i32],
    temp_vertices: &mut Vec<Vector3<f32>>,
    out_triangles: &mut Vec<[usize; 3]>,
    out_face_triangles: &mut Vec<[usize; 3]>,
) -> usize {
    out_triangles.clear();
    out_face_triangles.clear();

    // Find out how much vertices do we have per face.
    let mut vertex_per_face = 0;
    for &index in indices {
        vertex_per_face += 1;
        if index < 0 {
            break;
        }
    }

    match vertex_per_face.cmp(&3) {
        Ordering::Less => {
            // Silently ignore invalid faces.
        }
        Ordering::Equal => {
            let a = fix_index(indices[0]);
            let b = fix_index(indices[1]);
            let c = fix_index(indices[2]);

            // Ensure that we have valid indices here. Some exporters may fuck up indices
            // and they'll blow up loader.
            if a < vertices.len() && b < vertices.len() && c < vertices.len() {
                // We have a triangle
                out_triangles.push([a, b, c]);
                out_face_triangles.push([0, 1, 2]);
            }
        }
        Ordering::Greater => {
            // Found arbitrary polygon, triangulate it.
            temp_vertices.clear();
            for i in 0..vertex_per_face {
                temp_vertices.push(vertices[fix_index(indices[i])]);
            }
            triangulate(&temp_vertices, out_face_triangles);
            for triangle in out_face_triangles.iter() {
                out_triangles.push([
                    fix_index(indices[triangle[0]]),
                    fix_index(indices[triangle[1]]),
                    fix_index(indices[triangle[2]]),
                ]);
            }
        }
    }

    vertex_per_face
}

struct UnpackedVertex {
    // Index of surface this vertex belongs to.
    surface: usize,
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tangent: Vector3<f32>,
    uv: Vector2<f32>,
    // Set of weights for skinning.
    weights: Option<VertexWeightSet>,
}

impl Into<Vertex> for UnpackedVertex {
    fn into(self) -> Vertex {
        Vertex {
            position: self.position,
            tex_coord: self.uv,
            // TODO: FBX can contain second texture coordinates so they should be
            //  extracted when available
            second_tex_coord: Default::default(),
            normal: self.normal,
            tangent: Vector4::new(self.tangent.x, self.tangent.y, self.tangent.z, 1.0),
            // Correct values will be assigned in second pass of conversion
            // when all nodes will be converted.
            bone_weights: Default::default(),
            bone_indices: Default::default(),
        }
    }
}

fn convert_vertex(
    geom: &FbxGeometry,
    geometric_transform: &Matrix4<f32>,
    material_index: usize,
    index: usize,
    index_in_polygon: usize,
    skin_data: &[VertexWeightSet],
) -> Result<UnpackedVertex, FbxError> {
    let position = *geom.vertices.get(index).ok_or(FbxError::IndexOutOfBounds)?;

    let normal = match geom.normals.as_ref() {
        Some(normals) => *normals.get(index, index_in_polygon)?,
        None => Vector3::y(),
    };

    let tangent = match geom.tangents.as_ref() {
        Some(tangents) => *tangents.get(index, index_in_polygon)?,
        None => Vector3::y(),
    };

    let uv = match geom.uvs.as_ref() {
        Some(uvs) => *uvs.get(index, index_in_polygon)?,
        None => Vector2::default(),
    };

    let material = match geom.materials.as_ref() {
        Some(materials) => *materials.get(material_index, index_in_polygon)?,
        None => 0,
    };

    Ok(UnpackedVertex {
        position: geometric_transform
            .transform_point(&Point3::from(position))
            .coords,
        normal: geometric_transform.transform_vector(&normal),
        tangent: geometric_transform.transform_vector(&tangent),
        uv: Vector2::new(uv.x, -uv.y), // Invert Y because OpenGL has origin at left *bottom* corner.
        surface: material as usize,
        weights: if skin_data.is_empty() {
            None
        } else {
            Some(*skin_data.get(index).ok_or(FbxError::IndexOutOfBounds)?)
        },
    })
}

#[derive(Default, Clone)]
struct SurfaceData {
    builder: RawMeshBuilder<Vertex>,
    skin_data: Vec<VertexWeightSet>,
}

fn create_surfaces(
    fbx_scene: &FbxScene,
    data_set: Vec<SurfaceData>,
    resource_manager: ResourceManager,
    model: &FbxModel,
) -> Result<Vec<Surface>, FbxError> {
    let mut surfaces = Vec::new();

    // Create surfaces per material
    if model.materials.is_empty() {
        assert_eq!(data_set.len(), 1);
        let data = data_set.into_iter().next().unwrap();
        let mut surface = Surface::new(Arc::new(RwLock::new(SurfaceSharedData::from_raw_mesh(
            data.builder.build(),
            false,
        ))));
        surface.vertex_weights = data.skin_data;
        surfaces.push(surface);
    } else {
        assert_eq!(data_set.len(), model.materials.len());
        for (&material_handle, data) in model.materials.iter().zip(data_set.into_iter()) {
            let mut surface = Surface::new(Arc::new(RwLock::new(
                SurfaceSharedData::from_raw_mesh(data.builder.build(), false),
            )));
            surface.vertex_weights = data.skin_data;
            let material = fbx_scene.get(material_handle).as_material()?;
            for (name, texture_handle) in material.textures.iter() {
                let texture = fbx_scene.get(*texture_handle).as_texture()?;
                let path = texture.get_file_path();
                if let Some(filename) = path.file_name() {
                    let texture_path = resource_manager.state().textures_path().join(&filename);
                    let texture = resource_manager.request_texture(texture_path.as_path());
                    match name.as_str() {
                        "AmbientColor" => (), // TODO: Add ambient occlusion (AO) map support.
                        "DiffuseColor" => surface.set_diffuse_texture(Some(texture)),
                        "SpecularFactor" => surface.set_specular_texture(Some(texture)),
                        "ShininessExponent" => surface.set_roughness_texture(Some(texture)),
                        // No idea why it can be different for normal maps.
                        "Bump" | "NormalMap" => surface.set_normal_texture(Some(texture)),
                        _ => (),
                    }
                }
            }
            surfaces.push(surface);
        }
    }

    Ok(surfaces)
}

fn convert_mesh(
    base: BaseBuilder,
    fbx_scene: &FbxScene,
    resource_manager: ResourceManager,
    model: &FbxModel,
    graph: &mut Graph,
) -> Result<Handle<Node>, FbxError> {
    let geometric_transform = Matrix4::new_translation(&model.geometric_translation)
        * quat_from_euler(model.geometric_rotation).to_homogeneous()
        * Matrix4::new_nonuniform_scaling(&model.geometric_scale);

    let mut temp_vertices = Vec::new();
    let mut triangles = Vec::new();

    // Array for triangulation needs, it will contain triangle definitions for
    // triangulated polygon.
    let mut face_triangles = Vec::new();

    let mut mesh_surfaces = Vec::new();
    for &geom_handle in &model.geoms {
        let geom = fbx_scene.get(geom_handle).as_geometry()?;
        let skin_data = geom.get_skin_data(fbx_scene)?;

        let mut data_set = vec![
            SurfaceData {
                builder: RawMeshBuilder::new(1024, 1024),
                skin_data: Default::default(),
            };
            model.materials.len().max(1)
        ];

        let mut material_index = 0;
        let mut n = 0;
        while n < geom.indices.len() {
            let origin = n;
            n += prepare_next_face(
                &geom.vertices,
                &geom.indices[origin..],
                &mut temp_vertices,
                &mut triangles,
                &mut face_triangles,
            );
            for (triangle, face_triangle) in triangles.iter().zip(face_triangles.iter()) {
                for (&index, &face_vertex_index) in triangle.iter().zip(face_triangle.iter()) {
                    let polygon_vertex_index = origin + face_vertex_index;
                    let vertex = convert_vertex(
                        geom,
                        &geometric_transform,
                        material_index,
                        index,
                        polygon_vertex_index,
                        &skin_data,
                    )?;
                    let data = data_set.get_mut(vertex.surface).unwrap();
                    let weights = vertex.weights;
                    let is_unique_vertex = data.builder.insert(vertex.into());
                    if is_unique_vertex {
                        if let Some(skin_data) = weights {
                            data.skin_data.push(skin_data);
                        }
                    }
                }
            }
            if let Some(materials) = geom.materials.as_ref() {
                if materials.mapping == FbxMapping::ByPolygon {
                    material_index += 1;
                }
            }
        }

        let mut surfaces = create_surfaces(fbx_scene, data_set, resource_manager.clone(), model)?;

        if geom.tangents.is_none() {
            for surface in surfaces.iter_mut() {
                surface.data().write().unwrap().calculate_tangents();
            }
        }

        for surface in surfaces {
            mesh_surfaces.push(surface);
        }
    }

    Ok(MeshBuilder::new(base)
        .with_surfaces(mesh_surfaces)
        .build(graph))
}

fn convert_model_to_base(model: &FbxModel) -> BaseBuilder {
    BaseBuilder::new()
        .with_inv_bind_pose_transform(model.inv_bind_transform)
        .with_name(model.name.as_str())
        .with_local_transform(
            TransformBuilder::new()
                .with_local_rotation(quat_from_euler(model.rotation))
                .with_local_scale(model.scale)
                .with_local_position(model.translation)
                .with_post_rotation(quat_from_euler(model.post_rotation))
                .with_pre_rotation(quat_from_euler(model.pre_rotation))
                .with_rotation_offset(model.rotation_offset)
                .with_rotation_pivot(model.rotation_pivot)
                .with_scaling_offset(model.scaling_offset)
                .with_scaling_pivot(model.scaling_pivot)
                .build(),
        )
}

fn convert_model(
    fbx_scene: &FbxScene,
    model: &FbxModel,
    resource_manager: ResourceManager,
    graph: &mut Graph,
    animations: &mut AnimationContainer,
    animation_handle: Handle<Animation>,
) -> Result<Handle<Node>, FbxError> {
    let base = convert_model_to_base(model);

    // Create node with correct kind.
    let node_handle = if !model.geoms.is_empty() {
        convert_mesh(base, fbx_scene, resource_manager, model, graph)?
    } else if model.light.is_some() {
        fbx_scene.get(model.light).as_light()?.convert(base, graph)
    } else {
        base.build(graph)
    };

    // Convert animations
    if !model.animation_curve_nodes.is_empty() {
        // Find supported curve nodes (translation, rotation, scale)
        let mut lcl_translation = None;
        let mut lcl_rotation = None;
        let mut lcl_scale = None;
        for &anim_curve_node_handle in model.animation_curve_nodes.iter() {
            let component = fbx_scene.get(anim_curve_node_handle);
            if let FbxComponent::AnimationCurveNode(curve_node) = component {
                if curve_node.actual_type == FbxAnimationCurveNodeType::Rotation {
                    lcl_rotation = Some(curve_node);
                } else if curve_node.actual_type == FbxAnimationCurveNodeType::Translation {
                    lcl_translation = Some(curve_node);
                } else if curve_node.actual_type == FbxAnimationCurveNodeType::Scale {
                    lcl_scale = Some(curve_node);
                }
            }
        }

        // Convert to engine format
        let mut track = Track::new();
        track.set_node(node_handle);

        let node_local_rotation = quat_from_euler(model.rotation);

        let mut time = 0.0;
        loop {
            let translation = lcl_translation
                .map(|curve| curve.eval_vec3(fbx_scene, time))
                .unwrap_or(model.translation);

            let rotation = lcl_rotation
                .map(|curve| curve.eval_quat(fbx_scene, time))
                .unwrap_or(node_local_rotation);

            let scale = lcl_scale
                .map(|curve| curve.eval_vec3(fbx_scene, time))
                .unwrap_or(model.scale);

            track.add_key_frame(KeyFrame::new(time, translation, scale, rotation));

            let mut next_time = std::f32::MAX;
            for node in &[lcl_translation, lcl_rotation, lcl_scale] {
                if let Some(node) = node {
                    for &curve_handle in node.curves.iter() {
                        let curve_component = fbx_scene.get(curve_handle);
                        if let FbxComponent::AnimationCurve(curve) = curve_component {
                            for key in curve.keys.iter() {
                                if key.time > time {
                                    let distance = key.time - time;
                                    if distance < next_time - key.time {
                                        next_time = key.time;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if next_time >= std::f32::MAX {
                break;
            }

            time = next_time;
        }

        animations.get_mut(animation_handle).add_track(track);
    }

    Ok(node_handle)
}

///
/// Converts FBX DOM to native engine representation.
///
fn convert(
    fbx_scene: &FbxScene,
    resource_manager: ResourceManager,
    scene: &mut Scene,
) -> Result<(), FbxError> {
    let root = scene.graph.get_root();
    let animation_handle = scene.animations.add(Animation::default());
    let mut fbx_model_to_node_map = HashMap::new();
    for (component_handle, component) in fbx_scene.pair_iter() {
        if let FbxComponent::Model(model) = component {
            let node = convert_model(
                fbx_scene,
                model,
                resource_manager.clone(),
                &mut scene.graph,
                &mut scene.animations,
                animation_handle,
            )?;
            scene.graph.link_nodes(node, root);
            fbx_model_to_node_map.insert(component_handle, node);
        }
    }
    // Link according to hierarchy
    for (&fbx_model_handle, node_handle) in fbx_model_to_node_map.iter() {
        if let FbxComponent::Model(fbx_model) = fbx_scene.get(fbx_model_handle) {
            for fbx_child_handle in fbx_model.children.iter() {
                if let Some(child_handle) = fbx_model_to_node_map.get(fbx_child_handle) {
                    scene.graph.link_nodes(*child_handle, *node_handle);
                }
            }
        }
    }
    scene.graph.update_hierarchical_data();

    // Remap handles from fbx model to handles of instantiated nodes
    // on each surface of each mesh.
    for &handle in fbx_model_to_node_map.values() {
        if let Node::Mesh(mesh) = &mut scene.graph[handle] {
            let mut surface_bones = HashSet::new();
            for surface in mesh.surfaces_mut() {
                for weight_set in surface.vertex_weights.iter_mut() {
                    for weight in weight_set.iter_mut() {
                        let fbx_model: Handle<FbxComponent> = weight.effector.into();
                        let bone_handle = fbx_model_to_node_map
                            .get(&fbx_model)
                            .ok_or(FbxError::UnableToRemapModelToNode)?;
                        surface_bones.insert(*bone_handle);
                        weight.effector = (*bone_handle).into();
                    }
                }
                surface.bones = surface_bones.iter().copied().collect();

                let data_rc = surface.data();
                let mut data = data_rc.write().unwrap();
                if data.get_vertices().len() == surface.vertex_weights.len() {
                    for (vertex, weight_set) in data
                        .get_vertices_mut()
                        .iter_mut()
                        .zip(surface.vertex_weights.iter())
                    {
                        for (k, weight) in weight_set.iter().enumerate() {
                            vertex.bone_indices[k] = surface
                                .bones
                                .iter()
                                .position(|bone_handle| *bone_handle == weight.effector.into())
                                .ok_or(FbxError::UnableToFindBone)?
                                as u8;
                            vertex.bone_weights[k] = weight.value;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Tries to load and convert FBX from given path.
///
/// Normally you should never use this method, use resource manager to load models.
pub fn load_to_scene<P: AsRef<Path>>(
    scene: &mut Scene,
    resource_manager: ResourceManager,
    path: P,
) -> Result<(), FbxError> {
    let start_time = Instant::now();

    Log::writeln(
        MessageKind::Information,
        format!("Trying to load {:?}", path.as_ref()),
    );

    let now = Instant::now();
    let fbx = FbxDocument::new(path.as_ref())?;
    let parsing_time = now.elapsed().as_millis();

    let now = Instant::now();
    let fbx_scene = FbxScene::new(&fbx)?;
    let dom_prepare_time = now.elapsed().as_millis();

    let now = Instant::now();
    convert(&fbx_scene, resource_manager, scene)?;
    let conversion_time = now.elapsed().as_millis();

    Log::writeln(MessageKind::Information,
                 format!("FBX {:?} loaded in {} ms\n\t- Parsing - {} ms\n\t- DOM Prepare - {} ms\n\t- Conversion - {} ms",
                         path.as_ref(), start_time.elapsed().as_millis(), parsing_time, dom_prepare_time, conversion_time));

    // Check for multiple nodes with same name and throw a warning if any.
    // It seems that FBX was designed using ass, not brains. It has no unique **persistent**
    // IDs for entities, so the only way to find an entity is to use its name, but FBX also
    // allows to have multiple entities with the same name. facepalm.jpg
    let mut hash_set = HashSet::<String>::new();
    for node in scene.graph.linear_iter() {
        if hash_set.contains(node.name()) {
            Log::writeln(
                MessageKind::Error,
                format!(
                    "A node with existing name {} was found during the load of {} resource! \
                    Do **NOT IGNORE** this message, please fix names in your model, otherwise \
                    engine won't be able to correctly restore data from your resource!",
                    node.name(),
                    path.as_ref().display()
                ),
            );
        } else {
            hash_set.insert(node.name_owned());
        }
    }

    Ok(())
}
