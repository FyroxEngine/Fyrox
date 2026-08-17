(
    name: "ReflectionCubeMapPrefilter",
    resources: [
        (
            name: "environmentMap",
            kind: Texture(kind: SamplerCube, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "roughness", kind: Float()),
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
                blend: None,
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

                    out vec3 localPos;

                    void main()
                    {
                        localPos = vertexPosition;
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;

                    in vec3 localPos;

                    float RadicalInverse_VdC(uint bits)
                    {
                        bits = (bits << 16u) | (bits >> 16u);
                        bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
                        bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
                        bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
                        bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
                        return float(bits) * 2.3283064e-10; // / 0x100000000
                    }

                    vec2 Hammersley(uint i, uint N)
                    {
                        return vec2(float(i)/float(N), RadicalInverse_VdC(i));
                    }

                    vec3 ImportanceSampleGGX(vec2 Xi, vec3 N, float roughness)
                    {
                        float a = roughness*roughness;

                        float phi = 2.0 * PI * Xi.x;
                        float cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a*a - 1.0) * Xi.y));
                        float sinTheta = sqrt(1.0 - cosTheta*cosTheta);

                        vec3 H;
                        H.x = cos(phi) * sinTheta;
                        H.y = sin(phi) * sinTheta;
                        H.z = cosTheta;

                        vec3 up        = abs(N.z) < 0.999 ? vec3(0.0, 0.0, 1.0) : vec3(1.0, 0.0, 0.0);
                        vec3 tangent   = normalize(cross(up, N));
                        vec3 bitangent = cross(N, tangent);

                        vec3 sampleVec = tangent * H.x + bitangent * H.y + N * H.z;
                        return normalize(sampleVec);
                    }


                    void main()
                    {
                        vec3 N = normalize(localPos);
                        vec3 R = N;
                        vec3 V = R;

                        const uint SAMPLE_COUNT = 64u;
                        float totalWeight = 0.0;
                        vec3 prefilteredColor = vec3(0.0);
                        for(uint i = 0u; i < SAMPLE_COUNT; ++i)
                        {
                            vec2 Xi = Hammersley(i, SAMPLE_COUNT);
                            vec3 H  = ImportanceSampleGGX(Xi, N, properties.roughness);
                            vec3 L  = normalize(2.0 * dot(V, H) * H - V);

                            float NdotL = max(dot(N, L), 0.0);
                            if(NdotL > 0.0)
                            {
                                prefilteredColor += texture(environmentMap, L).rgb * NdotL;
                                totalWeight += NdotL;
                            }
                        }
                        prefilteredColor = prefilteredColor / totalWeight;

                        FragColor = vec4(prefilteredColor, 1.0);
                    }
                "#,
        )
    ]
)