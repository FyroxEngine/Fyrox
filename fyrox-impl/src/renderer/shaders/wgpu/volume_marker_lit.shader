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
                    struct VertexInput {
                        @location(0) vertexPosition: vec3f,
                    };

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                    };

                    @vertex
                    fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.position = properties.worldViewProjection * vec4f(input.vertexPosition, 1.0);
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    @fragment
                    fn fs_main() -> @location(0) vec4f {
                        return vec4f(1.0);
                    }
                "#,
        )
    ]
)
