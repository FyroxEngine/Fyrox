use crate::renderer::framework::gpu_program::GpuProgramBinding;
use crate::{
    core::{algebra::Vector4, color::Color, math::Rect, scope_profile},
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::{
            Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
            PixelKind, WrapMode,
        },
        state::PipelineState,
    },
    renderer::{
        batch::{BatchStorage, InstanceData, MatrixStorage, BONE_MATRICES_COUNT},
        GeometryCache, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, mesh::RenderPath},
};
use glow::HasContext;
use std::{cell::RefCell, rc::Rc};

fn make_vertex_shader_source(instancing: bool, lightmaps: bool) -> String {
    let mut source = "#version 330 core\n".to_owned();

    if instancing {
        if lightmaps {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;            
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 6) in vec2 vertexSecondTexCoord;
            layout(location = 7) in vec4 instanceColor;
            layout(location = 8) in mat4 worldMatrix;
            layout(location = 12) in float depthOffset;
            "#;
        } else {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 7) in vec4 instanceColor;
            layout(location = 8) in mat4 worldMatrix;
            layout(location = 12) in float depthOffset;
            "#;
        }

        source += r#"
            uniform sampler2D matrixStorage;
            uniform vec4 matrixStorageSize; // vec4(1/w, 1/h, w, h)
            uniform mat4 viewProjectionMatrix;
            uniform int matrixBufferStride;

            vec2 IdToCoords(float k, float w, float inv_w) {
                float y = floor(k * inv_w); // floor(k / w)
                float x = k - w * y; // k % w
                return vec2(x, y);
            }

            mat4 ReadMatrix(int id)
            {
                float w = matrixStorageSize.z;
                float inv_w = matrixStorageSize.x;
                float inv_h = matrixStorageSize.y;

                vec2 coords = IdToCoords(4.0 * float(id), w, inv_w);

                float ty = (coords.y + 0.5) * inv_h;

                vec4 col1 = texture(matrixStorage, vec2((coords.x + 0.5) * inv_w, ty));
                vec4 col2 = texture(matrixStorage, vec2((coords.x + 1.5) * inv_w, ty));
                vec4 col3 = texture(matrixStorage, vec2((coords.x + 2.5) * inv_w, ty));
                vec4 col4 = texture(matrixStorage, vec2((coords.x + 3.5) * inv_w, ty));

                return mat4(col1, col2, col3, col4);
            }
        "#;
    } else {
        if lightmaps {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;            
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 6) in vec2 vertexSecondTexCoord;
            "#;
        } else {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            "#;
        }

        source += r#"
            uniform mat4 worldMatrix;
            uniform mat4 worldViewProjection;
            uniform mat4 boneMatrices[60];
        "#;
    }

    source += r#"
        uniform bool useSkeletalAnimation;

        out vec3 position;
        out vec3 normal;
        out vec2 texCoord;
        out vec3 tangent;
        out vec3 binormal;        
    "#;

    if lightmaps {
        source += r#"
        out vec2 secondTexCoord;
    "#;
    }

    if instancing {
        source += r#"
        out vec4 diffuseColor;
    "#;
    }

    source += r#"
        void main()
        {
            vec4 localPosition = vec4(0);
            vec3 localNormal = vec3(0);
            vec3 localTangent = vec3(0);
    
            if (useSkeletalAnimation)
            {
                vec4 vertex = vec4(vertexPosition, 1.0);
    
                int i0 = int(boneIndices.x);
                int i1 = int(boneIndices.y);
                int i2 = int(boneIndices.z);
                int i3 = int(boneIndices.w);
                "#;

    if instancing {
        source += r#"
                int boneIndexOrigin = gl_InstanceID * matrixBufferStride;

                mat4 m0 = ReadMatrix(boneIndexOrigin + i0);
                mat4 m1 = ReadMatrix(boneIndexOrigin + i1);
                mat4 m2 = ReadMatrix(boneIndexOrigin + i2);
                mat4 m3 = ReadMatrix(boneIndexOrigin + i3);
                "#;
    } else {
        source += r#"
                mat4 m0 = boneMatrices[i0];
                mat4 m1 = boneMatrices[i1];
                mat4 m2 = boneMatrices[i2];
                mat4 m3 = boneMatrices[i3];
                "#;
    }

    source += r#"
                localPosition += m0 * vertex * boneWeights.x;
                localPosition += m1 * vertex * boneWeights.y;
                localPosition += m2 * vertex * boneWeights.z;
                localPosition += m3 * vertex * boneWeights.w;
                
                localNormal += mat3(m0) * vertexNormal * boneWeights.x;
                localNormal += mat3(m1) * vertexNormal * boneWeights.y;
                localNormal += mat3(m2) * vertexNormal * boneWeights.z;
                localNormal += mat3(m3) * vertexNormal * boneWeights.w;
                
                localTangent += mat3(m0) * vertexTangent.xyz * boneWeights.x;
                localTangent += mat3(m1) * vertexTangent.xyz * boneWeights.y;
                localTangent += mat3(m2) * vertexTangent.xyz * boneWeights.z;
                localTangent += mat3(m3) * vertexTangent.xyz * boneWeights.w;             
            }
            else
            {
                localPosition = vec4(vertexPosition, 1.0);
                localNormal = vertexNormal;
                localTangent = vertexTangent.xyz;
            }

            mat3 nm = mat3(worldMatrix);
            normal = normalize(nm * localNormal);
            tangent = normalize(nm * localTangent);
            binormal = normalize(vertexTangent.w * cross(tangent, normal));
            texCoord = vertexTexCoord;
            position = vec3(worldMatrix * localPosition);            
            "#;

    if lightmaps {
        source += r#"
        secondTexCoord = vertexSecondTexCoord;
    "#;
    }

    if instancing {
        source += r#"
            mat4 viewProj = viewProjectionMatrix;
            viewProj[3].z -= depthOffset;
            gl_Position = (viewProj * worldMatrix) * localPosition;
            diffuseColor = instanceColor;
        "#;
    } else {
        source += r#"
            gl_Position = worldViewProjection * localPosition;
        "#;
    }

    source += r#"}"#;

    source
}

fn make_fragment_shader_source(instancing: bool, lightmaps: bool) -> String {
    let mut source = r#"#version 330 core
    "#
    .to_owned();

    source += r#"
        layout(location = 0) out vec4 outColor;
        layout(location = 1) out vec4 outNormal;
        layout(location = 2) out vec4 outAmbient;

        uniform sampler2D diffuseTexture;
        uniform sampler2D normalTexture;
        uniform sampler2D specularTexture;
        uniform sampler2D roughnessTexture;
        uniform sampler2D heightTexture;
        uniform samplerCube environmentMap;
        uniform vec3 cameraPosition;
        uniform bool usePOM;
    "#;

    source += r#"
        in vec3 position;
        in vec3 normal;
        in vec2 texCoord;
        in vec3 tangent;
        in vec3 binormal;
    "#;

    if lightmaps {
        source += r#"
        in vec2 secondTexCoord;
        uniform sampler2D lightmapTexture;
        "#;
    }

    if instancing {
        source += r#"
        in vec4 diffuseColor;
        "#;
    } else {
        source += r#"
        uniform vec4 diffuseColor;
        "#;
    }

    source += r#"
    void main()
    {
        mat3 tangentSpace = mat3(tangent, binormal, normal);
        vec3 toFragment = normalize(position - cameraPosition);

        vec2 tc;
        if (usePOM) {
            vec3 toFragmentTangentSpace = normalize(transpose(tangentSpace) * toFragment);
            tc = S_ComputeParallaxTextureCoordinates(heightTexture, toFragmentTangentSpace, texCoord, normal);
        } else {
            tc = texCoord;
        }

        outColor = diffuseColor * texture(diffuseTexture, tc);
        if (outColor.a < 0.5) {
            discard;
        }
        outColor.a = 1.0;
        vec4 n = normalize(texture(normalTexture, tc) * 2.0 - 1.0);
        outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
        outNormal.w = texture(specularTexture, tc).r;
        "#;

    if lightmaps {
        source += r#"outAmbient = vec4(texture(lightmapTexture, secondTexCoord).rgb, 1.0);"#;
    } else {
        source += r#"outAmbient = vec4(0.0, 0.0, 0.0, 1.0);"#;
    }

    source += r#"
        // reflection mapping
        float roughness = texture(roughnessTexture, tc).r;
        vec3 reflectionTexCoord = reflect(toFragment, normalize(n.xyz));
        outColor = (1.0 - roughness) * outColor + roughness * vec4(texture(environmentMap, reflectionTexCoord).rgb, outColor.a);
    }
    "#;

    source
}

struct UberShader {
    program: GpuProgram,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
    specular_texture: UniformLocation,
    roughness_texture: UniformLocation,
    lightmap_texture: Option<UniformLocation>,
    matrix_buffer_stride: Option<UniformLocation>,
    matrix_storage_size: Option<UniformLocation>,
    matrix_storage: Option<UniformLocation>,
    environment_map: UniformLocation,
    camera_position: UniformLocation,
    view_projection_matrix: Option<UniformLocation>,
    use_pom: UniformLocation,
    height_texture: UniformLocation,
    // Non-instanced parts.
    world_matrix: Option<UniformLocation>,
    wvp_matrix: Option<UniformLocation>,
    bone_matrices: Option<UniformLocation>,
    diffuse_color: Option<UniformLocation>,
}

impl UberShader {
    fn new(
        state: &mut PipelineState,
        instancing: bool,
        lightmaps: bool,
    ) -> Result<Self, FrameworkError> {
        let name = match (instancing, lightmaps) {
            (false, false) => "GBufferShader",
            (false, true) => "GBufferLightmapShader",
            (true, false) => "GBufferInstancedShader",
            (true, true) => "GBufferInstancedLightmapShader",
        };
        let fragment_source = make_fragment_shader_source(instancing, lightmaps);
        let vertex_source = make_vertex_shader_source(instancing, lightmaps);
        let program = GpuProgram::from_source(state, name, &vertex_source, &fragment_source)?;
        Ok(Self {
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            world_matrix: if !instancing {
                Some(program.uniform_location(state, "worldMatrix")?)
            } else {
                None
            },
            wvp_matrix: if !instancing {
                Some(program.uniform_location(state, "worldViewProjection")?)
            } else {
                None
            },
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            normal_texture: program.uniform_location(state, "normalTexture")?,
            specular_texture: program.uniform_location(state, "specularTexture")?,
            roughness_texture: program.uniform_location(state, "roughnessTexture")?,
            lightmap_texture: if lightmaps {
                Some(program.uniform_location(state, "lightmapTexture")?)
            } else {
                None
            },
            matrix_buffer_stride: if instancing {
                Some(program.uniform_location(state, "matrixBufferStride")?)
            } else {
                None
            },
            matrix_storage_size: if instancing {
                Some(program.uniform_location(state, "matrixStorageSize")?)
            } else {
                None
            },
            matrix_storage: if instancing {
                Some(program.uniform_location(state, "matrixStorage")?)
            } else {
                None
            },
            environment_map: program.uniform_location(state, "environmentMap")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            view_projection_matrix: if instancing {
                Some(program.uniform_location(state, "viewProjectionMatrix")?)
            } else {
                None
            },
            use_pom: program.uniform_location(state, "usePOM")?,
            height_texture: program.uniform_location(state, "heightTexture")?,
            diffuse_color: if !instancing {
                Some(program.uniform_location(state, "diffuseColor")?)
            } else {
                None
            },
            bone_matrices: if !instancing {
                Some(program.uniform_location(state, "boneMatrices")?)
            } else {
                None
            },
            program,
        })
    }
}

pub struct GBuffer {
    framebuffer: FrameBuffer,
    pub final_frame: FrameBuffer,
    instanced_shader: UberShader,
    instanced_shader_lightmaps: UberShader,
    shader: UberShader,
    shader_lightmaps: UberShader,
    pub width: i32,
    pub height: i32,
    matrix_storage: MatrixStorage,
    instance_data_set: Vec<InstanceData>,
}

pub(in crate) struct GBufferRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub texture_cache: &'a mut TextureCache,
    pub environment_dummy: Rc<RefCell<GpuTexture>>,
    pub use_parallax_mapping: bool,
}

impl GBuffer {
    pub fn new(
        state: &mut PipelineState,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        scope_profile!();

        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));

        let mut diffuse_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        diffuse_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut normal_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        normal_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let mut ambient_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        ambient_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let framebuffer = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: depth_stencil,
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(diffuse_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(normal_texture)),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(ambient_texture)),
                },
            ],
        )?;

        let mut final_frame_depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        final_frame_depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let final_frame_depth_stencil = Rc::new(RefCell::new(final_frame_depth_stencil_texture));

        let frame_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Linear,
            MagnificationFilter::Linear,
            1,
            None,
        )?;

        let final_frame = FrameBuffer::new(
            state,
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: final_frame_depth_stencil,
            }),
            vec![Attachment {
                kind: AttachmentKind::Color,
                texture: Rc::new(RefCell::new(frame_texture)),
            }],
        )?;

        Ok(Self {
            framebuffer,
            instanced_shader: UberShader::new(state, true, false)?,
            instanced_shader_lightmaps: UberShader::new(state, true, true)?,
            shader: UberShader::new(state, false, false)?,
            shader_lightmaps: UberShader::new(state, false, true)?,
            width: width as i32,
            height: height as i32,
            final_frame,
            matrix_storage: MatrixStorage::new(state)?,
            instance_data_set: Default::default(),
        })
    }

    pub fn frame_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.final_frame.color_attachments()[0].texture.clone()
    }

    pub fn depth(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.depth_attachment().unwrap().texture.clone()
    }

    pub fn diffuse_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn normal_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[1].texture.clone()
    }

    pub fn ambient_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[2].texture.clone()
    }

    #[must_use]
    pub(in crate) fn fill(&mut self, args: GBufferRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            state,
            camera,
            geom_cache,
            batch_storage,
            texture_cache,
            environment_dummy,
            use_parallax_mapping,
        } = args;

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let params = DrawParameters {
            cull_face: CullFace::Back,
            culling: true,
            color_write: Default::default(),
            depth_write: true,
            stencil_test: false,
            depth_test: true,
            blend: false,
        };

        let initial_view_projection = camera.view_projection_matrix();

        for batch in batch_storage
            .batches
            .iter()
            .filter(|b| b.render_path == RenderPath::Deferred)
        {
            let data = batch.data.read().unwrap();
            let geometry = geom_cache.get(state, &data);
            let use_instanced_rendering = batch.instances.len() > 1;

            let environment = match camera.environment_ref() {
                Some(texture) => texture_cache.get(state, texture).unwrap(),
                None => environment_dummy.clone(),
            };

            // Prepare batch info storage in case if we're rendering multiple objects
            // at once.
            if use_instanced_rendering {
                self.matrix_storage.clear();
                self.instance_data_set.clear();
                for instance in batch.instances.iter() {
                    if camera.visibility_cache.is_visible(instance.owner) {
                        self.instance_data_set.push(InstanceData {
                            color: instance.color,
                            world: instance.world_transform,
                            depth_offset: instance.depth_offset,
                        });
                        self.matrix_storage
                            .push_slice(instance.bone_matrices.as_slice());
                    }
                }
                // Every object from batch might be clipped.
                if !self.instance_data_set.is_empty() {
                    self.matrix_storage.update(state);
                    geometry.set_buffer_data(state, 1, self.instance_data_set.as_slice());
                }
            }

            // Select shader
            let shader = if batch.use_lightmapping {
                if use_instanced_rendering {
                    &self.instanced_shader_lightmaps
                } else {
                    &self.shader_lightmaps
                }
            } else if use_instanced_rendering {
                &self.instanced_shader
            } else {
                &self.shader
            };

            let need_render = if use_instanced_rendering {
                !self.instance_data_set.is_empty()
            } else {
                camera
                    .visibility_cache
                    .is_visible(batch.instances.first().unwrap().owner)
            };

            if need_render {
                let matrix_storage = &self.matrix_storage;

                let apply_uniforms = |program_binding: GpuProgramBinding| {
                    let program_binding = program_binding
                        .set_texture(&shader.diffuse_texture, &batch.diffuse_texture)
                        .set_texture(&shader.normal_texture, &batch.normal_texture)
                        .set_texture(&shader.specular_texture, &batch.specular_texture)
                        .set_texture(&shader.environment_map, &environment)
                        .set_texture(&shader.roughness_texture, &batch.roughness_texture)
                        .set_texture(&shader.height_texture, &batch.height_texture)
                        .set_vector3(&shader.camera_position, &camera.global_position())
                        .set_bool(&shader.use_pom, batch.use_pom && use_parallax_mapping)
                        .set_bool(&shader.use_skeletal_animation, batch.is_skinned);

                    let program_binding = if batch.use_lightmapping {
                        program_binding.set_texture(
                            shader.lightmap_texture.as_ref().unwrap(),
                            &batch.lightmap_texture,
                        )
                    } else {
                        program_binding
                    };

                    if use_instanced_rendering {
                        program_binding
                            .set_texture(
                                shader.matrix_storage.as_ref().unwrap(),
                                &matrix_storage.matrices_storage,
                            )
                            .set_integer(
                                shader.matrix_buffer_stride.as_ref().unwrap(),
                                BONE_MATRICES_COUNT as i32,
                            )
                            .set_vector4(shader.matrix_storage_size.as_ref().unwrap(), {
                                let kind = matrix_storage.matrices_storage.borrow().kind();
                                let (w, h) =
                                    if let GpuTextureKind::Rectangle { width, height } = kind {
                                        (width, height)
                                    } else {
                                        unreachable!()
                                    };
                                &Vector4::new(
                                    1.0 / (w as f32),
                                    1.0 / (h as f32),
                                    w as f32,
                                    h as f32,
                                )
                            })
                            .set_matrix4(
                                shader.view_projection_matrix.as_ref().unwrap(),
                                &camera.view_projection_matrix(),
                            );
                    } else {
                        let instance = batch.instances.first().unwrap();

                        let view_projection = if instance.depth_offset != 0.0 {
                            let mut projection = camera.projection_matrix();
                            projection[14] -= instance.depth_offset;
                            projection * camera.view_matrix()
                        } else {
                            initial_view_projection
                        };
                        program_binding
                            .set_color(shader.diffuse_color.as_ref().unwrap(), &instance.color)
                            .set_matrix4(
                                shader.wvp_matrix.as_ref().unwrap(),
                                &(view_projection * instance.world_transform),
                            )
                            .set_matrix4_array(
                                shader.bone_matrices.as_ref().unwrap(),
                                instance.bone_matrices.as_slice(),
                            )
                            .set_matrix4(
                                shader.world_matrix.as_ref().unwrap(),
                                &instance.world_transform,
                            );
                    }
                };

                statistics += if use_instanced_rendering {
                    self.framebuffer.draw_instances(
                        batch.instances.len(),
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        apply_uniforms,
                    )
                } else {
                    self.framebuffer.draw(
                        geometry,
                        state,
                        viewport,
                        &shader.program,
                        &params,
                        apply_uniforms,
                    )
                };
            }
        }

        // Copy depth-stencil from gbuffer to final frame buffer.
        unsafe {
            state
                .gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer.id()));
            state
                .gl
                .bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.final_frame.id()));
            state.gl.blit_framebuffer(
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                self.width,
                self.height,
                glow::DEPTH_BUFFER_BIT | glow::STENCIL_BUFFER_BIT,
                glow::NEAREST,
            );
        }

        statistics
    }
}
