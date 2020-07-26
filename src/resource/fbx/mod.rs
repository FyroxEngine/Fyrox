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

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    animation::{Animation, AnimationContainer, KeyFrame, Track},
    core::{
        math::{
            mat4::Mat4,
            quat::{Quat, RotationOrder},
            triangulator::triangulate,
            vec2::Vec2,
            vec3::Vec3,
            vec4::Vec4,
        },
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    renderer::surface::{Surface, SurfaceSharedData, Vertex, VertexWeightSet},
    resource::{
        fbx::{
            document::FbxDocument,
            error::FbxError,
            scene::{
                animation::FbxAnimationCurveNodeType, geometry::FbxGeometry, model::FbxModel,
                FbxComponent, FbxMapping, FbxScene,
            },
        },
        texture::TextureKind,
    },
    scene::{base::Base, graph::Graph, mesh::Mesh, node::Node, Scene},
    utils::{log::Log, raw_mesh::RawMeshBuilder},
};
use std::cmp::Ordering;

/// Input angles in degrees
fn quat_from_euler(euler: Vec3) -> Quat {
    Quat::from_euler(
        Vec3::new(
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
    vertices: &[Vec3],
    indices: &[i32],
    temp_vertices: &mut Vec<Vec3>,
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
    position: Vec3,
    normal: Vec3,
    tangent: Vec3,
    uv: Vec2,
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
            tangent: Vec4::from_vec3(self.tangent, 1.0),
            // Correct values will be assigned in second pass of conversion
            // when all nodes will be converted.
            bone_weights: Default::default(),
            bone_indices: Default::default(),
        }
    }
}

fn convert_vertex(
    geom: &FbxGeometry,
    geometric_transform: &Mat4,
    material_index: usize,
    index: usize,
    index_in_polygon: usize,
    skin_data: &[VertexWeightSet],
) -> Result<UnpackedVertex, FbxError> {
    let position = *geom.vertices.get(index).ok_or(FbxError::IndexOutOfBounds)?;

    let normal = match geom.normals.as_ref() {
        Some(normals) => *normals.get(index, index_in_polygon)?,
        None => Vec3::UP,
    };

    let tangent = match geom.tangents.as_ref() {
        Some(tangents) => *tangents.get(index, index_in_polygon)?,
        None => Vec3::UP,
    };

    let uv = match geom.uvs.as_ref() {
        Some(uvs) => *uvs.get(index, index_in_polygon)?,
        None => Vec2::ZERO,
    };

    let material = match geom.materials.as_ref() {
        Some(materials) => *materials.get(material_index, index_in_polygon)?,
        None => 0,
    };

    Ok(UnpackedVertex {
        position: geometric_transform.transform_vector(position),
        normal: geometric_transform.transform_vector_normal(normal),
        tangent: geometric_transform.transform_vector_normal(tangent),
        uv: Vec2 { x: uv.x, y: -uv.y }, // Invert Y because OpenGL has origin at left *bottom* corner.
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
    mesh: &mut Mesh,
    resource_manager: &mut ResourceManager,
    model: &FbxModel,
) -> Result<(), FbxError> {
    // Create surfaces per material
    if model.materials.is_empty() {
        assert_eq!(data_set.len(), 1);
        let data = data_set.into_iter().next().unwrap();
        let mut surface = Surface::new(Arc::new(Mutex::new(SurfaceSharedData::from_raw_mesh(
            data.builder.build(),
            false,
        ))));
        surface.vertex_weights = data.skin_data;
        mesh.add_surface(surface);
    } else {
        assert_eq!(data_set.len(), model.materials.len());
        for (&material_handle, data) in model.materials.iter().zip(data_set.into_iter()) {
            let mut surface = Surface::new(Arc::new(Mutex::new(SurfaceSharedData::from_raw_mesh(
                data.builder.build(),
                false,
            ))));
            surface.vertex_weights = data.skin_data;
            let material = fbx_scene.get(material_handle).as_material()?;
            for (name, texture_handle) in material.textures.iter() {
                let texture = fbx_scene.get(*texture_handle).as_texture()?;
                let path = texture.get_file_path();
                if let Some(filename) = path.file_name() {
                    let diffuse_path = resource_manager.textures_path().join(&filename);
                    // Here we will load *every* texture as RGBA8, this probably is overkill,
                    // that will lead to higher memory consumption, but this will remove
                    // problems with transparent textures (like mesh texture, etc.)
                    let texture = resource_manager
                        .request_texture_async(diffuse_path.as_path(), TextureKind::RGBA8);
                    match name.as_str() {
                        "AmbientColor" => (), // TODO: Add ambient occlusion (AO) map support.
                        "DiffuseColor" => surface.set_diffuse_texture(texture),
                        // No idea why it can be different for normal maps.
                        "Bump" | "NormalMap" => surface.set_normal_texture(texture),
                        _ => (),
                    }
                }
            }
            mesh.add_surface(surface);
        }
    }

    Ok(())
}

fn convert_mesh(
    fbx_scene: &FbxScene,
    resource_manager: &mut ResourceManager,
    model: &FbxModel,
) -> Result<Mesh, FbxError> {
    let mut mesh = Mesh::default();

    let geometric_transform = Mat4::translate(model.geometric_translation)
        * Mat4::from_quat(quat_from_euler(model.geometric_rotation))
        * Mat4::scale(model.geometric_scale);

    let mut temp_vertices = Vec::new();
    let mut triangles = Vec::new();

    // Array for triangulation needs, it will contain triangle definitions for
    // triangulated polygon.
    let mut face_triangles = Vec::new();

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

        create_surfaces(fbx_scene, data_set, &mut mesh, resource_manager, model)?;

        if geom.tangents.is_none() {
            for surface in mesh.surfaces_mut() {
                surface.data().lock().unwrap().calculate_tangents();
            }
        }
    }

    Ok(mesh)
}

fn convert_model(
    fbx_scene: &FbxScene,
    model: &FbxModel,
    resource_manager: &mut ResourceManager,
    graph: &mut Graph,
    animations: &mut AnimationContainer,
    animation_handle: Handle<Animation>,
) -> Result<Handle<Node>, FbxError> {
    // Create node with correct kind.
    let mut node = if !model.geoms.is_empty() {
        Node::Mesh(convert_mesh(fbx_scene, resource_manager, model)?)
    } else if model.light.is_some() {
        let fbx_light_component = fbx_scene.get(model.light);
        Node::Light(fbx_light_component.as_light()?.convert())
    } else {
        Node::Base(Base::default())
    };

    let node_local_rotation = quat_from_euler(model.rotation);
    node.set_name(model.name.as_str())
        .local_transform_mut()
        .set_rotation(node_local_rotation)
        .set_scale(model.scale)
        .set_position(model.translation)
        .set_post_rotation(quat_from_euler(model.post_rotation))
        .set_pre_rotation(quat_from_euler(model.pre_rotation))
        .set_rotation_offset(model.rotation_offset)
        .set_rotation_pivot(model.rotation_pivot)
        .set_scaling_offset(model.scaling_offset)
        .set_scaling_pivot(model.scaling_pivot);
    node.inv_bind_pose_transform = model.inv_bind_transform;

    let node_handle = graph.add_node(node);

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
    resource_manager: &mut ResourceManager,
    scene: &mut Scene,
) -> Result<Handle<Node>, FbxError> {
    let root = scene.graph.add_node(Node::Base(Base::default()));
    let animation_handle = scene.animations.add(Animation::default());
    let mut fbx_model_to_node_map = HashMap::new();
    for (component_handle, component) in fbx_scene.pair_iter() {
        if let FbxComponent::Model(model) = component {
            let node = convert_model(
                fbx_scene,
                model,
                resource_manager,
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
    scene.graph.update_hierachical_data();

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
                let mut data = data_rc.lock().unwrap();
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

    Ok(root)
}

/// Tries to load and convert FBX from given path.
///
/// Normally you should never use this method, use resource manager to load models.
pub fn load_to_scene<P: AsRef<Path>>(
    scene: &mut Scene,
    resource_manager: &mut ResourceManager,
    path: P,
) -> Result<Handle<Node>, FbxError> {
    let start_time = Instant::now();

    Log::writeln(format!("Trying to load {:?}", path.as_ref()));

    let now = Instant::now();
    let fbx = FbxDocument::new(path.as_ref())?;
    let parsing_time = now.elapsed().as_millis();

    let now = Instant::now();
    let fbx_scene = FbxScene::new(&fbx)?;
    let dom_prepare_time = now.elapsed().as_millis();

    let now = Instant::now();
    let result = convert(&fbx_scene, resource_manager, scene);
    let conversion_time = now.elapsed().as_millis();

    Log::writeln(format!("FBX {:?} loaded in {} ms\n\t- Parsing - {} ms\n\t- DOM Prepare - {} ms\n\t- Conversion - {} ms",
                         path.as_ref(), start_time.elapsed().as_millis(), parsing_time, dom_prepare_time, conversion_time));

    result
}
