(
    name: "IrradianceShader",
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
                        @location(0) vertexPosition: vec3f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) localPos: vec3f,
                    };

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.localPos = input.vertexPosition;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment fn fs_main(@location(0) localPos: vec3f) -> @location(0) vec4f {
                        let N = normalize(localPos);

                        var irradiance = vec3f(0.0);

                        var up = vec3f(0.0, 1.0, 0.0);
                        let right = normalize(cross(up, N));
                        up = normalize(cross(N, right));

                        let sampleDelta: f32 = 0.1;
                        var nrSamples: f32 = 0.0;
                        for (var phi: f32 = 0.0; phi < 2.0 * PI; phi += sampleDelta) {
                            let cosPhi = cos(phi);
                            let sinPhi = sin(phi);

                            for (var theta: f32 = 0.0; theta < 0.5 * PI; theta += sampleDelta) {
                                let cosTheta = cos(theta);
                                let sinTheta = sin(theta);

                                let tangentSample = vec3f(sinTheta * cosPhi, sinTheta * sinPhi, cosTheta);
                                let sampleVec = tangentSample.x * right + tangentSample.y * up + tangentSample.z * N;

                                irradiance += textureSample(environmentMap_tex, environmentMap_samp, sampleVec).rgb * cosTheta * sinTheta;
                                nrSamples += 1.0;
                            }
                        }
                        irradiance = PI * irradiance * (1.0 / nrSamples);

                        return vec4f(irradiance, 1.0);
                    }
                "#,
        )
    ]
)
