(
    name: "UserInterface",
    resources: [
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "solidColor", kind: Color()),
                (name: "gradientColors", kind: Vector4Array(max_len: 16, value: [])),
                (name: "gradientStops", kind: FloatArray(max_len: 16, value: [])),
                (name: "gradientOrigin", kind: Vector2()),
                (name: "gradientEnd", kind: Vector2()),
                (name: "resolution", kind: Vector2()),
                (name: "boundsMin", kind: Vector2()),
                (name: "boundsMax", kind: Vector2()),
                (name: "isFont", kind: Bool()),
                (name: "opacity", kind: Float()),
                (name: "brushType", kind: Int()),
                (name: "gradientPointCount", kind: Int()),
            ]),
            binding: 0
        ),
    ],
    passes: [
        (
            name: "Primary",

            // Drawing parameters explicitly controlled from code.

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;
                    layout (location = 1) in vec2 vertexTexCoord;
                    layout (location = 2) in vec4 vertexColor;

                    out vec2 texCoord;
                    out vec4 color;

                    void main()
                    {
                        texCoord = vertexTexCoord;
                        color = vertexColor;
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    // IMPORTANT: UI is rendered in sRGB color space!
                    out vec4 fragColor;

                    in vec2 texCoord;
                    in vec4 color;

                    float project_point(vec2 a, vec2 b, vec2 p) {
                        vec2 ab = b - a;
                        return clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
                    }

                    int find_stop_index(float t) {
                        int idx = 0;

                        for (int i = 0; i < properties.gradientPointCount; ++i) {
                            if (t > properties.gradientStops[i]) {
                                idx = i;
                            }
                        }

                        return idx;
                    }

                    void main()
                    {
                        vec2 size = vec2(properties.boundsMax.x - properties.boundsMin.x, properties.boundsMax.y - properties.boundsMin.y);
                        vec2 localPosition = (vec2(gl_FragCoord.x, properties.resolution.y - gl_FragCoord.y) - properties.boundsMin) / size;

                        if (properties.brushType == 0) {
                            // Solid color
                            fragColor = properties.solidColor;
                        } else {
                            // Gradient brush
                            float t = 0.0;

                            if (properties.brushType == 1) {
                                // Linear gradient
                                t = project_point(properties.gradientOrigin, properties.gradientEnd, localPosition);
                            } else if (properties.brushType == 2) {
                                // Radial gradient
                                t = clamp(length(localPosition - properties.gradientOrigin), 0.0, 1.0);
                            }

                            int current = find_stop_index(t);
                            int next = min(current + 1, properties.gradientPointCount);
                            float delta = properties.gradientStops[next] - properties.gradientStops[current];
                            float mix_factor = (t - properties.gradientStops[current]) / delta;
                            fragColor = mix(properties.gradientColors[current], properties.gradientColors[next], mix_factor);
                        }

                        vec4 diffuseColor = texture(diffuseTexture, texCoord);

                        if (properties.isFont)
                        {
                            fragColor.a *= diffuseColor.r;
                        }
                        else
                        {
                            fragColor *= diffuseColor;
                        }

                        fragColor.a *= properties.opacity;

                        fragColor *= color;
                    }
                "#,
        )
    ]
)