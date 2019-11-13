use std::ffi::CString;
use crate::{
    scene::{
        camera::Camera,
        Scene,
        node::Node,
        SceneInterface,
        light::LightKind,
        base::AsBase,
    },
    renderer::{
        surface::SurfaceSharedData,
        gpu_program::{UniformLocation, GpuProgram},
        gl,
        gbuffer::GBuffer,
        FlatShader,
        error::RendererError,
        shadow_map_renderer::SpotShadowMapRenderer,
        gpu_texture::GpuTexture,
        shadow_map_renderer::PointShadowMapRenderer,
        QualitySettings,
        RenderPassStatistics
    }
};
use rg3d_core::{
    math::{
        vec2::Vec2,
        vec3::Vec3,
        mat4::Mat4,
        frustum::Frustum,
    },
    color::Color,
};

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
        self.program.set_bool(self.soft_shadows, enabled)
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
    point_shadow_map_renderer: PointShadowMapRenderer,
}

pub struct DeferredRendererContext<'a> {
    pub frame_size: Vec2,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub gbuffer: &'a GBuffer,
    pub white_dummy: &'a GpuTexture,
    pub ambient_color: Color,
    pub settings: &'a QualitySettings,
}

impl DeferredLightRenderer {
    pub fn new(settings: &QualitySettings) -> Result<Self, RendererError> {
        Ok(Self {
            shader: DeferredLightingShader::new()?,
            ambient_light_shader: AmbientLightShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(settings.spot_shadow_map_size)?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(settings.point_shadow_map_size)?,
        })
    }

    pub fn set_quality_settings(&mut self, settings: &QualitySettings) -> Result<(), RendererError> {
        if settings.spot_shadow_map_size != self.spot_shadow_map_renderer.size {
            self.spot_shadow_map_renderer = SpotShadowMapRenderer::new(settings.spot_shadow_map_size)?;
        }
        if settings.point_shadow_map_size != self.point_shadow_map_renderer.size {
            self.point_shadow_map_renderer = PointShadowMapRenderer::new(settings.point_shadow_map_size)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn render(&mut self, context: DeferredRendererContext) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let frustum = Frustum::from(context.camera.get_view_projection_matrix()).unwrap();

        let frame_matrix =
            Mat4::ortho(0.0, context.frame_size.x, context.frame_size.y, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::new(context.frame_size.x, context.frame_size.y, 0.0));

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, context.gbuffer.opt_fbo);
            gl::Viewport(0, 0, context.frame_size.x as i32, context.frame_size.y as i32);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            gl::Disable(gl::BLEND);
            gl::DepthMask(gl::FALSE);
            gl::StencilMask(0xFF);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::CULL_FACE);

            // Ambient light.
            self.ambient_light_shader.bind();
            self.ambient_light_shader.set_wvp_matrix(&frame_matrix);
            self.ambient_light_shader.set_ambient_color(context.ambient_color);
            self.ambient_light_shader.set_diffuse_texture(0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gbuffer.color_texture);
            self.quad.draw();

            // Lighting
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, context.gbuffer.depth_texture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, context.gbuffer.color_texture);
            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, context.gbuffer.normal_texture);

            let view_projection = context.camera.get_view_projection_matrix();
            let inv_view_projection = view_projection.inverse().unwrap();

            let SceneInterface { graph, .. } = context.scene.interface();

            for light_node in graph.linear_iter() {
                if !light_node.base().get_global_visibility() {
                    continue;
                }

                let light = match light_node {
                    Node::Light(light) => light,
                    _ => continue
                };

                let raw_radius = match light.get_kind() {
                    LightKind::Spot(spot_light) => spot_light.get_distance(),
                    LightKind::Point(point_light) => point_light.get_radius(),
                };

                let light_position = light_node.base().get_global_position();
                let light_radius_scale = light_node.base().get_local_transform().get_scale().max_value();
                let light_radius = light_radius_scale * raw_radius;
                let light_r_inflate = 1.05 * light_radius;
                let light_radius_vec = Vec3::new(light_r_inflate, light_r_inflate, light_r_inflate);
                let light_emit_direction = light_node.base().get_up_vector().normalized().unwrap_or(Vec3::UP);

                if !frustum.is_intersects_sphere(light_position, light_radius) {
                    continue;
                }

                let distance_to_camera = (light.base().get_global_position() - context.camera.base().get_global_position()).len();

                let mut light_view_projection = Mat4::IDENTITY;
                let apply_shadows = match light.get_kind() {
                    LightKind::Spot(spot) if distance_to_camera <= context.settings.spot_shadows_distance && context.settings.spot_shadows_enabled => {
                        let light_projection_matrix = Mat4::perspective(
                            spot.get_cone_angle(),
                            1.0,
                            0.01,
                            light_radius,
                        );

                        let emit_direction = light.base().get_up_vector().normalized().unwrap_or(Vec3::LOOK);

                        let light_look_at = light_position - emit_direction;

                        let light_up_vec = light.base().get_look_vector().normalized().unwrap_or(Vec3::UP);

                        let light_view_matrix = Mat4::look_at(light_position, light_look_at, light_up_vec)
                            .unwrap_or_default();

                        light_view_projection = light_projection_matrix * light_view_matrix;

                        statistics += self.spot_shadow_map_renderer.render(
                            graph,
                            &light_view_projection,
                            context.white_dummy,
                        );

                        true
                    }
                    LightKind::Point(_) if distance_to_camera <= context.settings.point_shadows_distance && context.settings.point_shadows_enabled => {
                        statistics += self.point_shadow_map_renderer.render(
                            graph,
                            context.white_dummy,
                            light_position,
                            light_radius,
                        );

                        true
                    }
                    _ => false
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
                statistics.add_draw_call(self.sphere.draw());

                gl::CullFace(gl::BACK);
                gl::StencilFunc(gl::ALWAYS, 0, 0xFF);
                gl::StencilOp(gl::KEEP, gl::DECR, gl::KEEP);
                statistics.add_draw_call(self.sphere.draw());

                gl::StencilFunc(gl::NOTEQUAL, 0, 0xFF);
                gl::StencilOp(gl::KEEP, gl::KEEP, gl::ZERO);

                gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

                gl::Disable(gl::CULL_FACE);

                let cone_angle_cos = match light.get_kind() {
                    LightKind::Spot(spot_light) => spot_light.get_cone_angle_cos(),
                    LightKind::Point(_) => -1.0, // cos(π)
                };

                // Finally render light.
                self.shader.bind();

                match light.get_kind() {
                    LightKind::Spot(_) => {
                        gl::ActiveTexture(gl::TEXTURE3);
                        gl::BindTexture(gl::TEXTURE_2D, self.spot_shadow_map_renderer.texture);
                        self.shader.set_spot_shadow_texture(3);
                        self.shader.set_light_view_proj_matrix(&light_view_projection);
                        self.shader.set_soft_shadows_enabled(context.settings.spot_soft_shadows);
                    }
                    LightKind::Point(_) => {
                        gl::ActiveTexture(gl::TEXTURE3);
                        gl::BindTexture(gl::TEXTURE_CUBE_MAP, self.point_shadow_map_renderer.texture);
                        self.shader.set_point_shadow_texture(3);
                        self.shader.set_soft_shadows_enabled(context.settings.point_soft_shadows);
                    }
                }

                let light_type = match light.get_kind() {
                    LightKind::Spot(_) if apply_shadows => 2,
                    LightKind::Point(_) if apply_shadows => 0,
                    _ => -1
                };

                self.shader.set_light_position(&light_position);
                self.shader.set_light_direction(&light_emit_direction);
                self.shader.set_light_type(light_type);
                self.shader.set_light_radius(light_radius);
                self.shader.set_inv_view_proj_matrix(&inv_view_projection);
                self.shader.set_light_color(light.get_color());
                self.shader.set_light_cone_angle_cos(cone_angle_cos);
                self.shader.set_wvp_matrix(&frame_matrix);
                self.shader.set_shadow_map_inv_size(1.0 / (self.spot_shadow_map_renderer.size as f32));
                self.shader.set_camera_position(&context.camera.base().get_global_position());
                self.shader.set_depth_sampler_id(0);
                self.shader.set_color_sampler_id(1);
                self.shader.set_normal_sampler_id(2);

                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, context.gbuffer.depth_texture);

                statistics.add_draw_call(self.quad.draw());

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

        statistics
    }
}