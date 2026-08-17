(
    name: "StandardTwoSidesShader",

    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: Normal),
            binding: 1
        ),
        (
            name: "metallicTexture",
            kind: Texture(kind: Sampler2D, fallback: Black),
            binding: 2
        ),
        (
            name: "roughnessTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "heightTexture",
            kind: Texture(kind: Sampler2D, fallback: Black),
            binding: 4
        ),
        (
            name: "emissionTexture",
            kind: Texture(kind: Sampler2D, fallback: Black),
            binding: 5
        ),
        (
            name: "lightmapTexture",
            kind: Texture(kind: Sampler2D, fallback: Black),
            binding: 6
        ),
        (
            name: "aoTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 7
        ),
        (
            name: "blendShapesStorage",
            kind: Texture(kind: Sampler3D, fallback: Volume),
            binding: 8
        ),
        (
            name: "properties",
            kind: PropertyGroup([
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
        (name: "fyrox_boneMatrices", kind: PropertyGroup([]), binding: 2),
        (name: "fyrox_graphicsSettings", kind: PropertyGroup([]), binding: 3),
        (name: "fyrox_cameraData", kind: PropertyGroup([]), binding: 4),
        (name: "fyrox_lightData", kind: PropertyGroup([]), binding: 5),
    ],

    passes: [
        (
            name: "GBuffer",
            draw_parameters: DrawParameters(cull_face: None, color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput {
                    @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(2) vertexNormal: vec3f,
                    @location(3) vertexTangent: vec4f, @location(4) boneWeights: vec4f, @location(5) boneIndices: vec4f,
                    @location(6) vertexSecondTexCoord: vec2f, @builtin(vertex_index) vertex_index: u32,
                };
                struct VertexOutput {
                    @builtin(position) position: vec4f, @location(0) outPosition: vec3f, @location(1) outNormal: vec3f,
                    @location(2) texCoord: vec2f, @location(3) outTangent: vec3f, @location(4) outBinormal: vec3f, @location(5) secondTexCoord: vec2f,
                };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput;
                    var localPosition = vec4f(0.0); var localNormal = vec3f(0.0); var localTangent = vec3f(0.0);
                    var inputPosition = vec4f(input.vertexPosition, 1.0); var inputNormal = input.vertexNormal; var inputTangent = input.vertexTangent.xyz;
                    for (var i: i32 = 0; i < i32(fyrox_instanceData.blendShapesCount); i++) {
                        let offsets = S_FetchBlendShapeOffsets(blendShapesStorage_tex, blendShapesStorage_samp, i32(input.vertex_index), i);
                        let weight = fyrox_instanceData.blendShapesWeights[i / 4][i % 4];
                        inputPosition += vec4f(offsets.position * weight, 0.0); inputNormal += offsets.normal * weight; inputTangent += offsets.tangent * weight;
                    }
                    if (fyrox_instanceData.useSkeletalAnimation != 0u) {
                        let m0 = fyrox_boneMatrices.matrices[i32(input.boneIndices.x)]; let m1 = fyrox_boneMatrices.matrices[i32(input.boneIndices.y)];
                        let m2 = fyrox_boneMatrices.matrices[i32(input.boneIndices.z)]; let m3 = fyrox_boneMatrices.matrices[i32(input.boneIndices.w)];
                        localPosition += m0 * inputPosition * input.boneWeights.x + m1 * inputPosition * input.boneWeights.y + m2 * inputPosition * input.boneWeights.z + m3 * inputPosition * input.boneWeights.w;
                        localNormal += mat3x3f(m0[0].xyz, m0[1].xyz, m0[2].xyz) * inputNormal * input.boneWeights.x + mat3x3f(m1[0].xyz, m1[1].xyz, m1[2].xyz) * inputNormal * input.boneWeights.y + mat3x3f(m2[0].xyz, m2[1].xyz, m2[2].xyz) * inputNormal * input.boneWeights.z + mat3x3f(m3[0].xyz, m3[1].xyz, m3[2].xyz) * inputNormal * input.boneWeights.w;
                        localTangent += mat3x3f(m0[0].xyz, m0[1].xyz, m0[2].xyz) * inputTangent * input.boneWeights.x + mat3x3f(m1[0].xyz, m1[1].xyz, m1[2].xyz) * inputTangent * input.boneWeights.y + mat3x3f(m2[0].xyz, m2[1].xyz, m2[2].xyz) * inputTangent * input.boneWeights.z + mat3x3f(m3[0].xyz, m3[1].xyz, m3[2].xyz) * inputTangent * input.boneWeights.w;
                    } else { localPosition = inputPosition; localNormal = inputNormal; localTangent = inputTangent; }
                    let nm = mat3x3f(fyrox_instanceData.worldMatrix[0].xyz, fyrox_instanceData.worldMatrix[1].xyz, fyrox_instanceData.worldMatrix[2].xyz);
                    output.outNormal = normalize(nm * localNormal); output.outTangent = normalize(nm * localTangent);
                    output.outBinormal = normalize(input.vertexTangent.w * cross(output.outNormal, output.outTangent));
                    output.texCoord = input.vertexTexCoord; output.outPosition = (fyrox_instanceData.worldMatrix * localPosition).xyz;
                    output.secondTexCoord = input.vertexSecondTexCoord; output.position = fyrox_instanceData.worldViewProjection * localPosition;
                    return output;
                }
                "#,
            fragment_shader:
                r#"
                struct FragmentOutput { @location(0) outColor: vec4f, @location(1) outNormal: vec4f, @location(2) outAmbient: vec4f, @location(3) outMaterial: vec4f, @location(4) outDecalMask: u32 };
                @fragment fn fs_main(@location(0) position: vec3f, @location(1) normal: vec3f, @location(2) texCoord: vec2f, @location(3) tangent: vec3f, @location(4) binormal: vec3f, @location(5) secondTexCoord: vec2f) -> FragmentOutput {
                    var output: FragmentOutput;
                    let tangentSpace = mat3x3f(tangent, binormal, normal);
                    let toFragment = normalize(position - fyrox_cameraData.position);
                    var tc: vec2f;
                    if (fyrox_graphicsSettings.usePOM != 0u) { tc = S_ComputeParallaxTextureCoordinates(heightTexture_tex, heightTexture_samp, normalize(transpose(tangentSpace) * toFragment), texCoord * properties.texCoordScale, properties.parallaxCenter, properties.parallaxScale); } else { tc = texCoord * properties.texCoordScale; }
                    output.outColor = properties.diffuseColor * textureSample(diffuseTexture_tex, diffuseTexture_samp, tc);
                    if (output.outColor.a < 0.5) { discard; } output.outColor.a = 1.0;
                    let n = normalize(textureSample(normalTexture_tex, normalTexture_samp, tc) * 2.0 - 1.0);
                    output.outNormal = vec4f(normalize(tangentSpace * n.xyz) * 0.5 + 0.5, 1.0);
                    output.outMaterial.x = textureSample(metallicTexture_tex, metallicTexture_samp, tc).r;
                    output.outMaterial.y = textureSample(roughnessTexture_tex, roughnessTexture_samp, tc).r;
                    output.outMaterial.z = textureSample(aoTexture_tex, aoTexture_samp, tc).r; output.outMaterial.a = 1.0;
                    output.outAmbient = vec4f(properties.emissionStrength * textureSample(emissionTexture_tex, emissionTexture_samp, tc).rgb + textureSample(lightmapTexture_tex, lightmapTexture_samp, secondTexCoord).rgb, 1.0); output.outDecalMask = properties.layerIndex;
                    return output;
                }
                "#,
        ),
        (
            name: "Forward",
            draw_parameters: DrawParameters(cull_face: None, color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: Some(BlendParameters(func: BlendFunc(sfactor: SrcAlpha, dfactor: OneMinusSrcAlpha, alpha_sfactor: SrcAlpha, alpha_dfactor: OneMinusSrcAlpha), equation: BlendEquation(rgb: Add, alpha: Add))), stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(4) boneWeights: vec4f, @location(5) boneIndices: vec4f, @builtin(vertex_index) vertex_index: u32 };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) outPosition: vec3f, @location(1) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput; var localPosition = vec4f(0.0); var inputPosition = vec4f(input.vertexPosition, 1.0);
                    for (var i: i32 = 0; i < i32(fyrox_instanceData.blendShapesCount); i++) { let offsets = S_FetchBlendShapeOffsets(blendShapesStorage_tex, blendShapesStorage_samp, i32(input.vertex_index), i); let weight = fyrox_instanceData.blendShapesWeights[i / 4][i % 4]; inputPosition += vec4f(offsets.position * weight, 0.0); }
                    if (fyrox_instanceData.useSkeletalAnimation != 0u) { let m0 = fyrox_boneMatrices.matrices[i32(input.boneIndices.x)]; let m1 = fyrox_boneMatrices.matrices[i32(input.boneIndices.y)]; let m2 = fyrox_boneMatrices.matrices[i32(input.boneIndices.z)]; let m3 = fyrox_boneMatrices.matrices[i32(input.boneIndices.w)]; localPosition += m0 * inputPosition * input.boneWeights.x + m1 * inputPosition * input.boneWeights.y + m2 * inputPosition * input.boneWeights.z + m3 * inputPosition * input.boneWeights.w; } else { localPosition = inputPosition; }
                    output.position = fyrox_instanceData.worldViewProjection * localPosition; output.texCoord = input.vertexTexCoord; return output;
                }
                "#,
            fragment_shader: r#"@fragment fn fs_main(@location(1) texCoord: vec2f) -> @location(0) vec4f { return properties.diffuseColor * S_SRGBToLinear(textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord)); }"#,
        ),
        (
            name: "DirectionalShadow",
            draw_parameters: DrawParameters(cull_face: None, color_write: ColorMask(red: false, green: false, blue: false, alpha: false), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(4) boneWeights: vec4f, @location(5) boneIndices: vec4f, @builtin(vertex_index) vertex_index: u32 };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput; var localPosition = vec4f(0.0); var inputPosition = vec4f(input.vertexPosition, 1.0);
                    for (var i: i32 = 0; i < i32(fyrox_instanceData.blendShapesCount); i++) { let offsets = S_FetchBlendShapeOffsets(blendShapesStorage_tex, blendShapesStorage_samp, i32(input.vertex_index), i); let weight = fyrox_instanceData.blendShapesWeights[i / 4][i % 4]; inputPosition += vec4f(offsets.position * weight, 0.0); }
                    if (fyrox_instanceData.useSkeletalAnimation != 0u) { let m0 = fyrox_boneMatrices.matrices[i32(input.boneIndices.x)]; let m1 = fyrox_boneMatrices.matrices[i32(input.boneIndices.y)]; let m2 = fyrox_boneMatrices.matrices[i32(input.boneIndices.z)]; let m3 = fyrox_boneMatrices.matrices[i32(input.boneIndices.w)]; localPosition += m0 * inputPosition * input.boneWeights.x + m1 * inputPosition * input.boneWeights.y + m2 * inputPosition * input.boneWeights.z + m3 * inputPosition * input.boneWeights.w; } else { localPosition = inputPosition; }
                    output.position = fyrox_instanceData.worldViewProjection * localPosition; output.texCoord = input.vertexTexCoord; return output;
                }
                "#,
            fragment_shader: r#"@fragment fn fs_main(@location(0) texCoord: vec2f) { if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; } }"#,
        ),
        (
            name: "SpotShadow",
            draw_parameters: DrawParameters(cull_face: None, color_write: ColorMask(red: false, green: false, blue: false, alpha: false), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(4) boneWeights: vec4f, @location(5) boneIndices: vec4f, @builtin(vertex_index) vertex_index: u32 };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput; var localPosition = vec4f(0.0); var inputPosition = vec4f(input.vertexPosition, 1.0);
                    for (var i: i32 = 0; i < i32(fyrox_instanceData.blendShapesCount); i++) { let offsets = S_FetchBlendShapeOffsets(blendShapesStorage_tex, blendShapesStorage_samp, i32(input.vertex_index), i); let weight = fyrox_instanceData.blendShapesWeights[i / 4][i % 4]; inputPosition += vec4f(offsets.position * weight, 0.0); }
                    if (fyrox_instanceData.useSkeletalAnimation != 0u) { let m0 = fyrox_boneMatrices.matrices[i32(input.boneIndices.x)]; let m1 = fyrox_boneMatrices.matrices[i32(input.boneIndices.y)]; let m2 = fyrox_boneMatrices.matrices[i32(input.boneIndices.z)]; let m3 = fyrox_boneMatrices.matrices[i32(input.boneIndices.w)]; localPosition += m0 * inputPosition * input.boneWeights.x + m1 * inputPosition * input.boneWeights.y + m2 * inputPosition * input.boneWeights.z + m3 * inputPosition * input.boneWeights.w; } else { localPosition = inputPosition; }
                    output.position = fyrox_instanceData.worldViewProjection * localPosition; output.texCoord = input.vertexTexCoord; return output;
                }
                "#,
            fragment_shader: r#"@fragment fn fs_main(@location(0) texCoord: vec2f) { if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; } }"#,
        ),
        (
            name: "PointShadow",
            draw_parameters: DrawParameters(cull_face: None, color_write: ColorMask(red: true, green: true, blue: true, alpha: true), depth_write: true, stencil_test: None, depth_test: Some(Less), blend: None, stencil_op: StencilOp(fail: Keep, zfail: Keep, zpass: Keep, write_mask: 0xFFFF_FFFF), scissor_box: None),
            vertex_shader:
                r#"
                struct VertexInput { @location(0) vertexPosition: vec3f, @location(1) vertexTexCoord: vec2f, @location(4) boneWeights: vec4f, @location(5) boneIndices: vec4f, @builtin(vertex_index) vertex_index: u32 };
                struct VertexOutput { @builtin(position) position: vec4f, @location(0) texCoord: vec2f, @location(1) worldPosition: vec3f };
                @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                    var output: VertexOutput; var localPosition = vec4f(0.0); var inputPosition = vec4f(input.vertexPosition, 1.0);
                    for (var i: i32 = 0; i < i32(fyrox_instanceData.blendShapesCount); i++) { let offsets = S_FetchBlendShapeOffsets(blendShapesStorage_tex, blendShapesStorage_samp, i32(input.vertex_index), i); let weight = fyrox_instanceData.blendShapesWeights[i / 4][i % 4]; inputPosition += vec4f(offsets.position * weight, 0.0); }
                    if (fyrox_instanceData.useSkeletalAnimation != 0u) { let m0 = fyrox_boneMatrices.matrices[i32(input.boneIndices.x)]; let m1 = fyrox_boneMatrices.matrices[i32(input.boneIndices.y)]; let m2 = fyrox_boneMatrices.matrices[i32(input.boneIndices.z)]; let m3 = fyrox_boneMatrices.matrices[i32(input.boneIndices.w)]; localPosition += m0 * inputPosition * input.boneWeights.x + m1 * inputPosition * input.boneWeights.y + m2 * inputPosition * input.boneWeights.z + m3 * inputPosition * input.boneWeights.w; } else { localPosition = inputPosition; }
                    output.position = fyrox_instanceData.worldViewProjection * localPosition; output.worldPosition = (fyrox_instanceData.worldMatrix * localPosition).xyz; output.texCoord = input.vertexTexCoord; return output;
                }
                "#,
            fragment_shader: r#"@fragment fn fs_main(@location(0) texCoord: vec2f, @location(1) worldPosition: vec3f) -> @location(0) f32 { if (textureSample(diffuseTexture_tex, diffuseTexture_samp, texCoord).a < 0.2) { discard; } return length(fyrox_lightData.lightPosition - worldPosition); }"#,
        )
    ],
)
