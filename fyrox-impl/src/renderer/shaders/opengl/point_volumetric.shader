(
    name: "PointVolumetric",
    resources: [
        (
            name: "depthSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "invProj", kind: Matrix4()),
                (name: "lightPosition", kind: Vector3()),
                (name: "lightColor", kind: Vector3()),
                (name: "scatterFactor", kind: Vector3()),
                (name: "intensity", kind: Float()),
                (name: "lightRadius", kind: Float()),
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
                stencil_test: Some(StencilFunc(
                    func: Equal,
                    ref_value: 0xFF,
                    mask: 0xFFFF_FFFF
                )),
                depth_test: None,
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
                    out vec4 FragColor;

                    in vec2 texCoord;

                    void main()
                    {
                        vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthSampler, texCoord).r), properties.invProj);
                        float fragmentDepth = length(fragmentPosition);
                        vec3 viewDirection = fragmentPosition / fragmentDepth;

                        // Find intersection
                        vec3 scatter = vec3(0.0);
                        float minDepth, maxDepth;
                        if (S_RaySphereIntersection(vec3(0.0), viewDirection, properties.lightPosition, properties.lightRadius, minDepth, maxDepth))
                        {
                            // Perform depth test.
                            if (minDepth > 0.0 || fragmentDepth > minDepth)
                            {
                                minDepth = max(minDepth, 0.0);
                                maxDepth = clamp(maxDepth, 0.0, fragmentDepth);

                                vec3 closestPoint = viewDirection * minDepth;

                                scatter = properties.scatterFactor * S_InScatter(closestPoint, viewDirection, properties.lightPosition, maxDepth - minDepth);
                            }
                        }

                        FragColor = vec4(properties.lightColor.xyz * pow(clamp(properties.intensity * scatter, 0.0, 1.0), vec3(2.2)), 1.0);
                    }
                "#,
        )
    ]
)