(
    name: "Standard2DShader",

    properties: [
        (
            name: "diffuseTexture",
            kind: Sampler(default: None, kind: Sampler2D, fallback: White),
        ),
    ],

    passes: [
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: None,
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: Some(Less),
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
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 2) in vec4 vertexColor;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec2 texCoord;
                out vec4 color;
                out vec3 fragmentPosition;

                void main()
                {
                    texCoord = vertexTexCoord;
                    fragmentPosition = (fyrox_instanceData.worldMatrix * vec4(vertexPosition, 1.0)).xyz;
                    gl_Position = fyrox_instanceData.worldViewProjection * vec4(vertexPosition, 1.0);
                    color = vertexColor;
                }
               "#,

           fragment_shader:
               r#"
                layout(std140) uniform FyroxLightData {
                    TLightData fyrox_lightData;
                };
                layout(std140) uniform FyroxLightsBlock {
                    TLightsBlock fyrox_lightsBlock;
                };

                out vec4 FragColor;

                in vec2 texCoord;
                in vec4 color;
                in vec3 fragmentPosition;

                void main()
                {
                    vec3 lighting = fyrox_lightData.ambientLightColor.xyz;
                    for(int i = 0; i < min(fyrox_lightsBlock.lightCount, MAX_LIGHT_COUNT); ++i) {
                        // "Unpack" light parameters.
                        float halfHotspotAngleCos = fyrox_lightsBlock.lightsParameters[i].x;
                        float halfConeAngleCos = fyrox_lightsBlock.lightsParameters[i].y;
                        vec3 lightColor = fyrox_lightsBlock.lightsColorRadius[i].xyz;
                        float radius = fyrox_lightsBlock.lightsColorRadius[i].w;
                        vec3 lightPosition = fyrox_lightsBlock.lightsPosition[i];
                        vec3 direction = fyrox_lightsBlock.lightsDirection[i];

                        // Calculate lighting.
                        vec3 toFragment = fragmentPosition - lightPosition;
                        float distance = length(toFragment);
                        vec3 toFragmentNormalized = toFragment / distance;
                        float distanceAttenuation = S_LightDistanceAttenuation(distance, radius);
                        float spotAngleCos = dot(toFragmentNormalized, direction);
                        float directionalAttenuation = smoothstep(halfConeAngleCos, halfHotspotAngleCos, spotAngleCos);
                        lighting += lightColor * (distanceAttenuation * directionalAttenuation);
                    }

                    FragColor = vec4(lighting, 1.0) * color * S_SRGBToLinear(texture(diffuseTexture, texCoord));
                }
               "#,
        )
    ],
)