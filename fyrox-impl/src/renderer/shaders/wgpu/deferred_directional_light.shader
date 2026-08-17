(
    name: "DeferredDirectionalLight",
    resources: [
        (
            name: "depthTexture",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "colorTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "materialTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "shadowCascade0",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 4
        ),
        (
            name: "shadowCascade1",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 5
        ),
        (
            name: "shadowCascade2",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 6
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "viewMatrix", kind: Matrix4()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "lightViewProjMatrices", kind: Matrix4Array(max_len: 3, value: [])),
                (name: "lightColor", kind: Vector4()),
                (name: "lightDirection", kind: Vector3()),
                (name: "cameraPosition", kind: Vector3()),
                (name: "lightIntensity", kind: Float()),
                (name: "shadowsEnabled", kind: Bool()),
                (name: "shadowBias", kind: Float()),
                (name: "softShadows", kind: Bool()),
                (name: "cascadeDistances", kind: FloatArray(max_len: 3, value: [])),
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
                    zpass: Keep,
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
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        output.tex_coord = input.vertex_tex_coord;
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment fn fs_main(@location(0) tex_coord: vec2f) -> @location(0) vec4f {
                        let material = textureSample(materialTexture_tex, materialTexture_samp, tex_coord).rgb;

                        let fragment_position = S_UnProject(vec3f(tex_coord, textureSample(depthTexture_tex, depthTexture_samp, tex_coord)), properties.invViewProj);
                        let diffuse_color = textureSample(colorTexture_tex, colorTexture_samp, tex_coord);

                        var ctx: TPBRContext;
                        ctx.albedo = S_SRGBToLinear(diffuse_color).rgb;
                        ctx.fragmentToLight = properties.lightDirection;
                        ctx.fragmentNormal = normalize(textureSample(normalTexture_tex, normalTexture_samp, tex_coord).xyz * 2.0 - 1.0);
                        ctx.lightColor = properties.lightColor.rgb;
                        ctx.metallic = material.x;
                        ctx.roughness = material.y;
                        ctx.viewVector = normalize(properties.cameraPosition - fragment_position);

                        let lighting = S_PBR_CalculateLight(ctx);

                        let fragment_z_view_space = abs((properties.viewMatrix * vec4f(fragment_position, 1.0)).z);

                        var shadow: f32 = 1.0;
                        if (fragment_z_view_space <= properties.cascadeDistances[0].x) {
                            let inv_size = 1.0 / f32(textureDimensions(shadowCascade0_tex).x);
                            shadow = S_SpotShadowFactor_Depth(properties.shadowsEnabled != 0u, properties.softShadows != 0u,
                                properties.shadowBias, fragment_position, properties.lightViewProjMatrices[0], inv_size, shadowCascade0_tex, shadowCascade0_samp);
                        } else if (fragment_z_view_space <= properties.cascadeDistances[1].x) {
                            let inv_size = 1.0 / f32(textureDimensions(shadowCascade1_tex).x);
                            shadow = S_SpotShadowFactor_Depth(properties.shadowsEnabled != 0u, properties.softShadows != 0u,
                                properties.shadowBias, fragment_position, properties.lightViewProjMatrices[1], inv_size, shadowCascade1_tex, shadowCascade1_samp);
                        } else if (fragment_z_view_space <= properties.cascadeDistances[2].x) {
                            let inv_size = 1.0 / f32(textureDimensions(shadowCascade2_tex).x);
                            shadow = S_SpotShadowFactor_Depth(properties.shadowsEnabled != 0u, properties.softShadows != 0u,
                                properties.shadowBias, fragment_position, properties.lightViewProjMatrices[2], inv_size, shadowCascade2_tex, shadowCascade2_samp);
                        }

                        return shadow * vec4f(properties.lightIntensity * lighting, diffuse_color.a);
                    }
                "#,
        )
    ]
)