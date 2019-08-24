use glutin::ContextTrait;
use std::{
    ffi::{
        CString,
        c_void,
    },
    mem::size_of,
    time::{
        Instant,
        Duration,
    },
    thread,
};
use crate::{
    utils::pool::{
        Handle,
        Pool,
    },
    engine::{
        State,
        duration_to_seconds_f32,
    },
    resource::{
        ResourceKind,
        ttf::Font,
    },
    gui::draw::{
        DrawingContext,
        CommandKind,
        Color,
    },
    scene::node::{
        Node,
        NodeKind,
    },
    renderer::{
        surface::{
            Surface,
            Vertex,
        },
        gl,
        gl::types::*,
        gpu_program::{
            GpuProgram,
            UniformLocation,
        },
    },
    math::{
        vec3::Vec3,
        mat4::Mat4,
        vec2::Vec2,
    },
};

pub fn check_gl_error() {
    unsafe {
        match gl::GetError() {
            gl::NO_ERROR => (),
            _ => panic!("unknown opengl error!")
        }
    }
}

struct UIShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
}

struct FlatShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
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

struct GBufferShader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    wvp_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    bone_matrices: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
}

struct UIRenderBuffers {
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
}

struct GBuffer {
    fbo: GLuint,
    depth_rt: GLuint,
    depth_buffer: GLuint,
    depth_texture: GLuint,
    color_rt: GLuint,
    color_texture: GLuint,
    normal_rt: GLuint,
    normal_texture: GLuint,
    opt_fbo: GLuint,
    frame_texture: GLuint,
}

pub struct Statistics {
    pub frame_time: f32,
    pub mean_fps: usize,
    pub min_fps: usize,
    pub current_fps: usize,
    frame_time_accumulator: f32,
    frame_time_measurements: usize,
    time_last_fps_measured: f32,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            frame_time: 0.0,
            mean_fps: 0,
            min_fps: 0,
            current_fps: 0,
            frame_time_accumulator: 0.0,
            frame_time_measurements: 0,
            time_last_fps_measured: 0.0,
        }
    }
}

pub struct Renderer {
    pub(crate) events_loop: glutin::EventsLoop,
    pub(crate) context: glutin::WindowedContext,
    flat_shader: FlatShader,
    ui_shader: UIShader,
    deferred_light_shader: DeferredLightingShader,
    gbuffer_shader: GBufferShader,
    gbuffer: GBuffer,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    white_dummy: GLuint,
    /// Separate lists of handles to nodes of specified kinds. Used reduce tree traversal
    /// count, it will performed once. Lists are valid while there is scene to render.
    cameras: Vec<Handle<Node>>,
    lights: Vec<Handle<Node>>,
    meshes: Vec<Handle<Node>>,
    /// Scene graph traversal stack.
    traversal_stack: Vec<Handle<Node>>,
    frame_rate_limit: usize,
    ui_render_buffers: UIRenderBuffers,
    statistics: Statistics,
}

fn create_flat_shader() -> FlatShader {
    let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;

        in vec2 texCoord;

        void main()
        {
            FragColor = texture(diffuseTexture, texCoord);
        }
        "#).unwrap();

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
        "#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();
    FlatShader {
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        diffuse_texture: program.get_uniform_location("diffuseTexture"),
        program,
    }
}

fn create_ui_shader() -> UIShader {
    let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;
        in vec2 texCoord;
        in vec4 color;

        void main()
        {
            FragColor = color;
            FragColor.a *= texture(diffuseTexture, texCoord).r;
        };"#).unwrap();


    let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;
        layout(location = 2) in vec4 vertexColor;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;
        out vec4 color;

        void main()
        {
            texCoord = vertexTexCoord;
            color = vertexColor;
            gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        };"#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();
    UIShader {
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        diffuse_texture: program.get_uniform_location("diffuseTexture"),
        program,
    }
}

fn create_g_buffer_shader() -> GBufferShader {
    let fragment_source = CString::new(r#"
        #version 330 core

        layout(location = 0) out float outDepth;
        layout(location = 1) out vec4 outColor;
        layout(location = 2) out vec4 outNormal;
    
        uniform sampler2D diffuseTexture;
        uniform sampler2D normalTexture;
        uniform sampler2D specularTexture;
    
        in vec4 position;
        in vec3 normal;
        in vec2 texCoord;
        in vec3 tangent;
        in vec3 binormal;
    
        void main()
        {
        	outDepth = position.z / position.w;
        	outColor = texture2D(diffuseTexture, texCoord);
           if(outColor.a < 0.5) discard;
        	outColor.a = 1;
           vec4 n = normalize(texture2D(normalTexture, texCoord) * 2.0 - 1.0);
           mat3 tangentSpace = mat3(tangent, binormal, normal);
        	outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
           outNormal.w = texture2D(specularTexture, texCoord).r;
        }
    "#).unwrap();

    let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;
        layout(location = 2) in vec3 vertexNormal;
        layout(location = 3) in vec4 vertexTangent;
        layout(location = 4) in vec4 boneWeights;
        layout(location = 5) in vec4 boneIndices;
    
        uniform mat4 worldMatrix;
        uniform mat4 worldViewProjection;
        uniform bool useSkeletalAnimation;
        uniform mat4 boneMatrices[60];
    
        out vec4 position;
        out vec3 normal;
        out vec2 texCoord;
        out vec3 tangent;
        out vec3 binormal;
    
        void main()
        {
           vec4 localPosition = vec4(0);
           vec3 localNormal = vec3(0);
           vec3 localTangent = vec3(0);
           if(useSkeletalAnimation)
           {
               vec4 vertex = vec4(vertexPosition, 1.0);
    
               int i0 = int(boneIndices.x);
               int i1 = int(boneIndices.y);
               int i2 = int(boneIndices.z);
               int i3 = int(boneIndices.w);
    
               localPosition += boneMatrices[i0] * vertex * boneWeights.x;
               localPosition += boneMatrices[i1] * vertex * boneWeights.y;
               localPosition += boneMatrices[i2] * vertex * boneWeights.z;
               localPosition += boneMatrices[i3] * vertex * boneWeights.w;
    
               localNormal += mat3(boneMatrices[i0]) * vertexNormal * boneWeights.x;
               localNormal += mat3(boneMatrices[i1]) * vertexNormal * boneWeights.y;
               localNormal += mat3(boneMatrices[i2]) * vertexNormal * boneWeights.z;
               localNormal += mat3(boneMatrices[i3]) * vertexNormal * boneWeights.w;
    
               localTangent += mat3(boneMatrices[i0]) * vertexTangent.xyz * boneWeights.x;
               localTangent += mat3(boneMatrices[i1]) * vertexTangent.xyz * boneWeights.y;
               localTangent += mat3(boneMatrices[i2]) * vertexTangent.xyz * boneWeights.z;
               localTangent += mat3(boneMatrices[i3]) * vertexTangent.xyz * boneWeights.w;
           }
           else
           {
               localPosition = vec4(vertexPosition, 1.0);
               localNormal = vertexNormal;
               localTangent = vertexTangent.xyz;
           }
           gl_Position = worldViewProjection * localPosition;
           normal = normalize(mat3(worldMatrix) * localNormal);
           tangent = normalize(mat3(worldMatrix) * localTangent);
           binormal = normalize(vertexTangent.w * cross(tangent, normal));
           texCoord = vertexTexCoord;
           position = gl_Position;
        }
    "#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();

    GBufferShader {
        world_matrix: program.get_uniform_location("worldMatrix"),
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation"),
        bone_matrices: program.get_uniform_location("boneMatrices"),
        diffuse_texture: program.get_uniform_location("diffuseTexture"),
        normal_texture: program.get_uniform_location("normalTexture"),
        program,
    }
}

fn create_deferred_lighting_shaders() -> DeferredLightingShader {
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
    "#).unwrap();

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
    "#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();

    DeferredLightingShader {
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        depth_sampler: program.get_uniform_location("depthTexture"),
        color_sampler: program.get_uniform_location("colorTexture"),
        normal_sampler: program.get_uniform_location("normalTexture"),
        spot_shadow_texture: program.get_uniform_location("spotShadowTexture"),
        point_shadow_texture: program.get_uniform_location("pointShadowTexture"),
        light_view_proj_matrix: program.get_uniform_location("lightViewProjMatrix"),
        light_type: program.get_uniform_location("lightType"),
        soft_shadows: program.get_uniform_location("softShadows"),
        shadow_map_inv_size: program.get_uniform_location("shadowMapInvSize"),
        light_position: program.get_uniform_location("lightPos"),
        light_radius: program.get_uniform_location("lightRadius"),
        light_color: program.get_uniform_location("lightColor"),
        light_direction: program.get_uniform_location("lightDirection"),
        light_cone_angle_cos: program.get_uniform_location("coneAngleCos"),
        inv_view_proj_matrix: program.get_uniform_location("invViewProj"),
        camera_position: program.get_uniform_location("cameraPosition"),
        program,
    }
}

fn create_ui_render_buffers() -> UIRenderBuffers {
    unsafe {
        let mut ui_render_buffers = UIRenderBuffers {
            vbo: 0,
            ebo: 0,
            vao: 0,
        };

        gl::GenVertexArrays(1, &mut ui_render_buffers.vao);
        gl::GenBuffers(1, &mut ui_render_buffers.vbo);
        gl::GenBuffers(1, &mut ui_render_buffers.ebo);

        ui_render_buffers
    }
}


fn create_gbuffer(width: i32, height: i32) -> GBuffer
{
    unsafe {
        let mut fbo = 0;
        gl::GenFramebuffers(1, &mut fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

        let buffers = [
            gl::COLOR_ATTACHMENT0,
            gl::COLOR_ATTACHMENT1,
            gl::COLOR_ATTACHMENT2
        ];
        gl::DrawBuffers(3, buffers.as_ptr());

        let mut depth_rt = 0;
        gl::GenRenderbuffers(1, &mut depth_rt);
        gl::BindRenderbuffer(gl::RENDERBUFFER, depth_rt);
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::R32F, width, height);
        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, depth_rt);

        let mut color_rt = 0;
        gl::GenRenderbuffers(1, &mut color_rt);
        gl::BindRenderbuffer(gl::RENDERBUFFER, color_rt);
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA8, width, height);
        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT1, gl::RENDERBUFFER, color_rt);

        let mut normal_rt= 0;
        gl::GenRenderbuffers(1, &mut normal_rt);
        gl::BindRenderbuffer(gl::RENDERBUFFER, normal_rt);
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA8, width, height);
        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT2, gl::RENDERBUFFER, normal_rt);

        let mut depth_buffer = 0;
        gl::GenRenderbuffers(1, &mut depth_buffer);
        gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer);
        gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);
        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, depth_buffer);

        let mut depth_texture = 0;
        gl::GenTextures(1, &mut depth_texture);
        gl::BindTexture(gl::TEXTURE_2D, depth_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::R32F as i32, width, height, 0, gl::BGRA, gl::FLOAT, std::ptr::null());

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, depth_texture, 0);

        let mut color_texture = 0;
        gl::GenTextures(1, &mut color_texture);
        gl::BindTexture(gl::TEXTURE_2D, color_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT1, gl::TEXTURE_2D, color_texture, 0);

        let mut normal_texture = 0;
        gl::GenTextures(1, &mut normal_texture);
        gl::BindTexture(gl::TEXTURE_2D, normal_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT2, gl::TEXTURE_2D, normal_texture, 0);

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("Unable to construct G-Buffer FBO.");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        /* Create another framebuffer for stencil optimizations */
        let mut opt_fbo = 0;
        gl::GenFramebuffers(1, &mut opt_fbo);
        gl::BindFramebuffer(gl::FRAMEBUFFER, opt_fbo);

        let light_buffers = [gl::COLOR_ATTACHMENT0];
        gl::DrawBuffers(1, light_buffers.as_ptr());

        let mut frame_texture = 0;
        gl::GenTextures(1, &mut frame_texture);
        gl::BindTexture(gl::TEXTURE_2D, frame_texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, frame_texture, 0);

        gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, depth_buffer);

        if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("Unable to initialize Stencil FBO.");
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        GBuffer {
            fbo,
            depth_rt,
            depth_buffer,
            depth_texture,
            color_rt,
            color_texture,
            normal_rt,
            normal_texture,
            opt_fbo,
            frame_texture
        }
    }
}

fn create_white_dummy() -> GLuint {
    unsafe {
        let mut texture: GLuint = 0;
        let white_pixel: [Color; 1] = [Color { r: 255, g: 255, b: 255, a: 255 }; 1];
        gl::GenTextures(1, &mut texture);

        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            1,
            1,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            white_pixel.as_ptr() as *const c_void,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);

        texture
    }
}

impl Renderer {
    pub fn new() -> Self {
        let events_loop = glutin::EventsLoop::new();

        let primary_monitor = events_loop.get_primary_monitor();
        let mut monitor_dimensions = primary_monitor.get_dimensions();
        monitor_dimensions.height *= 0.7;
        monitor_dimensions.width *= 0.7;
        let window_size = monitor_dimensions.to_logical(primary_monitor.get_hidpi_factor());

        let window_builder = glutin::WindowBuilder::new()
            .with_title("RG3D")
            .with_dimensions(window_size)
            .with_resizable(false);

        let context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &events_loop)
            .unwrap();

        unsafe {
            context.make_current().unwrap();
            gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);
            gl::Enable(gl::DEPTH_TEST);
        }

        Self {
            context,
            events_loop,
            flat_shader: create_flat_shader(),
            ui_shader: create_ui_shader(),
            deferred_light_shader: create_deferred_lighting_shaders(),
            gbuffer_shader: create_g_buffer_shader(),
            gbuffer: create_gbuffer(window_size.width as i32, window_size.height as i32),
            traversal_stack: Vec::new(),
            cameras: Vec::new(),
            lights: Vec::new(),
            meshes: Vec::new(),
            frame_rate_limit: 60,
            statistics: Statistics::default(),
            white_dummy: create_white_dummy(),
            ui_render_buffers: create_ui_render_buffers(),
        }
    }

    pub fn get_statistics(&self) -> &Statistics {
        &self.statistics
    }

    fn draw_surface(&self, surf: &Surface) {
        unsafe {
            if let Some(resource) = surf.get_diffuse_texture() {
                if let ResourceKind::Texture(texture) = resource.borrow().borrow_kind() {
                    gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                }
            } else {
                gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
            }

            let data_rc = surf.get_data();
            let mut data = data_rc.borrow_mut();

            if data.need_upload {
                let total_size_bytes = data.get_vertices().len() * std::mem::size_of::<Vertex>();

                gl::BindVertexArray(data.get_vertex_array_object());

                // Upload indices
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, data.get_element_buffer_object());
                gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                               (data.get_indices().len() * std::mem::size_of::<i32>()) as GLsizeiptr,
                               data.get_indices().as_ptr() as *const GLvoid,
                               gl::STATIC_DRAW);

                // Upload vertices
                gl::BindBuffer(gl::ARRAY_BUFFER, data.get_vertex_buffer_object());
                gl::BufferData(gl::ARRAY_BUFFER,
                               total_size_bytes as GLsizeiptr,
                               data.get_vertices().as_ptr() as *const GLvoid,
                               gl::STATIC_DRAW);

                let mut offset = 0;

                // Positions
                gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(0);
                offset += size_of::<Vec3>();

                // Texture coordinates
                gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(1);
                offset += size_of::<Vec2>();

                // Normals
                gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(2);
                offset += size_of::<Vec3>();

                // Tangents
                gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE,
                                        size_of::<Vertex>() as GLint, offset as *const c_void);
                gl::EnableVertexAttribArray(3);

                gl::BindVertexArray(0);

                check_gl_error();

                data.need_upload = false;
            }

            gl::BindVertexArray(data.get_vertex_array_object());
            gl::DrawElements(gl::TRIANGLES,
                             data.get_indices().len() as GLint,
                             gl::UNSIGNED_INT,
                             std::ptr::null());
        }
    }

    pub fn upload_font_cache(&mut self, font_cache: &mut Pool<Font>) {
        unsafe {
            for font in font_cache.iter_mut() {
                if font.get_texture_id() == 0 {
                    let mut texture: GLuint = 0;
                    gl::GenTextures(1, &mut texture);

                    gl::BindTexture(gl::TEXTURE_2D, texture);

                    let rgba_pixels: Vec<Color> = font.get_atlas_pixels().
                        iter().map(|p| Color { r: *p, g: *p, b: *p, a: *p }).collect();

                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as i32,
                        font.get_atlas_size(),
                        font.get_atlas_size(),
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        rgba_pixels.as_ptr() as *const c_void,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::BindTexture(gl::TEXTURE_2D, 0);

                    println!("font cache loaded! {}", texture);

                    font.set_texture_id(texture);
                }
            }
        }

        check_gl_error();
    }

    pub fn upload_resources(&mut self, state: &mut State) {
        state.get_resource_manager_mut().for_each_texture_mut(|texture| {
            if texture.need_upload {
                unsafe {
                    if texture.gpu_tex == 0 {
                        gl::GenTextures(1, &mut texture.gpu_tex);
                    }
                    gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as i32,
                        texture.width as i32,
                        texture.height as i32,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        texture.pixels.as_ptr() as *const c_void,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::LINEAR_MIPMAP_LINEAR as i32,
                    );
                    gl::GenerateMipmap(gl::TEXTURE_2D);
                    texture.need_upload = false;
                }
            }
        });
    }

    fn render_ui(&mut self, drawing_context: &DrawingContext) {
        unsafe {
            let client_size = self.context.get_inner_size().unwrap();

            // Render UI on top of everything
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Disable(gl::CULL_FACE);

            self.ui_shader.program.bind();
            gl::ActiveTexture(gl::TEXTURE0);

            let index_bytes = drawing_context.get_indices_bytes();
            let vertex_bytes = drawing_context.get_vertices_bytes();

            // Upload to GPU.
            gl::BindVertexArray(self.ui_render_buffers.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.ui_render_buffers.vbo);
            gl::BufferData(gl::ARRAY_BUFFER, vertex_bytes, drawing_context.get_vertices_ptr(), gl::DYNAMIC_DRAW);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ui_render_buffers.ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, index_bytes, drawing_context.get_indices_ptr(), gl::DYNAMIC_DRAW);

            let mut offset = 0;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(0);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(1);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(2, 4, gl::UNSIGNED_BYTE, gl::TRUE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(2);

            let ortho = Mat4::ortho(0.0,
                                    client_size.width as f32,
                                    client_size.height as f32,
                                    0.0,
                                    -1.0,
                                    1.0);
            self.ui_shader.program.set_mat4(self.ui_shader.wvp_matrix, &ortho);

            for cmd in drawing_context.get_commands() {
                let index_count = cmd.get_triangle_count() * 3;
                if cmd.get_nesting() != 0 {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }
                match cmd.get_kind() {
                    CommandKind::Clip => {
                        if cmd.get_nesting() == 1 {
                            gl::Clear(gl::STENCIL_BUFFER_BIT);
                        }
                        gl::StencilOp(gl::KEEP, gl::KEEP, gl::INCR);
                        // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting() - 1), 0xFF);
                        gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                        // Draw clipping geometry to stencil buffer
                        gl::StencilMask(0xFF);
                        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
                    }
                    CommandKind::Geometry => {
                        // Make sure to draw geometry only on clipping geometry with current nesting level
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting()), 0xFF);

                        if cmd.get_texture() != 0 {
                            gl::ActiveTexture(gl::TEXTURE0);
                            self.ui_shader.program.set_int(self.ui_shader.diffuse_texture, 0);
                            gl::BindTexture(gl::TEXTURE_2D, cmd.get_texture());
                        } else {
                            gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                        }

                        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                        // Do not draw geometry to stencil buffer
                        gl::StencilMask(0x00);
                    }
                }

                let index_offset_bytes = cmd.get_index_offset() * std::mem::size_of::<GLuint>();
                gl::DrawElements(gl::TRIANGLES, index_count as i32, gl::UNSIGNED_INT,
                                 index_offset_bytes as *const c_void);
            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::BindVertexArray(0);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::BLEND);
            gl::Enable(gl::DEPTH_TEST);
        }
    }

    pub fn render(&mut self, state: &State, drawing_context: &DrawingContext) {
        let frame_start_time = Instant::now();

        unsafe {
            let client_size = self.context.get_inner_size().unwrap();

            gl::ClearColor(0.0, 0.63, 0.91, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            for scene in state.get_scenes().iter() {
                // Prepare for render - fill lists of nodes participating in rendering
                // by traversing scene graph
                self.meshes.clear();
                self.lights.clear();
                self.cameras.clear();
                self.traversal_stack.clear();
                self.traversal_stack.push(scene.get_root().clone());
                while let Some(node_handle) = self.traversal_stack.pop() {
                    if let Some(node) = scene.get_node(&node_handle) {
                        match node.borrow_kind() {
                            NodeKind::Mesh(_) => self.meshes.push(node_handle),
                            NodeKind::Light(_) => self.lights.push(node_handle),
                            NodeKind::Camera(_) => self.cameras.push(node_handle),
                            _ => ()
                        }
                        // Queue children for render
                        for child_handle in node.get_children() {
                            self.traversal_stack.push(child_handle.clone());
                        }
                    }
                }

                self.flat_shader.program.bind();

                // Render scene from each camera
                for camera_handle in self.cameras.iter() {
                    if let Some(camera_node) = scene.get_node(&camera_handle) {
                        if let NodeKind::Camera(camera) = camera_node.borrow_kind() {
                            // Setup viewport
                            let viewport = camera.get_viewport_pixels(
                                Vec2 {
                                    x: client_size.width as f32,
                                    y: client_size.height as f32,
                                });
                            gl::Viewport(viewport.x, viewport.y, viewport.w, viewport.h);

                            let view_projection = camera.get_view_projection_matrix();

                            for mesh_handle in self.meshes.iter() {
                                if let Some(node) = scene.get_node(&mesh_handle) {
                                    if !node.get_global_visibility() {
                                        continue;
                                    }

                                    let mvp = view_projection * *node.get_global_transform();

                                    self.flat_shader.program.bind();
                                    self.flat_shader.program.set_mat4(self.flat_shader.wvp_matrix, &mvp);

                                    if let NodeKind::Mesh(mesh) = node.borrow_kind() {
                                        for surface in mesh.get_surfaces().iter() {
                                            self.draw_surface(surface);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            self.render_ui(drawing_context);
        }

        check_gl_error();

        self.context.swap_buffers().unwrap();

        if self.frame_rate_limit > 0 {
            let frame_time_ms = 1000.0 * duration_to_seconds_f32(Instant::now().duration_since(frame_start_time));
            let desired_frame_time_ms = 1000.0 / self.frame_rate_limit as f32;
            if frame_time_ms < desired_frame_time_ms {
                let sleep_time_us = 1000.0 * (desired_frame_time_ms - frame_time_ms);
                thread::sleep(Duration::from_micros(sleep_time_us as u64));
            }
        }

        let total_time_s = duration_to_seconds_f32(Instant::now().duration_since(frame_start_time));
        self.statistics.frame_time = total_time_s;
        self.statistics.current_fps = (1.0 / total_time_s) as usize;
    }
}