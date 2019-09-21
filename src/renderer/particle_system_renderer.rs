use std::ffi::CString;
use rg3d_core::math::{
    mat4::Mat4,
    vec3::Vec3,
    vec2::Vec2,
};
use crate::{
    renderer::{
        geometry_buffer::{GeometryBuffer, GeometryBufferKind, AttributeDefinition, AttributeKind},
        gl,
        gpu_program::{GpuProgram, UniformLocation},
        gbuffer::GBuffer,
        error::RendererError,
        gpu_texture::GpuTexture
    },
    engine::state::State,
    scene::{
        node::NodeKind,
        particle_system,
    },
};

struct ParticleSystemShader {
    program: GpuProgram,
    view_projection_matrix: UniformLocation,
    world_matrix: UniformLocation,
    camera_side_vector: UniformLocation,
    camera_up_vector: UniformLocation,
    diffuse_texture: UniformLocation,
    depth_buffer_texture: UniformLocation,
    inv_screen_size: UniformLocation,
    proj_params: UniformLocation,
}

impl ParticleSystemShader {
    fn new() -> Result<Self, RendererError> {
        let vertex_source = CString::new(r#"
            #version 330 core

            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in float particleSize;
            layout(location = 3) in float particleRotation;
            layout(location = 4) in vec4 vertexColor;

            uniform mat4 viewProjectionMatrix;
            uniform mat4 worldMatrix;
            uniform vec3 cameraUpVector;
            uniform vec3 cameraSideVector;

            out vec2 texCoord;
            out vec4 color;

            vec2 rotateVec2(vec2 v, float angle)
            {
               float c = cos(angle);
               float s = sin(angle);
               mat2 m = mat2(c, -s, s, c);
               return m * v;
            }

            void main()
            {
                color = vertexColor;
                texCoord = vertexTexCoord;
                vec2 vertexOffset = rotateVec2(vertexTexCoord * 2.0 - 1.0, particleRotation);
                vec4 worldPosition = worldMatrix * vec4(vertexPosition, 1.0);
                vec3 offset = (vertexOffset.x * cameraSideVector + vertexOffset.y * cameraUpVector) * particleSize;
                gl_Position = viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
            }"#)?;

        let fragment_source = CString::new(r#"
            #version 330 core

            uniform sampler2D diffuseTexture;
            uniform sampler2D depthBufferTexture;
            uniform vec2 invScreenSize;
            uniform vec2 projParams;

            out vec4 FragColor;
            in vec2 texCoord;
            in vec4 color;

            float toProjSpace(float z)
            {
               float far = projParams.x;
               float near = projParams.y;
                return (far * near) / (far - z * (far + near));
            }

            void main()
            {
               float sceneDepth = toProjSpace(texture(depthBufferTexture, gl_FragCoord.xy * invScreenSize).r);
               float depthOpacity = clamp((sceneDepth - gl_FragCoord.z / gl_FragCoord.w) * 2.0f, 0.0, 1.0);
                FragColor = color * texture(diffuseTexture, texCoord).r;
               FragColor.a *= depthOpacity;
            }
            "#)?;

        let mut program = GpuProgram::from_source("ParticleSystemShader", &vertex_source, &fragment_source)?;

        Ok(Self {
            view_projection_matrix: program.get_uniform_location("viewProjectionMatrix")?,
            world_matrix: program.get_uniform_location("worldMatrix")?,
            camera_side_vector: program.get_uniform_location("cameraSideVector")?,
            camera_up_vector: program.get_uniform_location("cameraUpVector")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            depth_buffer_texture: program.get_uniform_location("depthBufferTexture")?,
            inv_screen_size: program.get_uniform_location("invScreenSize")?,
            proj_params: program.get_uniform_location("projParams")?,
            program,
        })
    }

    pub fn bind(&self) {
        self.program.bind();
    }

    pub fn set_view_projection_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.view_projection_matrix, mat)
    }

    pub fn set_world_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_matrix, mat)
    }

    pub fn set_camera_side_vector(&self, vec: &Vec3) {
        self.program.set_vec3(self.camera_side_vector, vec)
    }

    pub fn set_camera_up_vector(&self, vec: &Vec3) {
        self.program.set_vec3(self.camera_up_vector, vec)
    }

    pub fn set_diffuse_texture(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }

    pub fn set_depth_buffer_texture(&self, id: i32) {
        self.program.set_int(self.depth_buffer_texture, id)
    }

    pub fn set_inv_screen_size(&self, size: Vec2) {
        self.program.set_vec2(self.inv_screen_size, size)
    }

    pub fn set_proj_params(&self, far: f32, near: f32) {
        let params = Vec2::make(far, near);
        self.program.set_vec2(self.proj_params, params);
    }
}

pub struct ParticleSystemRenderer {
    shader: ParticleSystemShader,
    draw_data: particle_system::DrawData,
    geometry_buffer: GeometryBuffer<particle_system::Vertex>,
}

impl ParticleSystemRenderer {
    pub fn new() -> Result<Self, RendererError> {
        let geometry_buffer = GeometryBuffer::new(GeometryBufferKind::DynamicDraw);

        geometry_buffer.describe_attributes(vec![
            AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float, normalized: false },
            AttributeDefinition { kind: AttributeKind::Float, normalized: false },
            AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: true },
        ])?;

        Ok(Self {
            shader: ParticleSystemShader::new()?,
            draw_data: Default::default(),
            geometry_buffer,
        })
    }

    pub fn render(&mut self, state: &State, white_dummy: &GpuTexture, frame_width: f32, frame_height: f32, gbuffer: &GBuffer) {
        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::DepthMask(gl::FALSE);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            self.shader.bind();

            for scene in state.get_scenes().iter() {
                let camera_node = match scene.get_active_camera() {
                    Some(camera_node) => camera_node,
                    None => continue
                };

                let camera =
                    if let NodeKind::Camera(camera) = camera_node.borrow_kind() {
                        camera
                    } else {
                        continue;
                    };

                let inv_view = camera.get_inv_view_matrix().unwrap();

                let camera_up = inv_view.up();
                let camera_side = inv_view.side();

                for node in scene.get_nodes().iter() {
                    let particle_system = if let NodeKind::ParticleSystem(particle_system) = node.borrow_kind() {
                        particle_system
                    } else {
                        continue;
                    };

                    particle_system.generate_draw_data(&mut self.draw_data);

                    self.geometry_buffer.set_triangles(self.draw_data.get_triangles());
                    self.geometry_buffer.set_vertices(self.draw_data.get_vertices());

                    if let Some(texture) = particle_system.get_texture() {
                        texture.lock().unwrap().gpu_tex.as_ref().unwrap().bind(0);
                    } else {
                        white_dummy.bind(0)
                    }

                    gl::ActiveTexture(gl::TEXTURE1);
                    gl::BindTexture(gl::TEXTURE_2D, gbuffer.depth_texture);

                    self.shader.set_diffuse_texture(0);
                    self.shader.set_view_projection_matrix(&camera.get_view_projection_matrix());
                    self.shader.set_world_matrix(node.get_global_transform());
                    self.shader.set_camera_up_vector(&camera_up);
                    self.shader.set_camera_side_vector(&camera_side);
                    self.shader.set_depth_buffer_texture(1);
                    self.shader.set_inv_screen_size(Vec2::make(1.0 / frame_width, 1.0 / frame_height));
                    self.shader.set_proj_params(camera.get_z_far(), camera.get_z_near());

                    self.geometry_buffer.draw();
                }
            }

            gl::Disable(gl::BLEND);
            gl::DepthMask(gl::TRUE);
        }
    }
}
