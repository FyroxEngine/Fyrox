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

                    out float outLum;

                    void main() {
                        float lum0 = texture(lumSampler, texCoord).r;
                        float lum1 = texture(lumSampler, texCoord + vec2(properties.invSize.x, 0.0)).r;
                        float lum2 = texture(lumSampler, texCoord + properties.invSize).r;
                        float lum3 = texture(lumSampler, texCoord + vec2(0.0, properties.invSize.y)).r;
                        outLum = (lum0 + lum1 + lum2 + lum3) / 4.0;
                    }
                "#,
        )
    ]
)