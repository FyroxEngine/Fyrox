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
                    struct VertexInput {
                        @location(0) vertexPosition: vec3f,
                        @location(1) vertexTexCoord: vec2f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) texCoord: vec2f,
                    };

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.texCoord = input.vertexTexCoord;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment fn fs_main(@location(0) texCoord: vec2f) -> @location(0) vec4f {
                        const weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

                        let center = textureSample(image_tex, image_samp, texCoord);

                        var result = center.rgb * weights[0];

                        if (properties.horizontal != 0u) {
                            for (var i: i32 = 1; i < 5; i++) {
                                let fi = f32(i);

                                result += textureSample(image_tex, image_samp, texCoord + vec2f(properties.pixelSize.x * fi, 0.0)).rgb * weights[i];
                                result += textureSample(image_tex, image_samp, texCoord - vec2f(properties.pixelSize.x * fi, 0.0)).rgb * weights[i];
                            }
                        } else {
                            for (var i: i32 = 1; i < 5; i++) {
                                let fi = f32(i);

                                result += textureSample(image_tex, image_samp, texCoord + vec2f(0.0, properties.pixelSize.y * fi)).rgb * weights[i];
                                result += textureSample(image_tex, image_samp, texCoord - vec2f(0.0, properties.pixelSize.y * fi)).rgb * weights[i];
                            }
                        }

                        return vec4f(result, center.a);
                    }
                "#,
        )
    ]
)
