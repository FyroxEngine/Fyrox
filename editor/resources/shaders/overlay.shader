(
    name: "Overlay",
    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "viewProjectionMatrix", kind: Matrix4()),
                (name: "worldMatrix", kind: Matrix4()),
                (name: "cameraSideVector", kind: Vector3()),
                (name: "cameraUpVector", kind: Vector3()),
                (name: "size", kind: Float()),
            ]),
            binding: 0
        ),
    ],
    passes: [
        (
            name: "Primary",

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
                depth_test: None,
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
                scissor_box: None
            ),

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;
                    layout (location = 1) in vec2 vertexTexCoord;

                    out vec2 texCoord;

                    void main()
                    {
                        texCoord = vertexTexCoord;
                        vec2 vertexOffset = vertexTexCoord * 2.0 - 1.0;
                        vec4 worldPosition = properties.worldMatrix * vec4(vertexPosition, 1.0);
                        vec3 offset = (vertexOffset.x * properties.cameraSideVector + vertexOffset.y * properties.cameraUpVector) * properties.size;
                        gl_Position = properties.viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;

                    in vec2 texCoord;

                    void main()
                    {
                        FragColor = texture(diffuseTexture, texCoord);
                    }
                "#,
        )
    ]
)