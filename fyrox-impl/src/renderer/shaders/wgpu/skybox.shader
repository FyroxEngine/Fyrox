(
    name: "SkyBox",
    resources: [
        (
            name: "cubemapTexture",
            kind: Texture(kind: SamplerCube, fallback: White),
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
                    }

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) tex_coord: vec3f,
                    }

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.tex_coord = input.vertex_position;
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment fn fs_main(@location(0) tex_coord: vec3f) -> @location(0) vec4f {
                        return S_SRGBToLinear(textureSample(cubemapTexture_tex, cubemapTexture_samp, tex_coord));
                    }
                "#,
        )
    ]
)