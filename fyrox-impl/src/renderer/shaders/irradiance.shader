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

                void main()
                {
                    vec3 N = normalize(localPos);

                    vec3 irradiance = vec3(0.0);

                    vec3 up  = vec3(0.0, 1.0, 0.0);
                    vec3 right = normalize(cross(up, N));
                    up = normalize(cross(N, right));

                    float sampleDelta = 0.025;
                    float nrSamples = 0.0;
                    for(float phi = 0.0; phi < 2.0 * PI; phi += sampleDelta)
                    {
                        for(float theta = 0.0; theta < 0.5 * PI; theta += sampleDelta)
                        {
                            vec3 tangentSample = vec3(sin(theta) * cos(phi),  sin(theta) * sin(phi), cos(theta));
                            vec3 sampleVec = tangentSample.x * right + tangentSample.y * up + tangentSample.z * N;

                            irradiance += texture(environmentMap, sampleVec).rgb * cos(theta) * sin(theta);
                            nrSamples++;
                        }
                    }
                    irradiance = PI * irradiance * (1.0 / float(nrSamples));

                    FragColor = vec4(irradiance, 1.0);
                }
                "#,
        )
    ]
)