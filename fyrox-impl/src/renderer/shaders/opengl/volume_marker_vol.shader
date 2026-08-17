(
    name: "VolumeMarkerVolume",
    resources: [
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
                    red: false,
                    green: false,
                    blue: false,
                    alpha: false,
                ),
                depth_write: false,
                stencil_test: Some(StencilFunc (
                    func: Equal,
                    ref_value: 0xFF,
                     mask: 0xFFFF_FFFF
                )),
                depth_test: Some(Less),
                blend: None,
                stencil_op: StencilOp(
                    fail: Replace,
                    zfail: Keep,
                    zpass: Replace,
                    write_mask: 0xFFFF_FFFF,
                ),
                scissor_box: None
            ),

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;

                    void main()
                    {
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;

                    void main()
                    {
                        FragColor = vec4(1.0);
                    }
                "#,
        )
    ]
)