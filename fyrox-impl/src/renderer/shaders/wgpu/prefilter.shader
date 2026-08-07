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
                    struct VertexInput {
                        @location(0) vertex_position: vec3f,
                    }

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) local_pos: vec3f,
                    }

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.local_pos = input.vertex_position;
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    fn RadicalInverse_VdC(bits_in: u32) -> f32 {
                        var bits = bits_in;
                        bits = (bits << 16u) | (bits >> 16u);
                        bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
                        bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
                        bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
                        bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
                        return f32(bits) * 2.3283064e-10;
                    }

                    fn Hammersley(i: u32, N: u32) -> vec2f {
                        return vec2f(f32(i) / f32(N), RadicalInverse_VdC(i));
                    }

                    fn ImportanceSampleGGX(Xi: vec2f, N: vec3f, roughness: f32) -> vec3f {
                        let a = roughness * roughness;

                        let phi = 2.0 * PI * Xi.x;
                        let cosTheta = sqrt((1.0 - Xi.y) / (1.0 + (a * a - 1.0) * Xi.y));
                        let sinTheta = sqrt(1.0 - cosTheta * cosTheta);

                        var H: vec3f;
                        H.x = cos(phi) * sinTheta;
                        H.y = sin(phi) * sinTheta;
                        H.z = cosTheta;

                        let up        = select(vec3f(1.0, 0.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(N.z) < 0.999);
                        let tangent   = normalize(cross(up, N));
                        let bitangent = cross(N, tangent);

                        let sampleVec = tangent * H.x + bitangent * H.y + N * H.z;
                        return normalize(sampleVec);
                    }

                    @fragment fn fs_main(@location(0) local_pos: vec3f) -> @location(0) vec4f {
                        let N = normalize(local_pos);
                        let R = N;
                        let V = R;

                        const SAMPLE_COUNT: u32 = 64u;
                        var totalWeight: f32 = 0.0;
                        var prefilteredColor = vec3f(0.0);
                        for (var i: u32 = 0u; i < SAMPLE_COUNT; i++)
                        {
                            let Xi = Hammersley(i, SAMPLE_COUNT);
                            let H  = ImportanceSampleGGX(Xi, N, properties.roughness);
                            let L  = normalize(2.0 * dot(V, H) * H - V);

                            let NdotL = max(dot(N, L), 0.0);
                            if (NdotL > 0.0)
                            {
                                prefilteredColor += textureSample(environmentMap_tex, environmentMap_samp, L).rgb * NdotL;
                                totalWeight += NdotL;
                            }
                        }
                        prefilteredColor = prefilteredColor / totalWeight;

                        return vec4f(prefilteredColor, 1.0);
                    }
                "#,
        )
    ]
)