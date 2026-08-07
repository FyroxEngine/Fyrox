(
    name: "HdrDownscale",
    resources: [
        (
            name: "lumSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "invSize", kind: Vector2()),
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
                    @fragment fn fs_main(@location(0) texCoord: vec2f) -> @location(0) f32 {
                        let x = properties.invSize.x;
                        let y = properties.invSize.y;
                        let twoX = 2.0 * x;
                        let twoY = 2.0 * y;

                        let a = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x - twoX, texCoord.y + twoY)).r;
                        let b = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x, texCoord.y + twoY)).r;
                        let c = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x + twoX, texCoord.y + twoY)).r;

                        let d = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x - twoX, texCoord.y)).r;
                        let e = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x, texCoord.y)).r;
                        let f = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x + twoX, texCoord.y)).r;

                        let g = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x - twoX, texCoord.y - twoY)).r;
                        let h = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x, texCoord.y - twoY)).r;
                        let i = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x + twoX, texCoord.y - twoY)).r;

                        let j = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x - x, texCoord.y + y)).r;
                        let k = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x + x, texCoord.y + y)).r;
                        let l = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x - x, texCoord.y - y)).r;
                        let m = textureSample(lumSampler_tex, lumSampler_samp, vec2f(texCoord.x + x, texCoord.y - y)).r;

                        var outLum = e * 0.125;
                        outLum += (a + c + g + i) * 0.03125;
                        outLum += (b + d + f + h) * 0.0625;
                        outLum += (j + k + l + m) * 0.125;
                        return outLum;
                    }
                "#,
        )
    ]
)
