(
    name: "GizmoShader",

    properties: [
        (
            name: "diffuseColor",
            kind: Color(r: 255, g: 255, b: 255, a: 255),
        ),
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
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: Some(BlendFunc(
                    sfactor: SrcAlpha,
                    dfactor: OneMinusSrcAlpha,
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

                uniform mat4 fyrox_worldViewProjection;

                void main()
                {
                    gl_Position = fyrox_worldViewProjection * vec4(vertexPosition, 1.0);
                }
               "#,

           fragment_shader:
               r#"
                uniform vec4 diffuseColor;

                out vec4 FragColor;

                void main()
                {
                    FragColor = diffuseColor;
                }
               "#,
        ),
    ],
)