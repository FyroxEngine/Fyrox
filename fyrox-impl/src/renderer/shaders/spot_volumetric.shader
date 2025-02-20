(
    name: "SpotVolumetric",
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
                (name: "lightDirection", kind: Vector3()),
                (name: "lightColor", kind: Vector3()),
                (name: "scatterFactor", kind: Vector3()),
                (name: "intensity", kind: Float()),
                (name: "coneAngleCos", kind: Float()),
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

                        // Ray-cone intersection
                        float sqrConeAngleCos = properties.coneAngleCos * properties.coneAngleCos;
                        vec3 CO = -properties.lightPosition;
                        float DdotV = dot(viewDirection, properties.lightDirection);
                        float COdotV = dot(CO, properties.lightDirection);
                        float a = DdotV * DdotV - sqrConeAngleCos;
                        float b = 2.0 * (DdotV * COdotV - dot(viewDirection, CO) * sqrConeAngleCos);
                        float c = COdotV * COdotV - dot(CO, CO) * sqrConeAngleCos;

                        // Find intersection
                        vec3 scatter = vec3(0.0);
                        float minDepth, maxDepth;
                        if (S_SolveQuadraticEq(a, b, c, minDepth, maxDepth))
                        {
                            float dt1 = dot((minDepth * viewDirection) - properties.lightPosition, properties.lightDirection);
                            float dt2 = dot((maxDepth * viewDirection) - properties.lightPosition, properties.lightDirection);

                            // Discard points on reflected cylinder and perform depth test.
                            if ((dt1 > 0.0 || dt2 > 0.0) && (minDepth > 0.0 || fragmentDepth > minDepth))
                            {
                                if (dt1 > 0.0 && dt2 < 0.0)
                                {
                                    // Closest point is on cylinder, farthest on reflected.
                                    maxDepth = minDepth;
                                    minDepth = 0.0;
                                }
                                else if (dt1 < 0.0 && dt2 > 0.0)
                                {
                                    // Farthest point is on cylinder, closest on reflected.
                                    minDepth = maxDepth;
                                    maxDepth = fragmentDepth;
                                }

                                minDepth = max(minDepth, 0.0);
                                maxDepth = clamp(maxDepth, 0.0, fragmentDepth);

                                scatter = properties.scatterFactor * S_InScatter(viewDirection * minDepth, viewDirection, properties.lightPosition, maxDepth - minDepth);
                            }
                        }

                        FragColor = vec4(properties.lightColor.xyz * pow(clamp(properties.intensity * scatter, 0.0, 1.0), vec3(2.2)), 1.0);
                    }
                "#,
        )
    ]
)