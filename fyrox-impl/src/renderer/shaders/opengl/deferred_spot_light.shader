(
    name: "DeferredSpotLight",
    resources: [
        (
            name: "depthTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
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
            kind: Texture(kind: Sampler2D, fallback: White),
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
                    layout (location = 0) in vec3 vertexPosition;
                    layout (location = 1) in vec2 vertexTexCoord;

                    out vec2 texCoord;

                    void main()
                    {
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                        texCoord = vertexTexCoord;
                    }
                "#,

            fragment_shader:
                r#"
                    in vec2 texCoord;
                    out vec4 FragColor;

                    void main()
                    {
                        vec3 material = texture(materialTexture, texCoord).rgb;

                        vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), properties.invViewProj);
                        vec3 fragmentToLight = properties.lightPos - fragmentPosition;
                        float distance = length(fragmentToLight);
                        vec4 diffuseColor = texture(colorTexture, texCoord);

                        TPBRContext ctx;
                        ctx.albedo = S_SRGBToLinear(diffuseColor).rgb;
                        ctx.fragmentToLight = fragmentToLight / distance;
                        ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
                        ctx.lightColor = properties.lightColor.rgb;
                        ctx.metallic = material.x;
                        ctx.roughness = material.y;
                        ctx.viewVector = normalize(properties.cameraPosition - fragmentPosition);

                        vec3 lighting = S_PBR_CalculateLight(ctx);

                        float distanceAttenuation = S_LightDistanceAttenuation(distance, properties.lightRadius);

                        float spotAngleCos = dot(properties.lightDirection, ctx.fragmentToLight);
                        float coneFactor = smoothstep(properties.halfConeAngleCos, properties.halfHotspotConeAngleCos, spotAngleCos);

                        float shadow = S_SpotShadowFactor(
                            properties.shadowsEnabled, properties.softShadows, properties.shadowBias, fragmentPosition,
                            properties.lightViewProjMatrix, properties.shadowMapInvSize, spotShadowTexture);
                        float finalShadow = mix(1.0, shadow, properties.shadowAlpha);

                        vec4 cookieAttenuation = vec4(1.0);
                        if (properties.cookieEnabled) {
                            vec2 texCoords = S_Project(fragmentPosition, properties.lightViewProjMatrix).xy;
                            cookieAttenuation = texture(cookieTexture, texCoords);
                        }

                        FragColor = cookieAttenuation * vec4(distanceAttenuation * properties.lightIntensity * coneFactor * finalShadow * lighting, diffuseColor.a);
                    }
                "#,
        )
    ]
)