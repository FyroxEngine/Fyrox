(
    name: "Decal",
    resources: [
        (
            name: "sceneDepth",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "decalMask",
            kind: Texture(kind: USampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "invWorldDecal", kind: Matrix4()),
                (name: "resolution", kind: Vector2()),
                (name: "color", kind: Vector4()),
                (name: "layerIndex", kind: UInt()),
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

                    out vec4 clipSpacePosition;

                    void main()
                    {
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                        clipSpacePosition = gl_Position;
                    }
                "#,

            fragment_shader:
                r#"
                    layout (location = 0) out vec4 outDiffuseMap;
                    layout (location = 1) out vec4 outNormalMap;

                    in vec4 clipSpacePosition;

                    void main()
                    {
                        vec2 screenPos = clipSpacePosition.xy / clipSpacePosition.w;

                        vec2 texCoord = vec2(
                        (1.0 + screenPos.x) / 2.0 + (0.5 / properties.resolution.x),
                        (1.0 + screenPos.y) / 2.0 + (0.5 / properties.resolution.y)
                        );

                        uvec4 maskIndex = texture(decalMask, texCoord);

                        // Masking.
                        if (maskIndex.r != properties.layerIndex) {
                            discard;
                        }

                        float sceneDepth = texture(sceneDepth, texCoord).r;

                        vec3 sceneWorldPosition = S_UnProject(vec3(texCoord, sceneDepth), properties.invViewProj);

                        vec3 decalSpacePosition = (properties.invWorldDecal * vec4(sceneWorldPosition, 1.0)).xyz;

                        // Check if scene pixel is not inside decal bounds.
                        vec3 dpos = vec3(0.5) - abs(decalSpacePosition.xyz);
                        if (dpos.x < 0.0 || dpos.y < 0.0 || dpos.z < 0.0) {
                            discard;
                        }

                        vec2 decalTexCoord = decalSpacePosition.xz + 0.5;

                        outDiffuseMap = properties.color * texture(diffuseTexture, decalTexCoord);

                        vec3 fragmentTangent = dFdx(sceneWorldPosition);
                        vec3 fragmentBinormal = dFdy(sceneWorldPosition);
                        vec3 fragmentNormal = cross(fragmentTangent, fragmentBinormal);

                        mat3 tangentToWorld;
                        tangentToWorld[0] = normalize(fragmentTangent); // Tangent
                        tangentToWorld[1] = normalize(fragmentBinormal); // Binormal
                        tangentToWorld[2] = normalize(fragmentNormal); // Normal

                        vec3 rawNormal = (texture(normalTexture, decalTexCoord) * 2.0 - 1.0).xyz;
                        vec3 worldSpaceNormal = tangentToWorld * rawNormal;
                        outNormalMap = vec4(worldSpaceNormal * 0.5 + 0.5, outDiffuseMap.a);
                    }
                "#,
        )
    ]
)