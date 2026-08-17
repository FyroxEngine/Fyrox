(
    name: "DeferredSpotLight",
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
            name: "spotShadowTexture",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 4
        ),
        (
            name: "cookieTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 5
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "lightViewProjMatrix", kind: Matrix4()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "lightPos", kind: Vector3()),
                (name: "lightColor", kind: Vector4()),
                (name: "cameraPosition", kind: Vector3()),
                (name: "lightDirection", kind: Vector3()),
                (name: "lightRadius", kind: Float()),
                (name: "halfHotspotConeAngleCos", kind: Float()),
                (name: "halfConeAngleCos", kind: Float()),
                (name: "shadowMapInvSize", kind: Float()),
                (name: "shadowBias", kind: Float()),
                (name: "lightIntensity", kind: Float()),
                (name: "shadowAlpha", kind: Float()),
                (name: "cookieEnabled", kind: Bool()),
                (name: "shadowsEnabled", kind: Bool()),
                (name: "softShadows", kind: Bool()),
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
                    func: NotEqual,
                    ref_value: 0,
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
                        let fragment_to_light = properties.lightPos - fragment_position;
                        let dist = length(fragment_to_light);
                        let diffuse_color = textureSample(colorTexture_tex, colorTexture_samp, tex_coord);

                        var ctx: TPBRContext;
                        ctx.albedo = S_SRGBToLinear(diffuse_color).rgb;
                        ctx.fragmentToLight = fragment_to_light / dist;
                        ctx.fragmentNormal = normalize(textureSample(normalTexture_tex, normalTexture_samp, tex_coord).xyz * 2.0 - 1.0);
                        ctx.lightColor = properties.lightColor.rgb;
                        ctx.metallic = material.x;
                        ctx.roughness = material.y;
                        ctx.viewVector = normalize(properties.cameraPosition - fragment_position);

                        let lighting = S_PBR_CalculateLight(ctx);

                        let distance_attenuation = S_LightDistanceAttenuation(dist, properties.lightRadius);

                        let spot_angle_cos = dot(properties.lightDirection, ctx.fragmentToLight);
                        let cone_factor = smoothstep(properties.halfConeAngleCos, properties.halfHotspotConeAngleCos, spot_angle_cos);

                        let shadow = S_SpotShadowFactor_Depth(
                            properties.shadowsEnabled != 0u, properties.softShadows != 0u, properties.shadowBias, fragment_position,
                            properties.lightViewProjMatrix, properties.shadowMapInvSize, spotShadowTexture_tex, spotShadowTexture_samp);
                        let final_shadow = mix(1.0, shadow, properties.shadowAlpha);

                        var cookie_attenuation = vec4f(1.0);
                        if (properties.cookieEnabled != 0u) {
                            let cookie_tex_coords = S_Project(fragment_position, properties.lightViewProjMatrix).xy;
                            cookie_attenuation = textureSample(cookieTexture_tex, cookieTexture_samp, cookie_tex_coords);
                        }

                        return cookie_attenuation * vec4f(distance_attenuation * properties.lightIntensity * cone_factor * final_shadow * lighting, diffuse_color.a);
                    }
                "#,
        )
    ]
)