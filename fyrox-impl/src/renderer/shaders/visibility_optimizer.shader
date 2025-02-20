(
    name: "VisibilityOptimizer",
    resources: [
        (
            name: "visibilityBuffer",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "viewProjection", kind: Matrix4()),
                (name: "tileSize", kind: Int()),
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
                depth_test: Some(LessOrEqual),
                blend: None,
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Zero,
                    write_mask: 0xFFFF_FFFF,
                ),
                scissor_box: None
            ),

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;

                    void main()
                    {
                        gl_Position = properties.viewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out uint optimizedVisibilityMask;

                    void main()
                    {
                        int tileX = int(gl_FragCoord.x);
                        int tileY = int(gl_FragCoord.y);

                        int beginX = tileX * properties.tileSize;
                        int beginY = tileY * properties.tileSize;

                        int endX = (tileX + 1) * properties.tileSize;
                        int endY = (tileY + 1) * properties.tileSize;

                        int visibilityMask = 0;
                        for (int y = beginY; y < endY; ++y) {
                            for (int x = beginX; x < endX; ++x) {
                                ivec4 mask = ivec4(texelFetch(visibilityBuffer, ivec2(x, y), 0) * 255.0);
                                visibilityMask |= (mask.a << 24) | (mask.b << 16) | (mask.g << 8) | mask.r;
                            }
                        }
                        optimizedVisibilityMask = uint(visibilityMask);
                    }
                "#,
        )
    ]
)