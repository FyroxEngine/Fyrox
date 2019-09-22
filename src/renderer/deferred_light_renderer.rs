use std::ffi::CString;
use crate::{
    scene::{
        camera::Camera,
        node::NodeKind,
        Scene,
        node::Node,
    },
    renderer::{
        surface::SurfaceSharedData,
        gpu_program::{UniformLocation, GpuProgram},
        gl,
        gbuffer::GBuffer,
        FlatShader,
        error::RendererError,
    },
};
use rg3d_core::{
    color::Color,
    math::{
        vec3::Vec3,
        mat4::Mat4,
    },
};

struct AmbientLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    ambient_color: UniformLocation,
}

impl AmbientLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;
        uniform vec4 ambientColor;

        out vec4 FragColor;
        in vec2 texCoord;

        void main()
        {
        	FragColor = ambientColor * texture(diffuseTexture, texCoord);
        }
        "#
        )?;

        let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;

        void main()
        {
        	texCoord = vertexTexCoord;
        	gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        }
        "#
        )?;

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
        let fragment_source = CString::new(r#"
            #version 330 core

            uniform sampler2D depthTexture;
            uniform sampler2D colorTexture;
            uniform sampler2D normalTexture;
            uniform sampler2D spotShadowTexture;
            uniform samplerCube pointShadowTexture;

            uniform mat4 lightViewProjMatrix;
            uniform vec3 lightPos;
            uniform float lightRadius;
            uniform vec4 lightColor;
            uniform vec3 lightDirection;
            uniform float coneAngleCos;
            uniform mat4 invViewProj;
            uniform vec3 cameraPosition;
            uniform int lightType;
            uniform bool softShadows;
            uniform float shadowMapInvSize;

            in vec2 texCoord;
            out vec4 FragColor;

            vec3 GetProjection(vec3 worldPosition, mat4 viewProjectionMatrix)
            {
               vec4 projPos = viewProjectionMatrix * vec4(worldPosition, 1);
               projPos /= projPos.w;
               return vec3(projPos.x * 0.5 + 0.5, projPos.y * 0.5 + 0.5, projPos.z * 0.5 + 0.5);
            }

            void main()
            {
                vec4 normalSpecular = texture2D(normalTexture, texCoord);
                vec3 normal = normalize(normalSpecular.xyz * 2.0 - 1.0);

                vec4 screenPosition;
                screenPosition.x = texCoord.x * 2.0 - 1.0;
                screenPosition.y = texCoord.y * 2.0 - 1.0;
                screenPosition.z = texture2D(depthTexture, texCoord).r;
                screenPosition.w = 1.0;

                vec4 worldPosition = invViewProj * screenPosition;
                worldPosition /= worldPosition.w;

                vec3 lightVector = lightPos - worldPosition.xyz;
                float distanceToLight = length(lightVector);
                float d = min(distanceToLight, lightRadius);
                vec3 normLightVector = lightVector / d;
                vec3 h = normalize(lightVector + (cameraPosition - worldPosition.xyz));
                vec3 specular = normalSpecular.w * vec3(0.4 * pow(clamp(dot(normal, h), 0.0, 1.0), 80));
                float y = dot(lightDirection, normLightVector);
                float k = max(dot(normal, normLightVector), 0);
                float attenuation = 1.0 + cos((d / lightRadius) * 3.14159);
                if (y < coneAngleCos)
                {
                    attenuation *= smoothstep(coneAngleCos - 0.1, coneAngleCos, y);
                }

                float shadow = 1.0;
                if (lightType == 2) /* Spot light shadows */
                {
                  vec3 lightSpacePosition = GetProjection(worldPosition.xyz, lightViewProjMatrix);
                  const float bias = 0.00005;
                  if (softShadows)
                  {
                     for (float y = -1.5; y <= 1.5; y += 0.5)
                     {
                        for (float x = -1.5; x <= 1.5; x += 0.5)
                        {
                           vec2 fetchTexCoord = lightSpacePosition.xy + vec2(x, y) * shadowMapInvSize;
                           if (lightSpacePosition.z - bias > texture(spotShadowTexture, fetchTexCoord).r)
                           {
                              shadow += 1.0;
                           }
                        }
                     }

                     shadow = clamp(1.0 - shadow / 9.0, 0.0, 1.0);
                  }
                  else
                  {
                     if (lightSpacePosition.z - bias > texture(spotShadowTexture, lightSpacePosition.xy).r)
                     {
                        shadow = 0.0;
                     }
                  }
                }
                else if(lightType == 0) /* Point light shadows */
                {
                  const float bias = 0.01;
                  if (softShadows)
                  {
                     const int samples = 20;

                     const vec3 directions[samples] = vec3[samples] (
                        vec3(1, 1,  1), vec3( 1, -1,  1), vec3(-1, -1,  1), vec3(-1, 1,  1),
                        vec3(1, 1, -1), vec3( 1, -1, -1), vec3(-1, -1, -1), vec3(-1, 1, -1),
                        vec3(1, 1,  0), vec3( 1, -1,  0), vec3(-1, -1,  0), vec3(-1, 1,  0),
                        vec3(1, 0,  1), vec3(-1,  0,  1), vec3( 1,  0, -1), vec3(-1, 0, -1),
                        vec3(0, 1,  1), vec3( 0, -1,  1), vec3( 0, -1, -1), vec3( 0, 1, -1)
                     );

                     const float diskRadius = 0.0025;

                     for (int i = 0; i < samples; ++i)
                     {
                        vec3 fetchDirection = -normLightVector + directions[i] * diskRadius;
                        float shadowDistanceToLight = texture(pointShadowTexture, fetchDirection).r;
                        if (distanceToLight - bias > shadowDistanceToLight)
                        {
                           shadow += 1.0;
                        }
                     }

                     shadow = clamp(1.0 - shadow / float(samples), 0.0, 1.0);
                  }
                  else
                  {
                     float shadowDistanceToLight = texture(pointShadowTexture, -normLightVector).r;
                     if (distanceToLight - bias > shadowDistanceToLight)
                     {
                        shadow = 0.0;
                     }
                  }
               }

               FragColor = texture2D(colorTexture, texCoord);
               FragColor.xyz += specular;
               FragColor *= k * shadow * attenuation * lightColor;
            }
        "#)?;

        let vertex_source = CString::new(r#"
            #version 330 core

            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;

            uniform mat4 worldViewProjection;

            out vec2 texCoord;

            void main()
            {
                gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
                texCoord = vertexTexCoord;
            }
        "#)?;

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
}

impl DeferredLightRenderer {
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            shader: DeferredLightingShader::new()?,
            ambient_light_shader: AmbientLightShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
        })
    }

    pub fn render(&self, frame_width: f32, frame_height: f32, scene: &Scene, camera_node: &Node,
                  camera: &Camera, gbuffer: &GBuffer) {
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

            for light_node in scene.get_nodes().iter() {
                if !light_node.get_global_visibility() {
                    continue;
                }

                let light =
                    if let NodeKind::Light(light) = light_node.get_kind() {
                        light
                    } else {
                        continue;
                    };

                let light_position = light_node.get_global_position();
                let light_r_inflate = light.get_radius() * 1.05;
                let light_radius_vec = Vec3::make(light_r_inflate, light_r_inflate, light_r_inflate);
                let light_emit_direction = light_node.get_up_vector().normalized().unwrap();

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
                self.shader.set_light_type(-1); // Disable shadows for now
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