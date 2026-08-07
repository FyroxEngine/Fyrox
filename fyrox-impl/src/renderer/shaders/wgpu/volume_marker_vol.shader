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
                    struct VertexInput {
                        @location(0) vertexPosition: vec3f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                    };

                    @vertex
                    fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main() -> @location(0) vec4f {
                        return vec4f(1.0);
                    }
                "#,
        )
    ]
)
