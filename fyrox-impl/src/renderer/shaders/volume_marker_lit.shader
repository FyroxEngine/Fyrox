(
    name: "VolumeMarkerLighting",
    resources: [
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

            // Drawing params are dynamic.

            vertex_shader:
                r#"
                    layout (location = 0) in vec3 vertexPosition;

                    void main()
                    {
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    out vec4 FragColor;

                    void main()
                    {
                        FragColor = vec4(1.0);
                    }
                "#,
        )
    ]
)