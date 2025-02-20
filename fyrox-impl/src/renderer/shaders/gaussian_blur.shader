(
    name: "GaussianBlur",
    resources: [
        (
            name: "image",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "pixelSize", kind: Vector2()),
                (name: "horizontal", kind: Bool()),
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
                    in vec2 texCoord;

                    out vec4 outColor;

                    void main()
                    {
                        const float weights[5] = float[](0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

                        vec4 center = texture(image, texCoord);

                        vec3 result = center.rgb * weights[0];

                        if (properties.horizontal) {
                            for (int i = 1; i < 5; ++i) {
                                float fi = float(i);

                                result += texture(image, texCoord + vec2(properties.pixelSize.x * fi, 0.0)).rgb * weights[i];
                                result += texture(image, texCoord - vec2(properties.pixelSize.x * fi, 0.0)).rgb * weights[i];
                            }
                        } else {
                            for (int i = 1; i < 5; ++i) {
                                float fi = float(i);

                                result += texture(image, texCoord + vec2(0.0, properties.pixelSize.y * fi)).rgb * weights[i];
                                result += texture(image, texCoord - vec2(0.0, properties.pixelSize.y * fi)).rgb * weights[i];
                            }
                        }

                        outColor = vec4(result, center.a);
                    }
                "#,
        )
    ]
)