(
    name: "StandardTerrainShader",

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
            name: "metallicTexture",
            kind: Sampler(default: None, fallback: Black),
        ),
        (
            name: "roughnessTexture",
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
            name: "maskTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "heightMapTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "holeMaskTexture",
            kind: Sampler(default: None, fallback: White),
        ),
        (
            name: "nodeUvOffsets",
            kind: Vector4((0.0, 0.0, 0.0, 0.0)),
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
                        alpha: Max
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
                layout(location = 2) in vec3 vertexNormal;
                layout(location = 3) in vec4 vertexTangent;
                layout(location = 6) in vec2 vertexSecondTexCoord;

                // Properties
                uniform sampler2D heightMapTexture;
                uniform vec4 nodeUvOffsets;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec3 position;
                out vec3 normal;
                out vec2 texCoord;
                out vec3 tangent;
                out vec3 binormal;
                out vec2 secondTexCoord;

                void main()
                {
                    // Each node has tex coords in [0; 1] range, here we must scale and offset it
                    // to match the actual position.
                    vec2 actualTexCoords = vec2(vertexTexCoord * nodeUvOffsets.zw + nodeUvOffsets.xy);
                    vec2 heightSize = vec2(textureSize(heightMapTexture, 0));
                    vec2 innerSize = heightSize - 3.0;
                    vec2 pixelSize = 1.0 / heightSize;
                    vec2 heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    float height = texture(heightMapTexture, heightCoords).r;
                    vec4 finalVertexPosition = vec4(vertexPosition.x, height, vertexPosition.z, 1.0);
                    float hx0 = texture(heightMapTexture, heightCoords + vec2(-1.0, 0.0) * pixelSize).r;
                    float hx1 = texture(heightMapTexture, heightCoords + vec2(1.0, 0.0) * pixelSize).r;
                    float hy0 = texture(heightMapTexture, heightCoords + vec2(0.0, -1.0) * pixelSize).r;
                    float hy1 = texture(heightMapTexture, heightCoords + vec2(0.0, 1.0) * pixelSize).r;
                    vec2 pixelFactor = heightSize / nodeUvOffsets.zw;
                    vec3 n = vec3(hx0-hx1, 2.0, hy0-hy1) * vec3(pixelFactor.x, 1.0, pixelFactor.y);
                    vec3 tan = vec3(n.y, -n.x, 0.0);

                    mat3 nm = mat3(fyrox_instanceData.worldMatrix);
                    normal = normalize(nm * n);
                    tangent = normalize(nm * tan);
                    binormal = normalize(-1.0 * cross(normal, tangent));
                    texCoord = actualTexCoords;
                    position = vec3(fyrox_instanceData.worldMatrix * finalVertexPosition);
                    secondTexCoord = vertexSecondTexCoord;
                    gl_Position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
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
                uniform sampler2D metallicTexture;
                uniform sampler2D roughnessTexture;
                uniform sampler2D heightTexture;
                uniform sampler2D emissionTexture;
                uniform sampler2D lightmapTexture;
                uniform sampler2D aoTexture;
                uniform vec2 texCoordScale;
                uniform uint layerIndex;
                uniform vec3 emissionStrength;
                uniform sampler2D maskTexture;
                uniform sampler2D holeMaskTexture;
                uniform vec4 diffuseColor;
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
                    if (texture(holeMaskTexture, texCoord).r < 0.5) discard;

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

                    vec3 n = normalize(texture(normalTexture, tc).xyz * 2.0 - 1.0);
                    outNormal = vec4(normalize(tangentSpace * n) * 0.5 + 0.5, 1.0);

                    outMaterial.x = texture(metallicTexture, tc).r;
                    outMaterial.y = texture(roughnessTexture, tc).r;
                    outMaterial.z = texture(aoTexture, tc).r;
                    outMaterial.a = 1.0;

                    outAmbient.xyz = emissionStrength * texture(emissionTexture, tc).rgb + texture(lightmapTexture, secondTexCoord).rgb;
                    outAmbient.a = 1.0;

                    outDecalMask = layerIndex;

                    float mask = texture(maskTexture, texCoord).r;

                    outColor.a = mask;
                    outAmbient.a = mask;
                    outNormal.a = mask;
                    outMaterial.a = mask;
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
                        alpha: Max
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

                // Properties
                uniform sampler2D heightMapTexture;
                uniform vec4 nodeUvOffsets;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec3 position;
                out vec2 texCoord;

                void main()
                {
                    vec2 actualTexCoords = vec2(vertexTexCoord * nodeUvOffsets.zw + nodeUvOffsets.xy);
                    vec2 heightSize = vec2(textureSize(heightMapTexture, 0));
                    vec2 innerSize = heightSize - 3.0;
                    vec2 pixelSize = 1.0 / heightSize;
                    vec2 heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    float height = texture(heightMapTexture, heightCoords).r;
                    vec4 finalVertexPosition = vec4(vertexPosition.x, height, vertexPosition.z, 1.0);

                    gl_Position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    texCoord = actualTexCoords;
                }
               "#,

           fragment_shader:
               r#"
                uniform sampler2D diffuseTexture;
                uniform vec4 diffuseColor;
                uniform sampler2D holeMaskTexture;

                out vec4 FragColor;

                in vec2 texCoord;

                void main()
                {
                    if (texture(holeMaskTexture, texCoord).r < 0.5) discard;
                    FragColor = diffuseColor * texture(diffuseTexture, texCoord);
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
                depth_test: Some(Less),
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
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;

                // Properties
                uniform sampler2D heightMapTexture;
                uniform vec4 nodeUvOffsets;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec2 texCoord;

                void main()
                {
                    vec2 actualTexCoords = vec2(vertexTexCoord * nodeUvOffsets.zw + nodeUvOffsets.xy);
                    vec2 heightSize = vec2(textureSize(heightMapTexture, 0));
                    vec2 innerSize = heightSize - 3.0;
                    vec2 pixelSize = 1.0 / heightSize;
                    vec2 heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    float height = texture(heightMapTexture, heightCoords).r;
                    vec4 finalVertexPosition = vec4(vertexPosition.x, height, vertexPosition.z, 1.0);

                    gl_Position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    texCoord = actualTexCoords;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;
                uniform sampler2D holeMaskTexture;

                in vec2 texCoord;

                void main()
                {
                    if (texture(holeMaskTexture, texCoord).r < 0.5) discard;
                    if (texture(diffuseTexture, texCoord).a < 0.2) discard;
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
                depth_test: Some(Less),
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
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;

                // Properties
                uniform sampler2D heightMapTexture;
                uniform vec4 nodeUvOffsets;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec2 texCoord;

                void main()
                {
                    vec2 actualTexCoords = vec2(vertexTexCoord * nodeUvOffsets.zw + nodeUvOffsets.xy);
                    vec2 heightSize = vec2(textureSize(heightMapTexture, 0));
                    vec2 innerSize = heightSize - 3.0;
                    vec2 pixelSize = 1.0 / heightSize;
                    vec2 heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    float height = texture(heightMapTexture, heightCoords).r;
                    vec4 finalVertexPosition = vec4(vertexPosition.x, height, vertexPosition.z, 1.0);

                    gl_Position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    texCoord = actualTexCoords;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;
                uniform sampler2D holeMaskTexture;

                in vec2 texCoord;

                void main()
                {
                    if (texture(holeMaskTexture, texCoord).r < 0.5) discard;
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
                depth_test: Some(Less),
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
                layout(location = 0) in vec3 vertexPosition;
                layout(location = 1) in vec2 vertexTexCoord;

                // Properties
                uniform sampler2D heightMapTexture;
                uniform vec4 nodeUvOffsets;

                layout(std140) uniform FyroxInstanceData {
                    TInstanceData fyrox_instanceData;
                };

                out vec2 texCoord;
                out vec3 worldPosition;

                void main()
                {
                    vec2 actualTexCoords = vec2(vertexTexCoord * nodeUvOffsets.zw + nodeUvOffsets.xy);
                    vec2 heightSize = vec2(textureSize(heightMapTexture, 0));
                    vec2 innerSize = heightSize - 3.0;
                    vec2 pixelSize = 1.0 / heightSize;
                    vec2 heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    float height = texture(heightMapTexture, heightCoords).r;
                    vec4 finalVertexPosition = vec4(vertexPosition.x, height, vertexPosition.z, 1.0);

                    gl_Position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    worldPosition = (fyrox_instanceData.worldMatrix * finalVertexPosition).xyz;
                    texCoord = actualTexCoords;
                }
                "#,

            fragment_shader:
                r#"
                uniform sampler2D diffuseTexture;
                uniform sampler2D holeMaskTexture;

                uniform vec3 fyrox_lightPosition;

                in vec2 texCoord;
                in vec3 worldPosition;

                layout(location = 0) out float depth;

                void main()
                {
                    if (texture(holeMaskTexture, texCoord).r < 0.5) discard;
                    if (texture(diffuseTexture, texCoord).a < 0.2) discard;
                    depth = length(fyrox_lightPosition - worldPosition);
                }
                "#,
        )
    ],
)