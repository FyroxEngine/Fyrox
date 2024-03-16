(
    name: "StandardSpriteShader",

    properties: [
        (
            name: "diffuseTexture",
            kind: Sampler(default: None, fallback: White),
        ),
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
                layout(location = 2) in vec2 vertexParams;
                layout(location = 3) in vec4 vertexColor;

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
                    float size = vertexParams.x;
                    float rotation = vertexParams.y;

                    texCoord = vertexTexCoord;
                    color = vertexColor;
                    vec2 vertexOffset = rotateVec2(vertexTexCoord * 2.0 - 1.0, rotation);
                    vec4 worldPosition = fyrox_worldMatrix * vec4(vertexPosition, 1.0);
                    vec3 offset = (vertexOffset.x * fyrox_cameraSideVector + vertexOffset.y * fyrox_cameraUpVector) * size;
                    gl_Position = fyrox_viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
                }
               "#,

           fragment_shader:
               r#"
                uniform sampler2D diffuseTexture;

                out vec4 FragColor;

                in vec2 texCoord;
                in vec4 color;

                void main()
                {
                    FragColor = color * S_SRGBToLinear(texture(diffuseTexture, texCoord));
                }
               "#,
        )
    ],
)