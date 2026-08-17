(
    name: "SpotVolumetric",
    resources: [
        (
            name: "depthSampler",
            kind: Texture(kind: DepthSampler2D, fallback: White),
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
                    struct VertexInput {
                        @location(0) vertex_position: vec3f,
                        @location(1) vertex_tex_coord: vec2f,
                    }

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) tex_coord: vec2f,
                    }

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.tex_coord = input.vertex_tex_coord;
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment fn fs_main(@location(0) tex_coord: vec2f) -> @location(0) vec4f {
                        let fragmentPosition = S_UnProject(vec3f(tex_coord, textureSample(depthSampler_tex, depthSampler_samp, tex_coord)), properties.invProj);
                        let fragmentDepth = length(fragmentPosition);
                        let viewDirection = fragmentPosition / fragmentDepth;

                        // Ray-cone intersection
                        let sqrConeAngleCos = properties.coneAngleCos * properties.coneAngleCos;
                        let CO = -properties.lightPosition;
                        let DdotV = dot(viewDirection, properties.lightDirection);
                        let COdotV = dot(CO, properties.lightDirection);
                        let a = DdotV * DdotV - sqrConeAngleCos;
                        let b = 2.0 * (DdotV * COdotV - dot(viewDirection, CO) * sqrConeAngleCos);
                        let c = COdotV * COdotV - dot(CO, CO) * sqrConeAngleCos;

                        // Find intersection
                        var scatter = vec3f(0.0);
                        var minDepth: f32;
                        var maxDepth: f32;
                        if (S_SolveQuadraticEq(a, b, c, &minDepth, &maxDepth))
                        {
                            let dt1 = dot((minDepth * viewDirection) - properties.lightPosition, properties.lightDirection);
                            let dt2 = dot((maxDepth * viewDirection) - properties.lightPosition, properties.lightDirection);

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

                        return vec4f(properties.lightColor.xyz * pow(clamp(properties.intensity * scatter, vec3f(0.0), vec3f(1.0)), vec3f(2.2)), 1.0);
                    }
                "#,
        )
    ]
)