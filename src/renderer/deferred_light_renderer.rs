use std::ffi::CString;
use crate::{
    scene::{
        camera::Camera, node::NodeKind, Scene,
        node::Node, SceneInterface,
    },
    renderer::{
        surface::SurfaceSharedData,
        gpu_program::{UniformLocation, GpuProgram},
        gl, gbuffer::GBuffer,
        FlatShader, error::RendererError,
        shadow_map_renderer::SpotShadowMapRenderer,
    },
};
use rg3d_core::{
    color::Color,
    math::{vec3::Vec3, mat4::Mat4},
};
use crate::scene::light::LightKind;
use crate::renderer::gpu_texture::GpuTexture;

struct AmbientLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    ambient_color: UniformLocation,
}

impl AmbientLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(include_str!("shaders/ambient_light_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/ambient_light_vs.glsl"))?;
        let mut program = GpuProgram::from_source("AmbientLightShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            ambient_color: program.get_uniform_location("ambientColor")?,
            program,
        })
    }

    fn bind(&self) {
        self.program.bind()
    }

    fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat)
    }

    fn set_diffuse_texture(&self, i: i32) {
        self.program.set_int(self.diffuse_texture, i)
    }

    fn set_ambient_color(&self, color: Color) {
        self.program.set_vec4(self.ambient_color, &color.as_frgba())
    }
}

struct DeferredLightingShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    depth_sampler: UniformLocation,
    color_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    spot_shadow_texture: UniformLocation,
    point_shadow_texture: UniformLocation,
    light_view_proj_matrix: UniformLocation,
    light_type: UniformLocation,
    soft_shadows: UniformLocation,
    shadow_map_inv_size: UniformLocation,
    light_position: UniformLocation,
    light_radius: UniformLocation,
    light_color: UniformLocation,
    light_direction: UniformLocation,
    light_cone_angle_cos: UniformLocation,
    inv_view_proj_matrix: UniformLocation,
    camera_position: UniformLocation,
}

impl DeferredLightingShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(include_str!("shaders/deferred_light_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/deferred_light_vs.glsl"))?;
        let mut program = GpuProgram::from_source("DeferredLightShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            depth_sampler: program.get_uniform_location("depthTexture")?,
            color_sampler: program.get_uniform_location("colorTexture")?,
            normal_sampler: program.get_uniform_location("normalTexture")?,
            spot_shadow_texture: program.get_uniform_location("spotShadowTexture")?,
            point_shadow_texture: program.get_uniform_location("pointShadowTexture")?,
            light_view_proj_matrix: program.get_uniform_location("lightViewProjMatrix")?,
            light_type: program.get_uniform_location("lightType")?,
            soft_shadows: program.get_uniform_location("softShadows")?,
            shadow_map_inv_size: program.get_uniform_location("shadowMapInvSize")?,
            light_position: program.get_uniform_location("lightPos")?,
            light_radius: program.get_uniform_location("lightRadius")?,
            light_color: program.get_uniform_location("lightColor")?,
            light_direction: program.get_uniform_location("lightDirection")?,
            light_cone_angle_cos: program.get_uniform_location("coneAngleCos")?,
            inv_view_proj_matrix: program.get_uniform_location("invViewProj")?,
            camera_position: program.get_uniform_location("cameraPosition")?,
            program,
        })
    }

    fn bind(&self) {
        self.program.bind();
    }

    fn set_wvp_matrix(&self, mat4: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat4)
    }

    fn set_depth_sampler_id(&self, id: i32) {
        self.program.set_int(self.depth_sampler, id)
    }

    fn set_color_sampler_id(&self, id: i32) {
        self.program.set_int(self.color_sampler, id)
    }

    fn set_normal_sampler_id(&self, id: i32) {
        self.program.set_int(self.normal_sampler, id)
    }

    fn set_spot_shadow_texture(&self, id: i32) {
        self.program.set_int(self.spot_shadow_texture, id)
    }

    fn set_point_shadow_texture(&self, id: i32) {
        self.program.set_int(self.point_shadow_texture, id)
    }

    fn set_light_view_proj_matrix(&self, mat4: &Mat4) {
        self.program.set_mat4(self.light_view_proj_matrix, mat4)
    }

    fn set_light_type(&self, light_type: i32) {
        self.program.set_int(self.light_type, light_type)
    }

    fn set_soft_shadows_enabled(&self, enabled: bool) {
        self.program.set_int(self.soft_shadows, if enabled { 1 } else { 0 })
    }

    fn set_shadow_map_inv_size(&self, value: f32) {
        self.program.set_float(self.shadow_map_inv_size, value)
    }

    fn set_light_position(&self, pos: &Vec3) {
        self.program.set_vec3(self.light_position, pos)
    }

    fn set_light_radius(&self, radius: f32) {
        self.program.set_float(self.light_radius, radius)
    }

    fn set_light_color(&self, color: Color) {
        self.program.set_vec4(self.light_color, &color.as_frgba())
    }

    fn set_light_direction(&self, direction: &Vec3) {
        self.program.set_vec3(self.light_direction, direction)
    }

    fn set_light_cone_angle_cos(&self, cone_angle_cos: f32) {
        self.program.set_float(self.light_cone_angle_cos, cone_angle_cos)
    }

    fn set_inv_view_proj_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.inv_view_proj_matrix, mat)
    }

    fn set_camera_position(&self, pos: &Vec3) {
        self.program.set_vec3(self.camera_position, pos)
    }
}

pub struct DeferredLightRenderer {
    shader: DeferredLightingShader,
    ambient_light_shader: AmbientLightShader,
    quad: SurfaceSharedData,
    sphere: SurfaceSharedData,
    flat_shader: FlatShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
}

impl DeferredLightRenderer {
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            shader: DeferredLightingShader::new()?,
            ambient_light_shader: AmbientLightShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(1024)?,
        })
    }

    pub fn render(&mut self, frame_width: f32, frame_height: f32, scene: &Scene, camera_node: &Node,
                  camera: &Camera, gbuffer: &GBuffer, white_dummy: &GpuTexture) {
        let frame_matrix =
            Mat4::ortho(0.0, frame_width, frame_height, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::make(frame_width, frame_height, 0.0));

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, gbuffer.opt_fbo);
            gl::Viewport(0, 0, frame_width as i32, frame_height as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            gl::Disable(gl::BLEND);
            gl::DepthMask(gl::FALSE);
            gl::StencilMask(0xFF);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::CULL_FACE);

            // Ambient light.
            self.ambient_light_shader.bind();
            self.ambient_light_shader.set_wvp_matrix(&frame_matrix);
            self.ambient_light_shader.set_ambient_color(Color::opaque(100, 100, 100));
            self.ambient_light_shader.set_diffuse_texture(0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, gbuffer.color_texture);
            self.quad.draw();

            // Lighting
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, gbuffer.depth_texture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, gbuffer.color_texture);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, gbuffer.normal_texture);

            let view_projection = camera.get_view_projection_matrix();
            let inv_view_projection = view_projection.inverse().unwrap();

            let SceneInterface { graph, .. } = scene.interface();

            for light_node in graph.linear_iter() {
                if !light_node.get_global_visibility() {
                    continue;
                }

                let light = match light_node.get_kind() {
                    NodeKind::Light(light) => light,
                    _ => continue
                };

                let light_position = light_node.get_global_position();
                let light_r_inflate = light.get_radius() * 1.05;
                let light_radius_vec = Vec3::make(light_r_inflate, light_r_inflate, light_r_inflate);
                let light_emit_direction = light_node.get_up_vector().normalized().unwrap_or(Vec3::up());

                match light.get_kind() {
                    LightKind::Spot => {
                        self.spot_shadow_map_renderer.render(graph, &Mat4::identity(), white_dummy, gbuffer.opt_fbo);
                    }
                    LightKind::Point => {}
                }

                let light_type = match light.get_kind() {
                    LightKind::Spot => 2,
                    LightKind::Point => -1, // TODO
                };

                // Mark lighted areas in stencil buffer to do light calculations only on them.
                self.flat_shader.bind();
                self.flat_shader.set_wvp_matrix(&(view_projection * Mat4::translate(light_position) *
                    Mat4::scale(light_radius_vec)));

                gl::Enable(gl::STENCIL_TEST);
                gl::StencilMask(0xFF);
                gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);

                gl::Enable(gl::CULL_FACE);

                gl::CullFace(gl::FRONT);
                gl::StencilFunc(gl::ALWAYS, 0, 0xFF);
                gl::StencilOp(gl::KEEP, gl::INCR, gl::KEEP);
                self.sphere.draw();

                gl::CullFace(gl::BACK);
                gl::StencilFunc(gl::ALWAYS, 0, 0xFF);
                gl::StencilOp(gl::KEEP, gl::DECR, gl::KEEP);
                self.sphere.draw();

                gl::StencilFunc(gl::NOTEQUAL, 0, 0xFF);
                gl::StencilOp(gl::KEEP, gl::KEEP, gl::ZERO);

                gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

                gl::Disable(gl::CULL_FACE);

                // Finally render light.
                self.shader.bind();
                self.shader.set_light_position(&light_position);
                self.shader.set_light_direction(&light_emit_direction);
                self.shader.set_light_type(light_type); // Disable shadows for now
                self.shader.set_light_radius(light.get_radius());
                self.shader.set_inv_view_proj_matrix(&inv_view_projection);
                self.shader.set_light_color(light.get_color());
                self.shader.set_light_cone_angle_cos(light.get_cone_angle_cos());
                self.shader.set_wvp_matrix(&frame_matrix);
                self.shader.set_shadow_map_inv_size(0.0); // TODO
                self.shader.set_camera_position(&camera_node.get_global_position());
                self.shader.set_depth_sampler_id(0);
                self.shader.set_color_sampler_id(1);
                self.shader.set_normal_sampler_id(2);

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, gbuffer.depth_texture);

                self.quad.draw();

                gl::ActiveTexture(gl::TEXTURE3);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);
            }

            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::BLEND);

            gl::DepthMask(gl::TRUE);

            // Unbind FBO textures.
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}