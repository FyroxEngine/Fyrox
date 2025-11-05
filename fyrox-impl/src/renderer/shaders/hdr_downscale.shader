(
    name: "HdrDownscale",
    resources: [
        (
            name: "lumSampler",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "invSize", kind: Vector2()),
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
                    layout (location = 0) in vec3 vertexPosition;
                    layout (location = 1) in vec2 vertexTexCoord;

                    out vec2 texCoord;

                    void main()
                    {
                        texCoord = vertexTexCoord;
                        gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
                    }
                "#,

            fragment_shader:
                r#"
                    in vec2 texCoord;

                    out float outLum;

                    void main() {
                        float x = properties.invSize.x;
                        float y = properties.invSize.y;

                        float a = texture(lumSampler, vec2(texCoord.x - 2*x, texCoord.y + 2*y)).r;
                        float b = texture(lumSampler, vec2(texCoord.x,       texCoord.y + 2*y)).r;
                        float c = texture(lumSampler, vec2(texCoord.x + 2*x, texCoord.y + 2*y)).r;

                        float d = texture(lumSampler, vec2(texCoord.x - 2*x, texCoord.y)).r;
                        float e = texture(lumSampler, vec2(texCoord.x,       texCoord.y)).r;
                        float f = texture(lumSampler, vec2(texCoord.x + 2*x, texCoord.y)).r;

                        float g = texture(lumSampler, vec2(texCoord.x - 2*x, texCoord.y - 2*y)).r;
                        float h = texture(lumSampler, vec2(texCoord.x,       texCoord.y - 2*y)).r;
                        float i = texture(lumSampler, vec2(texCoord.x + 2*x, texCoord.y - 2*y)).r;

                        float j = texture(lumSampler, vec2(texCoord.x - x, texCoord.y + y)).r;
                        float k = texture(lumSampler, vec2(texCoord.x + x, texCoord.y + y)).r;
                        float l = texture(lumSampler, vec2(texCoord.x - x, texCoord.y - y)).r;
                        float m = texture(lumSampler, vec2(texCoord.x + x, texCoord.y - y)).r;

                        outLum = e*0.125;
                        outLum += (a+c+g+i)*0.03125;
                        outLum += (b+d+f+h)*0.0625;
                        outLum += (j+k+l+m)*0.125;
                    }
                "#,
        )
    ]
)