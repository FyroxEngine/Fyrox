(
    name: "AmbientLight",
    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "aoSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "bakedLightingTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "depthTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 4
        ),
        (
            name: "materialTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 5
        ),
        (
            name: "prefilteredSpecularMap",
            kind: Texture(kind: SamplerCube, fallback: White),
            binding: 6
        ),
        (
            name: "irradianceMap",
            kind: Texture(kind: SamplerCube, fallback: White),
            binding: 7
        ),
        (
            name: "brdfLUT",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 8
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "ambientColor", kind: Vector4()),
                (name: "cameraPosition", kind: Vector3()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "skyboxLighting", kind: Bool()),
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
                        float depth = texture(depthTexture, texCoord).r;
                        vec3 fragmentPosition = S_UnProject(vec3(texCoord, depth), properties.invViewProj);

                        vec4 albedo = S_SRGBToLinear(texture(diffuseTexture, texCoord));

                        vec3 fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);

                        vec3 material = texture(materialTexture, texCoord).rgb;
                        float metallic = material.x;
                        float roughness = material.y;
                        float materialAo = material.z;

                        vec3 viewVector = normalize(properties.cameraPosition - fragmentPosition);
                        vec3 reflectionVector = -reflect(viewVector, fragmentNormal);

                        float clampedCosViewAngle = max(dot(fragmentNormal, viewVector), 0.0);

                        ivec2 cubeMapSize = textureSize(prefilteredSpecularMap, 0);
                        float mip = roughness * (floor(log2(float(cubeMapSize.x))) + 1.0);
                        vec3 reflection = properties.skyboxLighting ? S_SRGBToLinear(textureLod(prefilteredSpecularMap, reflectionVector, mip)).rgb : properties.ambientColor.rgb;

                        vec3 F0 = mix(vec3(0.04), albedo.rgb, metallic);
                        vec3 F = S_FresnelSchlickRoughness(clampedCosViewAngle, F0, roughness);
                        vec3 kD = (vec3(1.0) - F) * (1.0 - metallic);

                        vec2 envBRDF = texture(brdfLUT, vec2(clampedCosViewAngle, roughness)).rg;
                        vec3 specular = reflection * (F * envBRDF.x + envBRDF.y);

                        float ambientOcclusion = texture(aoSampler, texCoord).r * materialAo;
                        vec4 bakedLighting = texture(bakedLightingTexture, texCoord);

                        vec3 irradiance = S_SRGBToLinear(texture(irradianceMap, fragmentNormal)).rgb;
                        vec3 diffuse = (properties.skyboxLighting ? irradiance : properties.ambientColor.rgb) * albedo.rgb;

                        FragColor.rgb = bakedLighting.rgb + (kD * diffuse + specular) * ambientOcclusion;
                        FragColor.a = bakedLighting.a;
                    }
                "#,
        )
    ]
)