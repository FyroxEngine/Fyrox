(
    name: "Visibility",
    resources: [
        (
            name: "matrices",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "tileBuffer",
            kind: Texture(kind: USampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "viewProjection", kind: Matrix4()),
                (name: "tileSize", kind: Int()),
                (name: "frameBufferHeight", kind: Float()),
            ]),
            binding: 0
        ),
    ],
    passes: [
        (
            name: "Primary",

            draw_parameters: DrawParameters(
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: false,
                stencil_test: None,
                depth_test: Some(LessOrEqual),
                blend: Some(BlendParameters(
                    func: BlendFunc(
                        sfactor: One,
                        dfactor: One,
                        alpha_sfactor: One,
                        alpha_dfactor: One,
                    ),
                    equation: BlendEquation(
                        rgb: Add,
                        alpha: Add
                    )
                )),
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

                    flat out uint objectIndex;

                    void main()
                    {
                        objectIndex = uint(gl_InstanceID);
                        gl_Position = (properties.viewProjection * S_FetchMatrix(matrices, gl_InstanceID)) * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;

                    flat in uint objectIndex;

                    void main()
                    {
                        int x = int(gl_FragCoord.x) / properties.tileSize;
                        int y = int(properties.frameBufferHeight - gl_FragCoord.y) / properties.tileSize;

                        int bitIndex = -1;
                        int tileDataIndex = x * 33;
                        int count = int(texelFetch(tileBuffer, ivec2(tileDataIndex, y), 0).x);
                        int objectsListStartIndex = tileDataIndex + 1;
                        for (int i = 0; i < count; ++i) {
                            uint pixelObjectIndex = uint(texelFetch(tileBuffer, ivec2(objectsListStartIndex + i, y), 0).x);
                            if (pixelObjectIndex == objectIndex) {
                                bitIndex = i;
                                break;
                            }
                        }

                        if (bitIndex < 0) {
                            FragColor = vec4(0.0, 0.0, 0.0, 0.0);
                        } else {
                            uint outMask = uint(1 << bitIndex);
                            float r = float(outMask & 255u) / 255.0;
                            float g = float((outMask & 65280u) >> 8) / 255.0;
                            float b = float((outMask & 16711680u) >> 16) / 255.0;
                            float a = float((outMask & 4278190080u) >> 24) / 255.0;
                            FragColor = vec4(r, g, b, a);
                        }
                    }
                "#,
        )
    ]
)