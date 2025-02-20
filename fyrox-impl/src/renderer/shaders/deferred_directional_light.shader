(
    name: "DeferredDirectionalLight",
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
            name: "shadowCascade0",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 4
        ),
        (
            name: "shadowCascade1",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 5
        ),
        (
            name: "shadowCascade2",
            kind: Texture(kind: Sampler2D, fallback: White),
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
                (name: "shadowMapInvSize", kind: Float()),
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

                    // Returns **inverted** shadow factor where 1 - fully bright, 0 - fully in shadow.
                    float CsmGetShadow(in sampler2D sampler, in vec3 fragmentPosition, in mat4 lightViewProjMatrix)
                    {
                        return S_SpotShadowFactor(properties.shadowsEnabled, properties.softShadows, properties.shadowBias, fragmentPosition, lightViewProjMatrix, properties.shadowMapInvSize, sampler);
                    }

                    void main()
                    {
                        vec3 material = texture(materialTexture, texCoord).rgb;

                        vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), properties.invViewProj);
                        vec4 diffuseColor = texture(colorTexture, texCoord);

                        TPBRContext ctx;
                        ctx.albedo = S_SRGBToLinear(diffuseColor).rgb;
                        ctx.fragmentToLight = properties.lightDirection;
                        ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
                        ctx.lightColor = properties.lightColor.rgb;
                        ctx.metallic = material.x;
                        ctx.roughness = material.y;
                        ctx.viewVector = normalize(properties.cameraPosition - fragmentPosition);

                        vec3 lighting = S_PBR_CalculateLight(ctx);

                        float fragmentZViewSpace = abs((properties.viewMatrix * vec4(fragmentPosition, 1.0)).z);

                        float shadow = 1.0;
                        if (fragmentZViewSpace <= properties.cascadeDistances[0]) {
                            shadow = CsmGetShadow(shadowCascade0, fragmentPosition, properties.lightViewProjMatrices[0]);
                        } else if (fragmentZViewSpace <= properties.cascadeDistances[1]) {
                            shadow = CsmGetShadow(shadowCascade1, fragmentPosition, properties.lightViewProjMatrices[1]);
                        } else if (fragmentZViewSpace <= properties.cascadeDistances[2]) {
                            shadow = CsmGetShadow(shadowCascade2, fragmentPosition, properties.lightViewProjMatrices[2]);
                        }

                        FragColor = shadow * vec4(properties.lightIntensity * lighting, diffuseColor.a);
                    }
                "#,
        )
    ]
)