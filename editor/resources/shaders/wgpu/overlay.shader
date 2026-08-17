(
    name: "Overlay",
    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "viewProjectionMatrix", kind: Matrix4()),
                (name: "worldMatrix", kind: Matrix4()),
                (name: "cameraSideVector", kind: Vector3()),
                (name: "cameraUpVector", kind: Vector3()),
                (name: "size", kind: Float()),
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
                        var vertexOffset = input.vertexTexCoord * 2.0 - 1.0;
                        var worldPosition = properties.worldMatrix * vec4f(input.vertexPosition, 1.0);
                        var offset = (vertexOffset.x * properties.cameraSideVector + vertexOffset.y * properties.cameraUpVector) * properties.size;
                        output.position = properties.viewProjectionMatrix * (worldPosition + vec4f(offset.x, offset.y, offset.z, 0.0));
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main(@location(0) texCoord: vec2f) -> @location(0) vec4f {
                        return textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord);
                    }
                "#,
        )
    ]
)
