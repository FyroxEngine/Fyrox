(
    name: "GizmoShader",

    properties: [
        (
            name: "diffuseColor",
            kind: Color(r: 255, g: 255, b: 255, a: 255),
        ),
		(
            name: "diffuseTexture",
            kind: Sampler(default: None, fallback: White),
        ),
		(
			name: "uvScale",
			kind: Float(1024.0)
		)
    ],

    passes: [
        (
            name: "Forward",
            draw_parameters: DrawParameters(
                cull_face: None,
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

                uniform mat4 fyrox_worldViewProjection;

				out vec2 texCoord;

                void main()
                {
                    gl_Position = fyrox_worldViewProjection * vec4(vertexPosition, 1.0);
					texCoord = vertexTexCoord;
                }
               "#,

           fragment_shader:
               r#"
                uniform vec4 diffuseColor;
				uniform sampler2D diffuseTexture;
				uniform float uvScale;

				in vec2 texCoord;

                out vec4 FragColor;

                void main()
                {
                    FragColor = texture(diffuseTexture, uvScale * texCoord) * diffuseColor;
                }
               "#,
        ),
    ],
)