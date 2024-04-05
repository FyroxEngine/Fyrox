(
    name: "GLTFShader",

    // Each property's name must match respective uniform name.
    properties: [
        (
            name: "diffuseTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "normalTexture",
            kind: Sampler(default: None, fallback: Normal),
        ),
        (
            name: "metallicRoughnessTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "heightTexture",
            kind: Sampler(default: None, fallback: Black),
        ),
        (
            name: "emissionTexture",
            kind: Sampler(default: None, fallback: Black),
        ),
        (
            name: "lightmapTexture",
            kind: Sampler(default: None, fallback: Black),
        ),
        (
            name: "aoTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "texCoordScale",
            kind: Vector2((1.0, 1.0)),
        ),
        (
            name: "layerIndex",
            kind: UInt(0),
        ),
        (
            name: "emissionStrength",
            kind: Vector3((2.0, 2.0, 2.0)),
        ),
        (
            name: "diffuseColor",
            kind: Color(r: 255, g: 255, b: 255, a: 255),
        ),
        (
            name: "metallicFactor",
            kind: Float(0.0),
        ),
        (
            name: "roughnessFactor",
            kind: Float(0.0),
        ),
        (
            name: "parallaxCenter",
            kind: Float(0.0),
        ),
        (
            name: "parallaxScale",
            kind: Float(0.08),
        ),
    ],

    passes: [
        (
            name: "GBuffer",
            draw_parameters: DrawParameters(
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
                blend: None,
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),
            vertex_shader:
                r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 2) in vec3 vertexNormal;
                layout(location = 3) in vec4 vertexTangent;
                layout(location = 4) in vec4 boneWeights;
                layout(location = 5) in vec4 boneIndices;
                layout(location = 6) in vec2 vertexSecondTexCoord;

                // Define uniforms with reserved names. Fyrox will automatically provide
                // required data to these uniforms.
                uniform mat4 fyrox_worldMatrix;
                uniform mat4 fyrox_worldViewProjection;
                uniform bool fyrox_useSkeletalAnimation;
                uniform sampler2D fyrox_boneMatrices;
                uniform sampler3D fyrox_blendShapesStorage;
                uniform float fyrox_blendShapesWeights[128];
                uniform int fyrox_blendShapesCount;

                out vec3 position;
                out vec3 normal;
                out vec2 texCoord;
                out vec3 tangent;
                out vec3 binormal;
                out vec2 secondTexCoord;

                void main()
                {
                    vec4 localPosition = vec4(0);
                    vec3 localNormal = vec3(0);
                    vec3 localTangent = vec3(0);

                    vec4 inputPosition = vec4(vertexPosition, 1.0);
                    vec3 inputNormal = vertexNormal;
                    vec3 inputTangent = vertexTangent.xyz;

                    for (int i = 0; i < fyrox_blendShapesCount; ++i) {
                        TBlendShapeOffsets offsets = S_FetchBlendShapeOffsets(fyrox_blendShapesStorage, gl_VertexID, i);
                        float weight = fyrox_blendShapesWeights[i];
                        inputPosition.xyz += offsets.position * weight;
                        inputNormal += offsets.normal * weight;
                        inputTangent += offsets.tangent * weight;
                    }

                    if (fyrox_useSkeletalAnimation)
                    {
                        int i0 = int(boneIndices.x);
                        int i1 = int(boneIndices.y);
                        int i2 = int(boneIndices.z);
                        int i3 = int(boneIndices.w);

                        mat4 m0 = S_FetchMatrix(fyrox_boneMatrices, i0);
                        mat4 m1 = S_FetchMatrix(fyrox_boneMatrices, i1);
                        mat4 m2 = S_FetchMatrix(fyrox_boneMatrices, i2);
                        mat4 m3 = S_FetchMatrix(fyrox_boneMatrices, i3);

                        localPosition += m0 * inputPosition * boneWeights.x;
                        localPosition += m1 * inputPosition * boneWeights.y;
                        localPosition += m2 * inputPosition * boneWeights.z;
                        localPosition += m3 * inputPosition * boneWeights.w;

                        localNormal += mat3(m0) * inputNormal * boneWeights.x;
                        localNormal += mat3(m1) * inputNormal * boneWeights.y;
                        localNormal += mat3(m2) * inputNormal * boneWeights.z;
                        localNormal += mat3(m3) * inputNormal * boneWeights.w;

                        localTangent += mat3(m0) * inputTangent * boneWeights.x;
                        localTangent += mat3(m1) * inputTangent * boneWeights.y;
                        localTangent += mat3(m2) * inputTangent * boneWeights.z;
                        localTangent += mat3(m3) * inputTangent * boneWeights.w;
                    }
                    else
                    {
                        localPosition = inputPosition;
                        localNormal = inputNormal;
                        localTangent = inputTangent;
                    }

                    mat3 nm = mat3(fyrox_worldMatrix);
                    normal = normalize(nm * localNormal);
                    tangent = normalize(nm * localTangent);
                    binormal = normalize(vertexTangent.w * cross(normal, tangent));
                    texCoord = vertexTexCoord;
                    position = vec3(fyrox_worldMatrix * localPosition);
                    secondTexCoord = vertexSecondTexCoord;

                    gl_Position = fyrox_worldViewProjection * localPosition;
                }
                "#,
            fragment_shader:
                r#"
                layout(location = 0) out vec4 outColor;
                layout(location = 1) out vec4 outNormal;
                layout(location = 2) out vec4 outAmbient;
                layout(location = 3) out vec4 outMaterial;
                layout(location = 4) out uint outDecalMask;

                // Properties.
                uniform sampler2D diffuseTexture;
                uniform sampler2D normalTexture;
                uniform sampler2D metallicRoughnessTexture; // B for metallness, G for roughness
                uniform sampler2D heightTexture;
                uniform sampler2D emissionTexture;
                uniform sampler2D lightmapTexture;
                uniform sampler2D aoTexture;
                uniform vec2 texCoordScale;
                uniform uint layerIndex;
                uniform vec3 emissionStrength;
                uniform vec4 diffuseColor;
                uniform float metallicFactor;
                uniform float roughnessFactor;
                uniform float parallaxCenter;
                uniform float parallaxScale;

                // Define uniforms with reserved names. Fyrox will automatically provide
                // required data to these uniforms.
                uniform vec3 fyrox_cameraPosition;
                uniform bool fyrox_usePOM;

                in vec3 position;
                in vec3 normal;
                in vec2 texCoord;
                in vec3 tangent;
                in vec3 binormal;
                in vec2 secondTexCoord;

                void main()
                {
                    mat3 tangentSpace = mat3(tangent, binormal, normal);
                    vec3 toFragment = normalize(position - fyrox_cameraPosition);

                    vec2 tc;
                    if (fyrox_usePOM) {
                        vec3 toFragmentTangentSpace = normalize(transpose(tangentSpace) * toFragment);
                        tc = S_ComputeParallaxTextureCoordinates(
                            heightTexture,
                            toFragmentTangentSpace,
                            texCoord * texCoordScale,
                            parallaxCenter,
                            parallaxScale
                        );
                    } else {
                        tc = texCoord * texCoordScale;
                    }

                    outColor = diffuseColor * texture(diffuseTexture, tc);

                    // Alpha test.
                    if (outColor.a < 0.5) {
                        discard;
                    }
                    outColor.a = 1.0;

                    vec4 n = normalize(texture(normalTexture, tc) * 2.0 - 1.0);
                    outNormal = vec4(normalize(tangentSpace * n.xyz) * 0.5 + 0.5, 1.0);

                    outMaterial.x = metallicFactor * texture(metallicRoughnessTexture, tc).b; // Metallic
                    outMaterial.y = roughnessFactor * texture(metallicRoughnessTexture, tc).g; // Roughness
                    outMaterial.z = texture(aoTexture, tc).r;
                    outMaterial.a = 1.0;

                    outAmbient.xyz = emissionStrength * texture(emissionTexture, tc).rgb + texture(lightmapTexture, secondTexCoord).rgb;
                    outAmbient.a = 1.0;

                    outDecalMask = layerIndex;
                }
                "#,
        ),
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
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
            ),
            vertex_shader:
               r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 4) in vec4 boneWeights;
                layout(location = 5) in vec4 boneIndices;

                uniform mat4 fyrox_worldViewProjection;
                uniform bool fyrox_useSkeletalAnimation;
                uniform sampler2D fyrox_boneMatrices;
                uniform sampler3D fyrox_blendShapesStorage;
                uniform float fyrox_blendShapesWeights[128];
                uniform int fyrox_blendShapesCount;

                out vec3 position;
                out vec2 texCoord;

                void main()
                {
                    vec4 localPosition = vec4(0);

                    vec4 inputPosition = vec4(vertexPosition, 1.0);

                    for (int i = 0; i < fyrox_blendShapesCount; ++i) {
                        TBlendShapeOffsets offsets = S_FetchBlendShapeOffsets(fyrox_blendShapesStorage, gl_VertexID, i);
                        float weight = fyrox_blendShapesWeights[i];
                        inputPosition.xyz += offsets.position * weight;
                    }

                    if (fyrox_useSkeletalAnimation)
                    {
                        int i0 = int(boneIndices.x);
                        int i1 = int(boneIndices.y);
                        int i2 = int(boneIndices.z);
                        int i3 = int(boneIndices.w);

                        mat4 m0 = S_FetchMatrix(fyrox_boneMatrices, i0);
                        mat4 m1 = S_FetchMatrix(fyrox_boneMatrices, i1);
                        mat4 m2 = S_FetchMatrix(fyrox_boneMatrices, i2);
                        mat4 m3 = S_FetchMatrix(fyrox_boneMatrices, i3);

                        localPosition += m0 * inputPosition * boneWeights.x;
                        localPosition += m1 * inputPosition * boneWeights.y;
                        localPosition += m2 * inputPosition * boneWeights.z;
                        localPosition += m3 * inputPosition * boneWeights.w;
                    }
                    else
                    {
                        localPosition = inputPosition;
                    }
                    gl_Position = fyrox_worldViewProjection * localPosition;
                    texCoord = vertexTexCoord;
                }
               "#,

           fragment_shader:
               r#"
                uniform sampler2D diffuseTexture;
                uniform vec4 diffuseColor;

                out vec4 FragColor;

                in vec2 texCoord;

                void main()
                {
                    FragColor = diffuseColor * texture(diffuseTexture, texCoord);
                }
               "#,
        ),
        (
            name: "DirectionalShadow",

            draw_parameters: DrawParameters (
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: false,
                    green: false,
                    blue: false,
                    alpha: false,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
                blend: None,
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),

            vertex_shader:
                r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 4) in vec4 boneWeights;
                layout(location = 5) in vec4 boneIndices;

                uniform mat4 fyrox_worldViewProjection;
                uniform bool fyrox_useSkeletalAnimation;
                uniform sampler2D fyrox_boneMatrices;
                uniform sampler3D fyrox_blendShapesStorage;
                uniform float fyrox_blendShapesWeights[128];
                uniform int fyrox_blendShapesCount;

                out vec2 texCoord;

                void main()
                {
                    vec4 localPosition = vec4(0);

                    vec4 inputPosition = vec4(vertexPosition, 1.0);

                    for (int i = 0; i < fyrox_blendShapesCount; ++i) {
                        TBlendShapeOffsets offsets = S_FetchBlendShapeOffsets(fyrox_blendShapesStorage, gl_VertexID, i);
                        float weight = fyrox_blendShapesWeights[i];
                        inputPosition.xyz += offsets.position * weight;
                    }

                    if (fyrox_useSkeletalAnimation)
                    {
                        vec4 vertex = vec4(vertexPosition, 1.0);

                        mat4 m0 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.x));
                        mat4 m1 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.y));
                        mat4 m2 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.z));
                        mat4 m3 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.w));

                        localPosition += m0 * inputPosition * boneWeights.x;
                        localPosition += m1 * inputPosition * boneWeights.y;
                        localPosition += m2 * inputPosition * boneWeights.z;
                        localPosition += m3 * inputPosition * boneWeights.w;
                    }
                    else
                    {
                        localPosition = inputPosition;
                    }

                    gl_Position = fyrox_worldViewProjection * localPosition;
                    texCoord = vertexTexCoord;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;

                in vec2 texCoord;

                void main()
                {
                    if (texture(diffuseTexture, texCoord).a < 0.2) discard;
                }
                "#,
        ),
        (
            name: "SpotShadow",

            draw_parameters: DrawParameters (
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: false,
                    green: false,
                    blue: false,
                    alpha: false,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
                blend: None,
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),

            vertex_shader:
                r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 4) in vec4 boneWeights;
                layout(location = 5) in vec4 boneIndices;

                uniform mat4 fyrox_worldViewProjection;
                uniform bool fyrox_useSkeletalAnimation;
                uniform sampler2D fyrox_boneMatrices;
                uniform sampler3D fyrox_blendShapesStorage;
                uniform float fyrox_blendShapesWeights[128];
                uniform int fyrox_blendShapesCount;

                out vec2 texCoord;

                void main()
                {
                    vec4 localPosition = vec4(0);

                    vec4 inputPosition = vec4(vertexPosition, 1.0);

                    for (int i = 0; i < fyrox_blendShapesCount; ++i) {
                        TBlendShapeOffsets offsets = S_FetchBlendShapeOffsets(fyrox_blendShapesStorage, gl_VertexID, i);
                        float weight = fyrox_blendShapesWeights[i];
                        inputPosition.xyz += offsets.position * weight;
                    }

                    if (fyrox_useSkeletalAnimation)
                    {
                        vec4 vertex = vec4(vertexPosition, 1.0);

                        mat4 m0 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.x));
                        mat4 m1 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.y));
                        mat4 m2 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.z));
                        mat4 m3 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.w));

                        localPosition += m0 * inputPosition * boneWeights.x;
                        localPosition += m1 * inputPosition * boneWeights.y;
                        localPosition += m2 * inputPosition * boneWeights.z;
                        localPosition += m3 * inputPosition * boneWeights.w;
                    }
                    else
                    {
                        localPosition = inputPosition;
                    }

                    gl_Position = fyrox_worldViewProjection * localPosition;
                    texCoord = vertexTexCoord;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;

                in vec2 texCoord;

                void main()
                {
                    if (texture(diffuseTexture, texCoord).a < 0.2) discard;
                }
                "#,
        ),
        (
            name: "PointShadow",

            draw_parameters: DrawParameters (
                cull_face: Some(Back),
                color_write: ColorMask(
                    red: true,
                    green: true,
                    blue: true,
                    alpha: true,
                ),
                depth_write: true,
                stencil_test: None,
                depth_test: true,
                blend: None,
                stencil_op: StencilOp(
                    fail: Keep,
                    zfail: Keep,
                    zpass: Keep,
                    write_mask: 0xFFFF_FFFF,
                ),
            ),

            vertex_shader:
                r#"
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;
                layout(location = 4) in vec4 boneWeights;
                layout(location = 5) in vec4 boneIndices;

                uniform mat4 fyrox_worldMatrix;
                uniform mat4 fyrox_worldViewProjection;
                uniform bool fyrox_useSkeletalAnimation;
                uniform sampler2D fyrox_boneMatrices;
                uniform sampler3D fyrox_blendShapesStorage;
                uniform float fyrox_blendShapesWeights[128];
                uniform int fyrox_blendShapesCount;

                out vec2 texCoord;
                out vec3 worldPosition;

                void main()
                {
                    vec4 localPosition = vec4(0);

                    vec4 inputPosition = vec4(vertexPosition, 1.0);

                    for (int i = 0; i < fyrox_blendShapesCount; ++i) {
                        TBlendShapeOffsets offsets = S_FetchBlendShapeOffsets(fyrox_blendShapesStorage, gl_VertexID, i);
                        float weight = fyrox_blendShapesWeights[i];
                        inputPosition.xyz += offsets.position * weight;
                    }

                    if (fyrox_useSkeletalAnimation)
                    {
                        vec4 vertex = vec4(vertexPosition, 1.0);

                        mat4 m0 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.x));
                        mat4 m1 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.y));
                        mat4 m2 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.z));
                        mat4 m3 = S_FetchMatrix(fyrox_boneMatrices, int(boneIndices.w));

                        localPosition += m0 * inputPosition * boneWeights.x;
                        localPosition += m1 * inputPosition * boneWeights.y;
                        localPosition += m2 * inputPosition * boneWeights.z;
                        localPosition += m3 * inputPosition * boneWeights.w;
                    }
                    else
                    {
                        localPosition = inputPosition;
                    }

                    gl_Position = fyrox_worldViewProjection * localPosition;
                    worldPosition = (fyrox_worldMatrix * localPosition).xyz;
                    texCoord = vertexTexCoord;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;

                uniform vec3 fyrox_lightPosition;

                in vec2 texCoord;
                in vec3 worldPosition;

                layout(location = 0) out float depth;

                void main()
                {
                    if (texture(diffuseTexture, texCoord).a < 0.2) discard;
                    depth = length(fyrox_lightPosition - worldPosition);
                }
                "#,
        )
    ],
)