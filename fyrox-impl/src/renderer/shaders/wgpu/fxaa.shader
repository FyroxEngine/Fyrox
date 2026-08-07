(
    name: "FXAA",
    resources: [
        (
            name: "screenTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "inverseScreenSize", kind: Vector2()),
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
                        @location(1) vertexTexCoord: vec2f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) texCoord: vec2f,
                    };

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.texCoord = input.vertexTexCoord;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    // NVIDIA FXAA 3.11
                    // Original source code by TIMOTHY LOTTES
                    // WGSL port

                    const EDGE_THRESHOLD_MIN: f32 = 0.0312;
                    const EDGE_THRESHOLD_MAX: f32 = 0.125;
                    const ITERATIONS: i32 = 12;
                    const SUBPIXEL_QUALITY: f32 = 0.75;

                    fn quality(q: i32) -> f32 {
                        if (q < 5) { return 1.0; }
                        if (q > 5) {
                            if (q < 10) { return 2.0; }
                            if (q < 11) { return 4.0; }
                            return 8.0;
                        }
                        return 1.5;
                    }

                    fn rgb2luma(rgb: vec3f) -> f32 {
                        return sqrt(dot(rgb, vec3f(0.299, 0.587, 0.114)));
                    }

                    @fragment fn fs_main(@location(0) texCoord: vec2f) -> @location(0) vec4f {
                        let colorCenter = textureSample(screenTexture_tex, screenTexture_samp, texCoord);
                        let lumaCenter = rgb2luma(colorCenter.rgb);

                        let lumaDown = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(0.0, -properties.inverseScreenSize.y), 0.0).rgb);
                        let lumaUp = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(0.0, properties.inverseScreenSize.y), 0.0).rgb);
                        let lumaLeft = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(-properties.inverseScreenSize.x, 0.0), 0.0).rgb);
                        let lumaRight = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(properties.inverseScreenSize.x, 0.0), 0.0).rgb);

                        let lumaMin = min(lumaCenter, min(min(lumaDown, lumaUp), min(lumaLeft, lumaRight)));
                        let lumaMax = max(lumaCenter, max(max(lumaDown, lumaUp), max(lumaLeft, lumaRight)));
                        let lumaRange = lumaMax - lumaMin;

                        if (lumaRange < max(EDGE_THRESHOLD_MIN, lumaMax * EDGE_THRESHOLD_MAX)) {
                            return colorCenter;
                        }

                        let lumaDownLeft = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(-properties.inverseScreenSize.x, -properties.inverseScreenSize.y), 0.0).rgb);
                        let lumaUpRight = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(properties.inverseScreenSize.x, properties.inverseScreenSize.y), 0.0).rgb);
                        let lumaUpLeft = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(-properties.inverseScreenSize.x, properties.inverseScreenSize.y), 0.0).rgb);
                        let lumaDownRight = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, texCoord + vec2f(properties.inverseScreenSize.x, -properties.inverseScreenSize.y), 0.0).rgb);

                        let lumaDownUp = lumaDown + lumaUp;
                        let lumaLeftRight = lumaLeft + lumaRight;
                        let lumaLeftCorners = lumaDownLeft + lumaUpLeft;
                        let lumaDownCorners = lumaDownLeft + lumaDownRight;
                        let lumaRightCorners = lumaDownRight + lumaUpRight;
                        let lumaUpCorners = lumaUpRight + lumaUpLeft;

                        let edgeHorizontal = abs(-2.0 * lumaLeft + lumaLeftCorners) + abs(-2.0 * lumaCenter + lumaDownUp) * 2.0 + abs(-2.0 * lumaRight + lumaRightCorners);
                        let edgeVertical = abs(-2.0 * lumaUp + lumaUpCorners) + abs(-2.0 * lumaCenter + lumaLeftRight) * 2.0 + abs(-2.0 * lumaDown + lumaDownCorners);

                        let isHorizontal = (edgeHorizontal >= edgeVertical);
                        var stepLength = select(properties.inverseScreenSize.x, properties.inverseScreenSize.y, isHorizontal);

                        let luma1 = select(lumaLeft, lumaDown, isHorizontal);
                        let luma2 = select(lumaRight, lumaUp, isHorizontal);
                        let gradient1 = luma1 - lumaCenter;
                        let gradient2 = luma2 - lumaCenter;
                        let is1Steepest = abs(gradient1) >= abs(gradient2);
                        let gradientScaled = 0.25 * max(abs(gradient1), abs(gradient2));

                        var lumaLocalAverage: f32;
                        if (is1Steepest) {
                            stepLength = -stepLength;
                            lumaLocalAverage = 0.5 * (luma1 + lumaCenter);
                        } else {
                            lumaLocalAverage = 0.5 * (luma2 + lumaCenter);
                        }

                        var currentUv = texCoord;
                        if (isHorizontal) {
                            currentUv.y += stepLength * 0.5;
                        } else {
                            currentUv.x += stepLength * 0.5;
                        }

                        let offset = select(vec2f(0.0, properties.inverseScreenSize.y), vec2f(properties.inverseScreenSize.x, 0.0), isHorizontal);
                        var uv1 = currentUv - offset * quality(0);
                        var uv2 = currentUv + offset * quality(0);

                        var lumaEnd1 = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, uv1, 0.0).rgb) - lumaLocalAverage;
                        var lumaEnd2 = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, uv2, 0.0).rgb) - lumaLocalAverage;

                        var reached1 = abs(lumaEnd1) >= gradientScaled;
                        var reached2 = abs(lumaEnd2) >= gradientScaled;
                        var reachedBoth = reached1 && reached2;

                        if (!reached1) { uv1 -= offset * quality(1); }
                        if (!reached2) { uv2 += offset * quality(1); }

                        if (!reachedBoth) {
                            for (var i: i32 = 2; i < ITERATIONS; i++) {
                                if (!reached1) {
                                    lumaEnd1 = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, uv1, 0.0).rgb) - lumaLocalAverage;
                                }
                                if (!reached2) {
                                    lumaEnd2 = rgb2luma(textureSampleLevel(screenTexture_tex, screenTexture_samp, uv2, 0.0).rgb) - lumaLocalAverage;
                                }
                                reached1 = abs(lumaEnd1) >= gradientScaled;
                                reached2 = abs(lumaEnd2) >= gradientScaled;
                                reachedBoth = reached1 && reached2;

                                if (!reached1) { uv1 -= offset * quality(i); }
                                if (!reached2) { uv2 += offset * quality(i); }
                                if (reachedBoth) { break; }
                            }
                        }

                        let distance1 = select(texCoord.y - uv1.y, texCoord.x - uv1.x, isHorizontal);
                        let distance2 = select(uv2.y - texCoord.y, uv2.x - texCoord.x, isHorizontal);
                        let isDirection1 = distance1 < distance2;
                        let distanceFinal = min(distance1, distance2);
                        let edgeThickness = (distance1 + distance2);
                        let isLumaCenterSmaller = lumaCenter < lumaLocalAverage;
                        let correctVariation1 = (lumaEnd1 < 0.0) != isLumaCenterSmaller;
                        let correctVariation2 = (lumaEnd2 < 0.0) != isLumaCenterSmaller;
                        let correctVariation = select(correctVariation2, correctVariation1, isDirection1);

                        let pixelOffset = -distanceFinal / edgeThickness + 0.5;
                        var finalOffset = select(0.0, pixelOffset, correctVariation);

                        let lumaAverage = (1.0 / 12.0) * (2.0 * (lumaDownUp + lumaLeftRight) + lumaLeftCorners + lumaRightCorners);
                        let subPixelOffset1 = clamp(abs(lumaAverage - lumaCenter) / lumaRange, 0.0, 1.0);
                        let subPixelOffset2 = (-2.0 * subPixelOffset1 + 3.0) * subPixelOffset1 * subPixelOffset1;
                        let subPixelOffsetFinal = subPixelOffset2 * subPixelOffset2 * SUBPIXEL_QUALITY;
                        finalOffset = max(finalOffset, subPixelOffsetFinal);

                        var finalUv = texCoord;
                        if (isHorizontal) {
                            finalUv.y += finalOffset * stepLength;
                        } else {
                            finalUv.x += finalOffset * stepLength;
                        }

                        let finalColor = textureSampleLevel(screenTexture_tex, screenTexture_samp, finalUv, 0.0).rgb;
                        return vec4f(finalColor, 1.0);
                    }
                "#,
        )
    ]
)
