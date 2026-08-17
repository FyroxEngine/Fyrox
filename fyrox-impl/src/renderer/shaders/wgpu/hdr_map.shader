(
    name: "HdrMap",
    resources: [
        (
            name: "hdrSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "lumSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "bloomSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "colorMapSampler",
            kind: Texture(kind: Sampler3D, fallback: White),
            binding: 3
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "useColorGrading", kind: Bool()),
                (name: "minLuminance", kind: Float()),
                (name: "maxLuminance", kind: Float()),
                (name: "autoExposure", kind: Bool()),
                (name: "fixedExposure", kind: Float()),
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
                    fn ColorGrading(color: vec3f) -> vec3f {
                        const lutSize: f32 = 16.0;
                        const a: f32 = (lutSize - 1.0) / lutSize;
                        const b: f32 = 1.0 / (2.0 * lutSize);
                        let scale = vec3f(a);
                        let offset = vec3f(b);
                        return textureSample(colorMapSampler_tex, colorMapSampler_samp, scale * color + offset).rgb;
                    }

                    // Narkowicz 2015, "ACES Filmic Tone Mapping Curve"
                    fn TonemapACES(x: f32) -> f32 {
                        const a: f32 = 2.51;
                        const b: f32 = 0.03;
                        const c: f32 = 2.43;
                        const d: f32 = 0.59;
                        const e: f32 = 0.14;
                        return (x * (a * x + b)) / (x * (c * x + d) + e);
                    }

                    @fragment fn fs_main(@location(0) texCoord: vec2f) -> @location(0) vec4f {
                        let hdrColor = textureSample(hdrSampler_tex, hdrSampler_samp, texCoord) + textureSample(bloomSampler_tex, bloomSampler_samp, texCoord);

                        var Yxy = S_ConvertRgbToYxy(hdrColor.rgb);

                        var lp: f32;
                        if (properties.autoExposure != 0u) {
                            let avgLum = textureSample(lumSampler_tex, lumSampler_samp, vec2f(0.5, 0.5)).r;
                            let clampedAvgLum = clamp(avgLum, properties.minLuminance, properties.maxLuminance);
                            lp = Yxy.x / (9.6 * clampedAvgLum + 0.0001);
                        } else {
                            lp = Yxy.x * properties.fixedExposure;
                        }

                        Yxy.x = TonemapACES(lp);

                        let ldrColor = vec4f(S_ConvertYxyToRgb(Yxy), hdrColor.a);

                        if (properties.useColorGrading != 0u) {
                            return vec4f(ColorGrading(S_LinearToSRGB(ldrColor).rgb), ldrColor.a);
                        } else {
                            return S_LinearToSRGB(ldrColor);
                        }
                    }
                "#,
        )
    ]
)
