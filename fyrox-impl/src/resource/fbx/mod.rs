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

use crate::resource::texture::{TextureImportOptions, TextureResource, TextureResourceExtension};
use crate::{
    asset::manager::ResourceManager,
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3, Vector4},
        instant::Instant,
        log::{Log, MessageKind},
        math::curve::{CurveKey, CurveKeyKind},
        math::{self, triangulator::triangulate, RotationOrder},
        pool::Handle,
        sstorage::ImmutableString,
    },
    graph::BaseSceneGraph,
    material::{shader::SamplerFallback, PropertyValue},
    resource::{
        fbx::{
            document::FbxDocument,
            error::FbxError,
            scene::{
                animation::{FbxAnimationCurveNode, FbxAnimationCurveNodeType},
                geometry::FbxMeshGeometry,
                model::FbxModel,
                FbxComponent, FbxMapping, FbxScene,
            },
        },
        model::{MaterialSearchOptions, ModelImportOptions},
        texture::Texture,
    },
    scene::{
        animation::{Animation, AnimationContainer, AnimationPlayerBuilder, Track},
        base::BaseBuilder,
        graph::Graph,
        mesh::{
            buffer::{VertexAttributeUsage, VertexBuffer, VertexWriteTrait},
            surface::{
                BlendShape, BlendShapesContainer, InputBlendShapeData, Surface, SurfaceData,
                SurfaceResource, VertexWeightSet,
            },
            vertex::{AnimatedVertex, StaticVertex},
            Mesh, MeshBuilder,
        },
        node::Node,
        pivot::PivotBuilder,
        transform::TransformBuilder,
        Scene,
    },
    utils::{self, raw_mesh::RawMeshBuilder},
};
use fxhash::{FxHashMap, FxHashSet};
use fyrox_resource::io::ResourceIo;
use fyrox_resource::untyped::ResourceKind;
use std::{cmp::Ordering, path::Path};

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
            triangulate(temp_vertices, out_face_triangles);
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

#[derive(Clone)]
struct UnpackedVertex {
    // Index of surface this vertex belongs to.
    surface_index: usize,
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tangent: Vector3<f32>,
    uv: Vector2<f32>,
    // Set of weights for skinning.
    weights: Option<VertexWeightSet>,
}

impl Into<AnimatedVertex> for UnpackedVertex {
    fn into(self) -> AnimatedVertex {
        AnimatedVertex {
            position: self.position,
            tex_coord: self.uv,
            normal: self.normal,
            tangent: Vector4::new(self.tangent.x, self.tangent.y, self.tangent.z, 1.0),
            // Correct values will be assigned in second pass of conversion
            // when all nodes will be converted.
            bone_weights: Default::default(),
            bone_indices: Default::default(),
        }
    }
}

impl Into<StaticVertex> for UnpackedVertex {
    fn into(self) -> StaticVertex {
        StaticVertex {
            position: self.position,
            tex_coord: self.uv,
            normal: self.normal,
            tangent: Vector4::new(self.tangent.x, self.tangent.y, self.tangent.z, 1.0),
        }
    }
}

fn convert_vertex(
    geom: &FbxMeshGeometry,
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
        uv: Vector2::new(uv.x, 1.0 - uv.y), // Invert Y because OpenGL has origin at left *bottom* corner.
        surface_index: material as usize,
        weights: if geom.deformers.is_empty() {
            None
        } else {
            Some(*skin_data.get(index).ok_or(FbxError::IndexOutOfBounds)?)
        },
    })
}

#[derive(Clone)]
enum FbxMeshBuilder {
    Static(RawMeshBuilder<StaticVertex>),
    Animated(RawMeshBuilder<AnimatedVertex>),
}

impl FbxMeshBuilder {
    fn build(self) -> SurfaceData {
        match self {
            FbxMeshBuilder::Static(builder) => SurfaceData::from_raw_mesh(builder.build()),
            FbxMeshBuilder::Animated(builder) => SurfaceData::from_raw_mesh(builder.build()),
        }
    }
}

#[derive(Clone)]
struct FbxSurfaceData {
    base_mesh_builder: FbxMeshBuilder,
    blend_shapes: Vec<InputBlendShapeData>,
    skin_data: Vec<VertexWeightSet>,
}

fn make_blend_shapes_container(
    base_shape: &VertexBuffer,
    blend_shapes: Vec<InputBlendShapeData>,
) -> Option<BlendShapesContainer> {
    if blend_shapes.is_empty() {
        None
    } else {
        Some(BlendShapesContainer::from_lists(base_shape, &blend_shapes))
    }
}

async fn create_surfaces(
    fbx_scene: &FbxScene,
    data_set: Vec<FbxSurfaceData>,
    resource_manager: ResourceManager,
    model: &FbxModel,
    model_path: &Path,
    model_import_options: &ModelImportOptions,
) -> Result<Vec<Surface>, FbxError> {
    let mut surfaces = Vec::new();

    // Create surfaces per material
    if model.materials.is_empty() {
        assert_eq!(data_set.len(), 1);
        let data = data_set.into_iter().next().unwrap();
        let mut surface_data = data.base_mesh_builder.build();
        surface_data.blend_shapes_container =
            make_blend_shapes_container(&surface_data.vertex_buffer, data.blend_shapes);
        let mut surface = Surface::new(SurfaceResource::new_ok(
            ResourceKind::External(model_path.to_path_buf()),
            surface_data,
        ));
        surface.vertex_weights = data.skin_data;
        surfaces.push(surface);
    } else {
        assert_eq!(data_set.len(), model.materials.len());
        for (&material_handle, data) in model.materials.iter().zip(data_set.into_iter()) {
            let mut surface_data = data.base_mesh_builder.build();
            surface_data.blend_shapes_container =
                make_blend_shapes_container(&surface_data.vertex_buffer, data.blend_shapes);
            let mut surface = Surface::new(SurfaceResource::new_ok(
                ResourceKind::External(model_path.to_path_buf()),
                surface_data,
            ));
            surface.vertex_weights = data.skin_data;
            let material = fbx_scene.get(material_handle).as_material()?;
            if let Err(e) = surface.material().data_ref().set_property(
                &ImmutableString::new("diffuseColor"),
                PropertyValue::Color(material.diffuse_color),
            ) {
                Log::writeln(
                    MessageKind::Error,
                    format!(
                        "Failed to set diffuseColor property for material. Reason: {:?}",
                        e,
                    ),
                )
            }

            let io = resource_manager.resource_io();

            for (name, texture_handle) in material.textures.iter() {
                let texture = fbx_scene.get(*texture_handle).as_texture()?;
                let path = texture.get_file_path();

                if let Some(filename) = path.file_name() {
                    let texture_path = if texture.content.is_empty() {
                        match model_import_options.material_search_options {
                            MaterialSearchOptions::MaterialsDirectory(ref directory) => {
                                Some(directory.join(filename))
                            }
                            MaterialSearchOptions::RecursiveUp => {
                                let mut texture_path = None;
                                let mut path = model_path.to_owned();
                                while let Some(parent) = path.parent() {
                                    let candidate = parent.join(filename);
                                    if io.exists(&candidate).await {
                                        texture_path = Some(candidate);
                                        break;
                                    }
                                    path.pop();
                                }
                                texture_path
                            }
                            MaterialSearchOptions::WorkingDirectory => {
                                let mut texture_path = None;

                                let path = Path::new(".");

                                if let Ok(iter) = io.walk_directory(path).await {
                                    for dir in iter {
                                        if io.is_dir(&dir).await {
                                            let candidate = dir.join(filename);
                                            if candidate.exists() {
                                                texture_path = Some(candidate);
                                                break;
                                            }
                                        }
                                    }
                                }

                                texture_path
                            }
                            MaterialSearchOptions::UsePathDirectly => Some(path.clone()),
                        }
                    } else {
                        Some(path.clone())
                    };

                    if let Some(texture_path) = texture_path {
                        let texture = if texture.content.is_empty() {
                            resource_manager.request::<Texture>(texture_path.as_path())
                        } else {
                            TextureResource::load_from_memory(
                                ResourceKind::External(texture_path.clone()),
                                &texture.content,
                                TextureImportOptions::default(),
                            )
                            .unwrap()
                        };

                        // Make up your mind, Autodesk and Blender.
                        // Handle all possible combinations of links to auto-import materials.
                        let name_usage = if name.contains("AmbientColor")
                            || name.contains("ambient_color")
                        {
                            Some(("aoTexture", SamplerFallback::White))
                        } else if name.contains("DiffuseColor") || name.contains("diffuse_color") {
                            Some(("diffuseTexture", SamplerFallback::White))
                        } else if name.contains("MetalnessMap")
                            || name.contains("metalness_map")
                            || name.contains("ReflectionFactor")
                        {
                            Some(("metallicTexture", SamplerFallback::Black))
                        } else if name.contains("RoughnessMap")
                            || name.contains("roughness_map")
                            || name.contains("Shininess")
                            || name.contains("ShininessExponent")
                        {
                            Some(("roughnessTexture", SamplerFallback::White))
                        } else if name.contains("Bump")
                            || name.contains("bump_map")
                            || name.contains("NormalMap")
                            || name.contains("normal_map")
                        {
                            Some(("normalTexture", SamplerFallback::Normal))
                        } else if name.contains("DisplacementColor")
                            || name.contains("displacement_map")
                        {
                            Some(("heightTexture", SamplerFallback::Black))
                        } else if name.contains("EmissiveColor") || name.contains("emit_color_map")
                        {
                            Some(("emissionTexture", SamplerFallback::Black))
                        } else {
                            None
                        };

                        if let Some((property_name, usage)) = name_usage {
                            if let Err(e) = surface.material().data_ref().set_property(
                                &ImmutableString::new(property_name),
                                PropertyValue::Sampler {
                                    value: Some(texture),
                                    fallback: usage,
                                },
                            ) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!(
                                        "Unable to set material property {}\
                                 for FBX material! Reason: {:?}",
                                        property_name, e
                                    ),
                                );
                            }
                        }
                    } else {
                        Log::writeln(
                            MessageKind::Warning,
                            format!(
                                "Unable to find a texture {:?} for 3D model {:?} using {:?} option!",
                                filename, model_path, model_import_options
                            ),
                        );
                    }
                }
            }
            surfaces.push(surface);
        }
    }

    Ok(surfaces)
}

async fn convert_mesh(
    base: BaseBuilder,
    fbx_scene: &FbxScene,
    resource_manager: ResourceManager,
    model: &FbxModel,
    graph: &mut Graph,
    model_path: &Path,
    model_import_options: &ModelImportOptions,
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
    let mut mesh_blend_shapes = Vec::new();

    for &geom_handle in &model.geoms {
        let geom = fbx_scene.get(geom_handle).as_mesh_geometry()?;
        let skin_data = geom.get_skin_data(fbx_scene)?;
        let blend_shapes = geom.collect_blend_shapes_refs(fbx_scene)?;

        if !mesh_blend_shapes.is_empty() {
            Log::warn("More than two geoms with blend shapes?");
        }
        mesh_blend_shapes = blend_shapes
            .iter()
            .map(|bs| BlendShape {
                weight: bs.deform_percent,
                name: bs.name.clone(),
            })
            .collect();

        let mut data_set = vec![
            FbxSurfaceData {
                base_mesh_builder: if geom.deformers.is_empty() {
                    FbxMeshBuilder::Static(RawMeshBuilder::new(1024, 1024))
                } else {
                    FbxMeshBuilder::Animated(RawMeshBuilder::new(1024, 1024))
                },
                blend_shapes: blend_shapes
                    .iter()
                    .map(|bs_channel| {
                        InputBlendShapeData {
                            name: bs_channel.name.clone(),
                            default_weight: bs_channel.deform_percent,
                            positions: Default::default(),
                            normals: Default::default(),
                            tangents: Default::default(),
                        }
                    })
                    .collect(),
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
                    let data = data_set.get_mut(vertex.surface_index).unwrap();
                    let weights = vertex.weights;
                    let final_index;
                    let is_unique_vertex = match data.base_mesh_builder {
                        FbxMeshBuilder::Static(ref mut builder) => {
                            final_index = builder.vertex_count();
                            builder.insert(vertex.clone().into())
                        }
                        FbxMeshBuilder::Animated(ref mut builder) => {
                            final_index = builder.vertex_count();
                            builder.insert(vertex.clone().into())
                        }
                    };
                    if is_unique_vertex {
                        if let Some(skin_data) = weights {
                            data.skin_data.push(skin_data);
                        }
                    }

                    // Fill each blend shape, but modify the vertex first using the "offsets" from blend shapes.
                    assert_eq!(blend_shapes.len(), data.blend_shapes.len());
                    for (fbx_blend_shape, blend_shape) in
                        blend_shapes.iter().zip(data.blend_shapes.iter_mut())
                    {
                        let blend_shape_geometry = fbx_scene
                            .get(fbx_blend_shape.geometry)
                            .as_shape_geometry()?;

                        // Only certain vertices are affected by a blend shape, because FBX stores only changed
                        // parts ("diff").
                        if let Some(relative_index) =
                            blend_shape_geometry.indices.get(&(index as i32))
                        {
                            blend_shape.positions.insert(
                                final_index as u32,
                                utils::vec3_f16_from_f32(
                                    blend_shape_geometry.vertices[*relative_index as usize],
                                ),
                            );
                            if let Some(normals) = blend_shape_geometry.normals.as_ref() {
                                blend_shape.normals.insert(
                                    final_index as u32,
                                    utils::vec3_f16_from_f32(normals[*relative_index as usize]),
                                );
                            }
                            if let Some(tangents) = blend_shape_geometry.tangents.as_ref() {
                                blend_shape.normals.insert(
                                    final_index as u32,
                                    utils::vec3_f16_from_f32(tangents[*relative_index as usize]),
                                );
                            }
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

        let mut surfaces = create_surfaces(
            fbx_scene,
            data_set,
            resource_manager.clone(),
            model,
            model_path,
            model_import_options,
        )
        .await?;

        if geom.tangents.is_none() {
            for surface in surfaces.iter_mut() {
                surface.data().data_ref().calculate_tangents().unwrap();
            }
        }

        for surface in surfaces {
            mesh_surfaces.push(surface);
        }
    }

    Ok(MeshBuilder::new(base)
        .with_blend_shapes(mesh_blend_shapes)
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

async fn convert_model(
    fbx_scene: &FbxScene,
    model: &FbxModel,
    resource_manager: ResourceManager,
    graph: &mut Graph,
    animation: &mut Animation,
    model_path: &Path,
    model_import_options: &ModelImportOptions,
) -> Result<Handle<Node>, FbxError> {
    let base = convert_model_to_base(model);

    // Create node with correct kind.
    let node_handle = if !model.geoms.is_empty() {
        convert_mesh(
            base,
            fbx_scene,
            resource_manager,
            model,
            graph,
            model_path,
            model_import_options,
        )
        .await?
    } else if model.light.is_some() {
        fbx_scene.get(model.light).as_light()?.convert(base, graph)
    } else {
        PivotBuilder::new(base).build(graph)
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

        fn fill_track<F: Fn(f32) -> f32>(
            track: &mut Track,
            fbx_scene: &FbxScene,
            fbx_track: &FbxAnimationCurveNode,
            default: Vector3<f32>,
            transform_value: F,
        ) {
            let curves = track.data_container_mut().curves_mut();

            if !fbx_track.curves.contains_key("d|X") {
                curves[0].add_key(CurveKey::new(0.0, default.x, CurveKeyKind::Constant));
            }
            if !fbx_track.curves.contains_key("d|Y") {
                curves[1].add_key(CurveKey::new(0.0, default.y, CurveKeyKind::Constant));
            }
            if !fbx_track.curves.contains_key("d|Z") {
                curves[2].add_key(CurveKey::new(0.0, default.z, CurveKeyKind::Constant));
            }

            for (id, curve_handle) in fbx_track.curves.iter() {
                let index = match id.as_str() {
                    "d|X" => Some(0),
                    "d|Y" => Some(1),
                    "d|Z" => Some(2),
                    _ => None,
                };

                if let Some(index) = index {
                    if let FbxComponent::AnimationCurve(fbx_curve) = fbx_scene.get(*curve_handle) {
                        if fbx_curve.keys.is_empty() {
                            curves[index].add_key(CurveKey::new(
                                0.0,
                                default[index],
                                CurveKeyKind::Constant,
                            ));
                        } else {
                            for pair in fbx_curve.keys.iter() {
                                curves[index].add_key(CurveKey::new(
                                    pair.time,
                                    transform_value(pair.value),
                                    CurveKeyKind::Linear,
                                ))
                            }
                        }
                    }
                }
            }
        }

        fn add_vec3_key(track: &mut Track, value: Vector3<f32>) {
            let curves = track.data_container_mut().curves_mut();
            curves[0].add_key(CurveKey::new(0.0, value.x, CurveKeyKind::Constant));
            curves[1].add_key(CurveKey::new(0.0, value.y, CurveKeyKind::Constant));
            curves[2].add_key(CurveKey::new(0.0, value.z, CurveKeyKind::Constant));
        }

        // Convert to engine format
        let mut translation_track = Track::new_position();
        translation_track.set_target(node_handle);
        if let Some(lcl_translation) = lcl_translation {
            fill_track(
                &mut translation_track,
                fbx_scene,
                lcl_translation,
                model.translation,
                |v| v,
            );
        } else {
            add_vec3_key(&mut translation_track, model.translation);
        }

        let mut rotation_track = Track::new_rotation();
        rotation_track.set_target(node_handle);
        if let Some(lcl_rotation) = lcl_rotation {
            fill_track(
                &mut rotation_track,
                fbx_scene,
                lcl_rotation,
                model.rotation,
                |v| v.to_radians(),
            );
        } else {
            add_vec3_key(&mut rotation_track, model.rotation);
        }

        let mut scale_track = Track::new_scale();
        scale_track.set_target(node_handle);
        if let Some(lcl_scale) = lcl_scale {
            fill_track(&mut scale_track, fbx_scene, lcl_scale, model.scale, |v| v);
        } else {
            add_vec3_key(&mut scale_track, model.scale);
        }

        animation.add_track(translation_track);
        animation.add_track(rotation_track);
        animation.add_track(scale_track);
    }

    animation.fit_length_to_content();

    Ok(node_handle)
}

///
/// Converts FBX DOM to native engine representation.
///
async fn convert(
    fbx_scene: &FbxScene,
    resource_manager: ResourceManager,
    scene: &mut Scene,
    model_path: &Path,
    model_import_options: &ModelImportOptions,
) -> Result<(), FbxError> {
    let root = scene.graph.get_root();

    let mut animation = Animation::default();
    animation.set_name("Animation");

    let mut fbx_model_to_node_map = FxHashMap::default();
    for (component_handle, component) in fbx_scene.pair_iter() {
        if let FbxComponent::Model(model) = component {
            let node = convert_model(
                fbx_scene,
                model,
                resource_manager.clone(),
                &mut scene.graph,
                &mut animation,
                model_path,
                model_import_options,
            )
            .await?;
            scene.graph.link_nodes(node, root);
            fbx_model_to_node_map.insert(component_handle, node);
        }
    }

    // Do not create animation player if there's no animation content.
    if !animation.tracks().is_empty() {
        let mut animations_container = AnimationContainer::new();
        animations_container.add(animation);
        AnimationPlayerBuilder::new(BaseBuilder::new().with_name("AnimationPlayer"))
            .with_animations(animations_container)
            .build(&mut scene.graph);
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
        if let Some(mesh) = scene.graph[handle].cast_mut::<Mesh>() {
            let mut surface_bones = FxHashSet::default();
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
                surface
                    .bones
                    .set_value_silent(surface_bones.iter().copied().collect());

                let data_rc = surface.data();
                let mut data = data_rc.data_ref();
                if data.vertex_buffer.vertex_count() as usize == surface.vertex_weights.len() {
                    let mut vertex_buffer_mut = data.vertex_buffer.modify();
                    for (mut view, weight_set) in vertex_buffer_mut
                        .iter_mut()
                        .zip(surface.vertex_weights.iter())
                    {
                        let mut indices = Vector4::default();
                        let mut weights = Vector4::default();
                        for (k, weight) in weight_set.iter().enumerate() {
                            indices[k] = surface
                                .bones
                                .iter()
                                .position(|bone_handle| *bone_handle == weight.effector.into())
                                .ok_or(FbxError::UnableToFindBone)?
                                as u8;
                            weights[k] = weight.value;
                        }

                        view.write_4_f32(VertexAttributeUsage::BoneWeight, weights)
                            .unwrap();
                        view.write_4_u8(VertexAttributeUsage::BoneIndices, indices)
                            .unwrap();
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
pub async fn load_to_scene<P: AsRef<Path>>(
    scene: &mut Scene,
    resource_manager: ResourceManager,
    io: &dyn ResourceIo,
    path: P,
    model_import_options: &ModelImportOptions,
) -> Result<(), FbxError> {
    let start_time = Instant::now();

    Log::writeln(
        MessageKind::Information,
        format!("Trying to load {:?}", path.as_ref()),
    );

    let now = Instant::now();
    let fbx = FbxDocument::new(path.as_ref(), io).await?;
    let parsing_time = now.elapsed().as_millis();

    let now = Instant::now();
    let fbx_scene = FbxScene::new(&fbx)?;
    let dom_prepare_time = now.elapsed().as_millis();

    let now = Instant::now();
    convert(
        &fbx_scene,
        resource_manager,
        scene,
        path.as_ref(),
        model_import_options,
    )
    .await?;
    let conversion_time = now.elapsed().as_millis();

    Log::writeln(MessageKind::Information,
                 format!("FBX {:?} loaded in {} ms\n\t- Parsing - {} ms\n\t- DOM Prepare - {} ms\n\t- Conversion - {} ms",
                         path.as_ref(), start_time.elapsed().as_millis(), parsing_time, dom_prepare_time, conversion_time));

    // Check for multiple nodes with same name and throw a warning if any.
    // It seems that FBX was designed using ass, not brains. It has no unique **persistent**
    // IDs for entities, so the only way to find an entity is to use its name, but FBX also
    // allows to have multiple entities with the same name. facepalm.jpg
    let mut hash_set = FxHashSet::<String>::default();
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
