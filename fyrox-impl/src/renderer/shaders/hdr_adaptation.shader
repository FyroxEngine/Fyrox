(
    name: "HdrAdaptation",
    resources: [
        (
            name: "oldLumSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "newLumSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "speed", kind: Float()),
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
                    zpass: Zero,
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
                    out float outLum;

                    void main() {
                        float oldLum = texture(oldLumSampler, vec2(0.5, 0.5)).r;
                        float newLum = texture(newLumSampler, vec2(0.5, 0.5)).r;
                        outLum = clamp(oldLum + (newLum - oldLum) * properties.speed, 0.0, newLum);
                    }
                "#,
        )
    ]
)