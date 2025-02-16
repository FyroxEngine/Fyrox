(
    name: "Blur",
    resources: [
        (
            name: "inputTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
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
                blend: None,
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
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    // Simple 4x4 box blur.
                    out float FragColor;

                    in vec2 texCoord;

                    void main()
                    {
                        vec2 texelSize = 1.0 / vec2(textureSize(inputTexture, 0));
                        float result = 0.0;
                        for (int y = -2; y < 2; ++y)
                        {
                            for (int x = -2; x < 2; ++x)
                            {
                                vec2 offset = vec2(float(x), float(y)) * texelSize;
                                result += texture(inputTexture, texCoord + offset).r;
                            }
                        }
                        FragColor = result / 16.0;
                    }
                "#,
        )
    ]
)