(
    name: "StandardParticleSystemShader",

    properties: [
        (
            name: "diffuseTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "softBoundarySharpnessFactor",
            kind: Float(100.0),
        )
    ],

    passes: [
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: None,
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: false,
                stencil_test: None,
                depth_test: true,
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: SrcAlpha,
                        dfactor: OneMinusSrcAlpha,
                        alpha_sfactor: SrcAlpha,
                        alpha_dfactor: OneMinusSrcAlpha,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),
            vertex_shader:
               r#"
               layout(location = 0) in vec3 vertexPosition;
               layout(location = 1) in vec2 vertexTexCoord;
               layout(location = 2) in float particleSize;
               layout(location = 3) in float particleRotation;
               layout(location = 4) in vec4 vertexColor;

               uniform mat4 fyrox_viewProjectionMatrix;
               uniform mat4 fyrox_worldMatrix;
               uniform vec3 fyrox_cameraUpVector;
               uniform vec3 fyrox_cameraSideVector;

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
                   color = S_SRGBToLinear(vertexColor);
                   texCoord = vertexTexCoord;
                   vec2 vertexOffset = rotateVec2(vertexTexCoord * 2.0 - 1.0, particleRotation);
                   vec4 worldPosition = fyrox_worldMatrix * vec4(vertexPosition, 1.0);
                   vec3 offset = (vertexOffset.x * fyrox_cameraSideVector + vertexOffset.y * fyrox_cameraUpVector) * particleSize;
                   gl_Position = fyrox_viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
               }
               "#,

           fragment_shader:
               r#"
               uniform sampler2D diffuseTexture;
               uniform float softBoundarySharpnessFactor;

               uniform sampler2D fyrox_sceneDepth;
               uniform float fyrox_zNear;
               uniform float fyrox_zFar;

               out vec4 FragColor;
               in vec2 texCoord;
               in vec4 color;

               float toProjSpace(float z)
               {
                   return (fyrox_zFar * fyrox_zNear) / (fyrox_zFar - z * (fyrox_zFar - fyrox_zNear));
               }

               void main()
               {
                   ivec2 depthTextureSize = textureSize(fyrox_sceneDepth, 0);
                   vec2 pixelSize = vec2(1.0 / float(depthTextureSize.x), 1.0 / float(depthTextureSize.y));
                   float sceneDepth = toProjSpace(texture(fyrox_sceneDepth, gl_FragCoord.xy * pixelSize).r);
                   float fragmentDepth = toProjSpace(gl_FragCoord.z);
                   float depthOpacity = smoothstep((sceneDepth - fragmentDepth) * softBoundarySharpnessFactor, 0.0, 1.0);
                   FragColor = color * S_SRGBToLinear(texture(diffuseTexture, texCoord)).r;
                   FragColor.a *= depthOpacity;
               }
               "#,
        )
    ],
)