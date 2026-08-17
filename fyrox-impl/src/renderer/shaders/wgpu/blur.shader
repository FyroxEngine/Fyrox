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
                    struct VertexInput {
                        @location(0) vertex_position: vec3f,
                        @location(1) vertex_tex_coord: vec2f,
                    }

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) tex_coord: vec2f,
                    }

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.tex_coord = input.vertex_tex_coord;
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    // Simple 4x4 box blur.

                    @fragment fn fs_main(@location(0) tex_coord: vec2f) -> @location(0) f32 {
                        let texel_size = 1.0 / vec2f(textureDimensions(inputTexture_tex, 0));
                        var result: f32 = 0.0;
                        for (var y: i32 = -2; y < 2; y++) {
                            for (var x: i32 = -2; x < 2; x++) {
                                let offset = vec2f(f32(x), f32(y)) * texel_size;
                                result += textureSample(inputTexture_tex, inputTexture_samp, tex_coord + offset).r;
                            }
                        }
                        return result / 16.0;
                    }
                "#,
        )
    ]
)