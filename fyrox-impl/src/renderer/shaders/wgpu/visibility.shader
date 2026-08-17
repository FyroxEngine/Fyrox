(
    name: "Visibility",
    resources: [
        (
            name: "matrices",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "tileBuffer",
            kind: Texture(kind: USampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "viewProjection", kind: Matrix4()),
                (name: "tileSize", kind: Int()),
                (name: "frameBufferHeight", kind: Float()),
            ]),
            binding: 0
        ),
    ],
    passes: [
        (
            name: "Primary",

            draw_parameters: DrawParameters(
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: false,
                stencil_test: None,
                depth_test: Some(LessOrEqual),
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: One,
                        dfactor: One,
                        alpha_sfactor: One,
                        alpha_dfactor: One,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
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
                        @location(0) @interpolate(flat) objectIndex: u32,
                    };

                    @vertex
                    fn vs_main(input: VertexInput, @builtin(instance_index) instance_index: u32) -> VertexOutput {
                        var output: VertexOutput;
                        output.objectIndex = instance_index;
                        output.position = (properties.viewProjection * S_FetchMatrix(matrices_tex, matrices_samp, i32(instance_index))) * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main(@location(0) @interpolate(flat) objectIndex: u32, @builtin(position) fragCoord: vec4f) -> @location(0) vec4f {
                        var x = i32(fragCoord.x) / properties.tileSize;
                        var y = i32(properties.frameBufferHeight - fragCoord.y) / properties.tileSize;

                        var bitIndex: i32 = -1;
                        var tileDataIndex = x * 33;
                        var count = i32(textureLoad(tileBuffer_tex, vec2i(tileDataIndex, y), 0).x);
                        var objectsListStartIndex = tileDataIndex + 1;
                        for (var i: i32 = 0; i < count; i++) {
                            var pixelObjectIndex = u32(textureLoad(tileBuffer_tex, vec2i(objectsListStartIndex + i, y), 0).x);
                            if (pixelObjectIndex == objectIndex) {
                                bitIndex = i;
                                break;
                            }
                        }

                        if (bitIndex < 0) {
                            return vec4f(0.0, 0.0, 0.0, 0.0);
                        } else {
                            var outMask = 1u << u32(bitIndex);
                            var r = f32(outMask & 255u) / 255.0;
                            var g = f32((outMask & 65280u) >> 8) / 255.0;
                            var b = f32((outMask & 16711680u) >> 16) / 255.0;
                            var a = f32((outMask & 4278190080u) >> 24) / 255.0;
                            return vec4f(r, g, b, a);
                        }
                    }
                "#,
        )
    ]
)
