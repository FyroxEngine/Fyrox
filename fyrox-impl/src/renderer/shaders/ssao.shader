(
    name: "SSAO",
    resources: [
        (
            name: "depthSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "normalSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "noiseSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "inverseProjectionMatrix", kind: Matrix4()),
                (name: "projectionMatrix", kind: Matrix4()),
                (name: "kernel", kind: Vector3Array(max_len: 32, value: [])),
                (name: "noiseScale", kind: Vector2()),
                (name: "viewMatrix", kind: Matrix3()),
                (name: "radius", kind: Float()),
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
                    out float finalOcclusion;

                    in vec2 texCoord;

                    vec3 GetViewSpacePosition(vec2 screenCoord) {
                        return S_UnProject(vec3(screenCoord, texture(depthSampler, screenCoord).r), properties.inverseProjectionMatrix);
                    }

                    void main() {
                        vec3 fragPos = GetViewSpacePosition(texCoord);
                        vec3 worldSpaceNormal = texture(normalSampler, texCoord).xyz * 2.0 - 1.0;
                        vec3 viewSpaceNormal = normalize(properties.viewMatrix * worldSpaceNormal);
                        vec3 randomVec = normalize(texture(noiseSampler, texCoord * properties.noiseScale).xyz * 2.0 - 1.0);

                        vec3 tangent = normalize(randomVec - viewSpaceNormal * dot(randomVec, viewSpaceNormal));
                        vec3 bitangent = normalize(cross(viewSpaceNormal, tangent));
                        mat3 TBN = mat3(tangent, bitangent, viewSpaceNormal);

                        float occlusion = 0.0;
                        const int kernelSize = 32;
                        for (int i = 0; i < kernelSize; ++i) {
                            vec3 samplePoint = fragPos.xyz + TBN * properties.kernel[i] * properties.radius;

                            vec4 offset = properties.projectionMatrix * vec4(samplePoint, 1.0);
                            offset.xy /= offset.w;
                            offset.xy = offset.xy * 0.5 + 0.5;

                            vec3 position = GetViewSpacePosition(offset.xy);

                            float rangeCheck = smoothstep(0.0, 1.0, properties.radius / abs(fragPos.z - position.z));
                            occlusion += rangeCheck * ((position.z > samplePoint.z + 0.04) ? 1.0 : 0.0);
                        }

                        finalOcclusion = 1.0 - occlusion / float(kernelSize);
                    }
                "#,
        )
    ]
)