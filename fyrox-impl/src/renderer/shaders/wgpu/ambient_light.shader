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
            kind: Texture(kind: DepthSampler2D, fallback: White),
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
                (name: "environmentLightingBrightness", kind: Float()),
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
                        let depth = textureSample(depthTexture_tex, depthTexture_samp, tex_coord);
                        let fragment_position = S_UnProject(vec3f(tex_coord, depth), properties.invViewProj);

                        let albedo = S_SRGBToLinear(textureSample(diffuseTexture_tex, diffuseTexture_samp, tex_coord));

                        let fragment_normal = normalize(textureSample(normalTexture_tex, normalTexture_samp, tex_coord).xyz * 2.0 - 1.0);

                        let material = textureSample(materialTexture_tex, materialTexture_samp, tex_coord).rgb;
                        let metallic = material.x;
                        let roughness = material.y;
                        let material_ao = material.z;

                        let view_vector = normalize(properties.cameraPosition - fragment_position);
                        let reflection_vector = -reflect(view_vector, fragment_normal);

                        let clamped_cos_view_angle = max(dot(fragment_normal, view_vector), 0.0);

                        let cube_map_size = textureDimensions(prefilteredSpecularMap_tex, 0);
                        let mip = roughness * (floor(log2(f32(cube_map_size.x))) + 1.0);

                        var reflection: vec3f;
                        if (properties.skyboxLighting != 0u) {
                            reflection = S_SRGBToLinear(textureSampleLevel(prefilteredSpecularMap_tex, prefilteredSpecularMap_samp, reflection_vector, mip)).rgb;
                        } else {
                            reflection = properties.ambientColor.rgb;
                        }

                        let F0 = mix(vec3f(0.04), albedo.rgb, metallic);
                        let F = S_FresnelSchlickRoughness(clamped_cos_view_angle, F0, roughness);
                        let kD = (vec3f(1.0) - F) * (1.0 - metallic);

                        let envBRDF = textureSample(brdfLUT_tex, brdfLUT_samp, vec2f(clamped_cos_view_angle, roughness)).rg;
                        let specular = reflection * (F * envBRDF.x + envBRDF.y);

                        let ambient_occlusion = textureSample(aoSampler_tex, aoSampler_samp, tex_coord).r * material_ao;
                        let baked_lighting = textureSample(bakedLightingTexture_tex, bakedLightingTexture_samp, tex_coord);

                        let irradiance = S_SRGBToLinear(textureSample(irradianceMap_tex, irradianceMap_samp, fragment_normal)).rgb;

                        var ambient_lighting: vec3f;
                        if (properties.skyboxLighting != 0u) {
                            ambient_lighting = irradiance;
                        } else {
                            ambient_lighting = properties.ambientColor.rgb;
                        }
                        let diffuse = (baked_lighting.rgb + properties.environmentLightingBrightness * ambient_lighting) * albedo.rgb;

                        let output_rgb = (kD * diffuse + specular) * ambient_occlusion;
                        return vec4f(output_rgb, baked_lighting.a);
                    }
                "#,
        )
    ]
)