(
    name: "StandardTerrainShader",

    resources: [
        (name: "diffuseTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 0),
        (name: "normalTexture", kind: Texture(kind: Sampler2D, fallback: Normal), binding: 1),
        (name: "metallicTexture", kind: Texture(kind: Sampler2D, fallback: Black), binding: 2),
        (name: "roughnessTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 3),
        (name: "heightTexture", kind: Texture(kind: Sampler2D, fallback: Black), binding: 4),
        (name: "emissionTexture", kind: Texture(kind: Sampler2D, fallback: Black), binding: 5),
        (name: "lightmapTexture", kind: Texture(kind: Sampler2D, fallback: Black), binding: 6),
        (name: "aoTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 7),
        (name: "maskTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 8),
        (name: "heightMapTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 9),
        (name: "holeMaskTexture", kind: Texture(kind: Sampler2D, fallback: White), binding: 10),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "nodeUvOffsets", kind: Vector4(value: (0.0, 0.0, 0.0, 0.0))),
                (name: "texCoordScale", kind: Vector2(value: (1.0, 1.0))),
                (name: "layerIndex", kind: UInt(value: 0)),
                (name: "emissionStrength", kind: Vector3(value: (2.0, 2.0, 2.0))),
                (name: "diffuseColor", kind: Color(r: 255, g: 255, b: 255, a: 255)),
                (name: "parallaxCenter", kind: Float(value: 0.0)),
                (name: "parallaxScale", kind: Float(value: 0.08)),
            ]),
            binding: 0
        ),
        (name: "fyrox_instanceData", kind: PropertyGroup([]), binding: 1),
        (name: "fyrox_graphicsSettings", kind: PropertyGroup([]), binding: 2),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 3),
        (name: "fyrox_lightData", kind: PropertyGroup([]), binding: 4),
    ],

    passes: [
        (
            name: "GBuffer",
            draw_parameters: DrawParameters(cull_face: Some(Back), color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(LessOrEqual), blend: Some(BlendParameters(func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha, alpha_sfactor: SrcAlpha, alpha_dfactor: OneMinusSrcAlpha), equation: BlendEquation(rgb: Add, alpha: Max))), stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput {
                    @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(2) vertexNormal: vec3f,
                    @location(3) vertexTangent: vec4f, @location(6) vertexSecondTexCoord: vec2f,
                };
                struct VertexOutput {
                    @builtin(position) position: vec4f, @location(0) outPosition: vec3f, @location(1) outNormal: vec3f,
                    @location(2) texCoord: vec2f, @location(3) outTangent: vec3f, @location(4) outBinormal: vec3f, @location(5) secondTexCoord: vec2f,
                };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    let actualTexCoords = input.vertexTexCoord * properties.nodeUvOffsets.zw + properties.nodeUvOffsets.xy;
                    let heightSize = vec2f(textureDimensions(heightMapTexture_tex, 0));
                    let innerSize = heightSize - 3.0;
                    let pixelSize = 1.0 / heightSize;
                    let heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    let heightDim = textureDimensions(heightMapTexture_tex, 0);
                    let height = textureLoad(heightMapTexture_tex, vec2i(heightCoords * vec2f(heightDim)), 0).r;
                    let finalVertexPosition = vec4f(input.vertexPosition.x, height, input.vertexPosition.z, 1.0);
                    let hx0 = textureLoad(heightMapTexture_tex, vec2i((heightCoords + vec2f(-1.0, 0.0) * pixelSize) * vec2f(heightDim)), 0).r;
                    let hx1 = textureLoad(heightMapTexture_tex, vec2i((heightCoords + vec2f(1.0, 0.0) * pixelSize) * vec2f(heightDim)), 0).r;
                    let hy0 = textureLoad(heightMapTexture_tex, vec2i((heightCoords + vec2f(0.0, -1.0) * pixelSize) * vec2f(heightDim)), 0).r;
                    let hy1 = textureLoad(heightMapTexture_tex, vec2i((heightCoords + vec2f(0.0, 1.0) * pixelSize) * vec2f(heightDim)), 0).r;
                    let n = vec3f((hx0 - hx1) / 2.0, 1.0, (hy0 - hy1) / 2.0);
                    let tan = vec3f(n.y, -n.x, 0.0);
                    let nm = mat3x3f(fyrox_instanceData.worldMatrix[0].xyz, fyrox_instanceData.worldMatrix[1].xyz, fyrox_instanceData.worldMatrix[2].xyz);
                    output.outNormal = normalize(nm * n);
                    output.outTangent = normalize(nm * tan);
                    output.outBinormal = normalize(-1.0 * cross(output.outNormal, output.outTangent));
                    output.texCoord = actualTexCoords;
                    output.outPosition = (fyrox_instanceData.worldMatrix * finalVertexPosition).xyz;
                    output.secondTexCoord = input.vertexSecondTexCoord;
                    output.position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                struct FragmentOutput { @location(0) outColor: vec4f, @location(1) outNormal: vec4f, @location(2) outAmbient: vec4f, @location(3) outMaterial: vec4f, @location(4) outDecalMask: u32 };
                @fragment fn fs_main(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) texCoord: vec2f, @location(3) tangent: vec3f, @location(4) binormal: vec3f, @location(5) secondTexCoord: vec2f) -> FragmentOutput {
                    var output: FragmentOutput;
                    if (textureSample(holeMaskTexture_tex, holeMaskTexture_samp, texCoord).r < 0.5) { discard; }
                    let tangentSpace = mat3x3f(tangent, binormal, normal);
                    let toFragment = normalize(position - fyrox_cameraData.position);
                    var tc: vec2f;
                    if (fyrox_graphicsSettings.usePOM != 0u) { tc = S_ComputeParallaxTextureCoordinates(heightTexture_tex, heightTexture_samp, normalize(transpose(tangentSpace) * toFragment), texCoord * properties.texCoordScale, properties.parallaxCenter, properties.parallaxScale); } else { tc = texCoord * properties.texCoordScale; }
                    output.outColor = properties.diffuseColor * textureSample(diffuseTexture_tex, diffuseTexture_samp, tc);
                    let n = normalize(textureSample(normalTexture_tex, normalTexture_samp, tc).xyz * 2.0 - 1.0);
                    output.outNormal = vec4f(normalize(tangentSpace * n) * 0.5 + 0.5, 1.0);
                    output.outMaterial.x = textureSample(metallicTexture_tex, metallicTexture_samp, tc).r;
                    output.outMaterial.y = textureSample(roughnessTexture_tex, roughnessTexture_samp, tc).r;
                    output.outMaterial.z = textureSample(aoTexture_tex, aoTexture_samp, tc).r; output.outMaterial.a = 1.0;
                    output.outAmbient = vec4f(properties.emissionStrength * textureSample(emissionTexture_tex, emissionTexture_samp, tc).rgb + textureSample(lightmapTexture_tex, lightmapTexture_samp, secondTexCoord).rgb, 1.0); output.outDecalMask = properties.layerIndex;
                    let mask = textureSample(maskTexture_tex, maskTexture_samp, texCoord).r;
                    output.outColor.a = mask; output.outAmbient.a = mask; output.outNormal.a = mask; output.outMaterial.a = mask;
                    return output;
                }
                "#,
        ),
        (
            name: "Forward",
            draw_parameters: DrawParameters(cull_face: Some(Back), color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: Some(BlendParameters(func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha, alpha_sfactor: SrcAlpha, alpha_dfactor: OneMinusSrcAlpha), equation: BlendEquation(rgb: Add, alpha: Max))), stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) outPosition: vec3f, @location(1) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    let actualTexCoords = input.vertexTexCoord * properties.nodeUvOffsets.zw + properties.nodeUvOffsets.xy;
                    let heightSize = vec2f(textureDimensions(heightMapTexture_tex, 0));
                    let innerSize = heightSize - 3.0;
                    let pixelSize = 1.0 / heightSize;
                    let heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    let heightDim = textureDimensions(heightMapTexture_tex, 0);
                    let height = textureLoad(heightMapTexture_tex, vec2i(heightCoords * vec2f(heightDim)), 0).r;
                    let finalVertexPosition = vec4f(input.vertexPosition.x, height, input.vertexPosition.z, 1.0);
                    output.position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    output.texCoord = actualTexCoords;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                @fragment fn fs_main(@location(1) texCoord: vec2f) -> @location(0) vec4f {
                    if (textureSample(holeMaskTexture_tex, holeMaskTexture_samp, texCoord).r < 0.5) { discard; }
                    return properties.diffuseColor * S_SRGBToLinear(textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord));
                }
                "#,
        ),
        (
            name: "SpotShadow",
            draw_parameters: DrawParameters(cull_face: Some(Back), color_write: ColorMask(red: false, green: false, blue: false, alpha: false), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    let actualTexCoords = input.vertexTexCoord * properties.nodeUvOffsets.zw + properties.nodeUvOffsets.xy;
                    let heightSize = vec2f(textureDimensions(heightMapTexture_tex, 0));
                    let innerSize = heightSize - 3.0;
                    let pixelSize = 1.0 / heightSize;
                    let heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    let heightDim = textureDimensions(heightMapTexture_tex, 0);
                    let height = textureLoad(heightMapTexture_tex, vec2i(heightCoords * vec2f(heightDim)), 0).r;
                    let finalVertexPosition = vec4f(input.vertexPosition.x, height, input.vertexPosition.z, 1.0);
                    output.position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    output.texCoord = actualTexCoords;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                @fragment fn fs_main(@location(0) texCoord: vec2f) {
                    if (textureSample(holeMaskTexture_tex, holeMaskTexture_samp, texCoord).r < 0.5) { discard; }
                    if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; }
                }
                "#,
        ),
        (
            name: "DirectionalShadow",
            draw_parameters: DrawParameters(cull_face: Some(Back), color_write: ColorMask(red: false, green: false, blue: false, alpha: false), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    let actualTexCoords = input.vertexTexCoord * properties.nodeUvOffsets.zw + properties.nodeUvOffsets.xy;
                    let heightSize = vec2f(textureDimensions(heightMapTexture_tex, 0));
                    let innerSize = heightSize - 3.0;
                    let pixelSize = 1.0 / heightSize;
                    let heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    let heightDim = textureDimensions(heightMapTexture_tex, 0);
                    let height = textureLoad(heightMapTexture_tex, vec2i(heightCoords * vec2f(heightDim)), 0).r;
                    let finalVertexPosition = vec4f(input.vertexPosition.x, height, input.vertexPosition.z, 1.0);
                    output.position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    output.texCoord = actualTexCoords;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                @fragment fn fs_main(@location(0) texCoord: vec2f) {
                    if (textureSample(holeMaskTexture_tex, holeMaskTexture_samp, texCoord).r < 0.5) { discard; }
                    if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; }
                }
                "#,
        ),
        (
            name: "PointShadow",
            draw_parameters: DrawParameters(cull_face: Some(Back), color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f, @location(1) worldPosition: vec3f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    let actualTexCoords = input.vertexTexCoord * properties.nodeUvOffsets.zw + properties.nodeUvOffsets.xy;
                    let heightSize = vec2f(textureDimensions(heightMapTexture_tex, 0));
                    let innerSize = heightSize - 3.0;
                    let pixelSize = 1.0 / heightSize;
                    let heightCoords = (actualTexCoords * innerSize + 1.5) * pixelSize;
                    let heightDim = textureDimensions(heightMapTexture_tex, 0);
                    let height = textureLoad(heightMapTexture_tex, vec2i(heightCoords * vec2f(heightDim)), 0).r;
                    let finalVertexPosition = vec4f(input.vertexPosition.x, height, input.vertexPosition.z, 1.0);
                    output.position = fyrox_instanceData.worldViewProjection * finalVertexPosition;
                    output.worldPosition = (fyrox_instanceData.worldMatrix * finalVertexPosition).xyz;
                    output.texCoord = actualTexCoords;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                @fragment fn fs_main(@location(0) texCoord: vec2f, @location(1) worldPosition: vec3f) -> @location(0) f32 {
                    if (textureSample(holeMaskTexture_tex, holeMaskTexture_samp, texCoord).r < 0.5) { discard; }
                    if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; }
                    return length(fyrox_lightData.lightPosition - worldPosition);
                }
                "#,
        )
    ],
)
