(
    name: "Highlight",
    resources: [
        (
            name: "frameTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "color", kind: Vector4()),
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
                    struct VertexInput {
                        @location(0) vertexPosition: vec3f,
                        @location(1) vertexTexCoord: vec2f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) texCoord: vec2f,
                    };

                    @vertex
                    fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.texCoord = input.vertexTexCoord;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main(@location(0) texCoord: vec2f) -> @location(0) vec4f {
                        var size = textureDimensions(frameTexture_tex);

                        var w = 1.0 / f32(size.x);
                        var h = 1.0 / f32(size.y);

                        var n: array<f32, 9>;
                        n[0] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(-w, -h)).a;
                        n[1] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(0.0, -h)).a;
                        n[2] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(w, -h)).a;
                        n[3] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(-w, 0.0)).a;
                        n[4] = textureSample(frameTexture_tex, frameTexture_samp, texCoord).a;
                        n[5] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(w, 0.0)).a;
                        n[6] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(-w, h)).a;
                        n[7] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(0.0, h)).a;
                        n[8] = textureSample(frameTexture_tex, frameTexture_samp, texCoord + vec2f(w, h)).a;

                        var sobel_edge_h = n[2] + (2.0 * n[5]) + n[8] - (n[0] + (2.0 * n[3]) + n[6]);
                        var sobel_edge_v = n[0] + (2.0 * n[1]) + n[2] - (n[6] + (2.0 * n[7]) + n[8]);
                        var sobel = sqrt((sobel_edge_h * sobel_edge_h) + (sobel_edge_v * sobel_edge_v));

                        return vec4f(properties.color.rgb, properties.color.a * sobel);
                    }
                "#,
        )
    ]
)
