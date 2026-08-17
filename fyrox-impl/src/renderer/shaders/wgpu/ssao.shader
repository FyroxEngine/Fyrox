(
    name: "SSAO",
    resources: [
        (
            name: "depthSampler",
            kind: Texture(kind: DepthSampler2D, fallback: White),
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
                    fn GetViewSpacePosition(screenCoord: vec2f) -> vec3f {
                        return S_UnProject(vec3f(screenCoord, textureSample(depthSampler_tex, depthSampler_samp, screenCoord)), properties.inverseProjectionMatrix);
                    }

                    @fragment fn fs_main(@location(0) tex_coord: vec2f) -> @location(0) f32 {
                        let fragPos = GetViewSpacePosition(tex_coord);
                        let worldSpaceNormal = textureSample(normalSampler_tex, normalSampler_samp, tex_coord).xyz * 2.0 - 1.0;
                        let viewSpaceNormal = normalize(properties.viewMatrix * worldSpaceNormal);
                        let randomVec = normalize(textureSample(noiseSampler_tex, noiseSampler_samp, tex_coord * properties.noiseScale).xyz * 2.0 - 1.0);

                        let tangent = normalize(randomVec - viewSpaceNormal * dot(randomVec, viewSpaceNormal));
                        let bitangent = normalize(cross(viewSpaceNormal, tangent));
                        let TBN = mat3x3f(tangent, bitangent, viewSpaceNormal);

                        var occlusion: f32 = 0.0;
                        const kernelSize: i32 = 32;
                        for (var i: i32 = 0; i < kernelSize; i++) {
                            let samplePoint = fragPos + TBN * properties.kernel[i] * properties.radius;

                            let offset = properties.projectionMatrix * vec4f(samplePoint, 1.0);
                            let screenUv = (offset.xy / offset.w) * 0.5 + 0.5;

                            let position = GetViewSpacePosition(screenUv);

                            let rangeCheck = smoothstep(0.0, 1.0, properties.radius / abs(fragPos.z - position.z));
                            occlusion += rangeCheck * select(0.0, 1.0, position.z > samplePoint.z + 0.04);
                        }

                        return 1.0 - occlusion / f32(kernelSize);
                    }
                "#,
        )
    ]
)