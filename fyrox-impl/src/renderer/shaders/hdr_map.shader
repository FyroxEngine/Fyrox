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
                (name: "keyValue", kind: Float()),
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

                    out vec4 outLdrColor;

                    vec3 ColorGrading(vec3 color) {
                        const float lutSize = 16.0;
                        const float a = (lutSize - 1.0) / lutSize;
                        const float b = 1.0 / (2.0 * lutSize);
                        vec3 scale = vec3(a);
                        vec3 offset = vec3(b);
                        return texture(colorMapSampler, scale * color + offset).rgb;
                    }

                    void main() {
                        vec4 hdrColor = texture(hdrSampler, texCoord);

                        hdrColor += texture(bloomSampler, texCoord);

                        float luminance = texture(lumSampler, vec2(0.5, 0.5)).r;

                        float exposure;
                        if (properties.autoExposure) {
                            exposure = properties.keyValue / clamp(luminance, properties.minLuminance, properties.maxLuminance);
                        } else {
                            exposure = properties.fixedExposure;
                        }

                        vec4 ldrColor = vec4(vec3(1.0) - exp(-hdrColor.rgb * exposure), hdrColor.a);

                        if (properties.useColorGrading) {
                            outLdrColor = vec4(ColorGrading(S_LinearToSRGB(ldrColor).rgb), ldrColor.a);
                        } else {
                            outLdrColor = S_LinearToSRGB(ldrColor);
                        }
                    }
                "#,
        )
    ]
)