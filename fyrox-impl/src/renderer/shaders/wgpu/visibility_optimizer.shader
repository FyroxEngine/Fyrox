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
                    struct VertexInput {
                        @location(0) vertexPosition: vec3f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                    };

                    @vertex
                    fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.position = properties.viewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main(@builtin(position) fragCoord: vec4f) -> @location(0) u32 {
                        var tileX = i32(fragCoord.x);
                        var tileY = i32(fragCoord.y);

                        var beginX = tileX * properties.tileSize;
                        var beginY = tileY * properties.tileSize;

                        var endX = (tileX + 1) * properties.tileSize;
                        var endY = (tileY + 1) * properties.tileSize;

                        var visibilityMask: i32 = 0;
                        for (var y: i32 = beginY; y < endY; y++) {
                            for (var x: i32 = beginX; x < endX; x++) {
                                var mask = vec4i(textureLoad(visibilityBuffer_tex, vec2i(x, y), 0) * 255.0);
                                visibilityMask |= (mask.w << 24) | (mask.z << 16) | (mask.y << 8) | mask.x;
                            }
                        }
                        return u32(visibilityMask);
                    }
                "#,
        )
    ]
)
