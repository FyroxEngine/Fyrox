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

                uniform mat4 fyrox_viewProjectionMatrix;

                out vec3 nearPoint;
                out vec3 farPoint;

                vec3 Unproject(float x, float y, float z, mat4 matrix)
                {
                    vec4 position = matrix * vec4(x, y, z, 1.0);
                    return position.xyz / position.w;
                }

                void main()
                {
                    mat4 invViewProj = inverse(fyrox_viewProjectionMatrix);
                    nearPoint = Unproject(vertexPosition.x, vertexPosition.y, 0.0, invViewProj);
                    farPoint = Unproject(vertexPosition.x, vertexPosition.y, 1.0, invViewProj);
                    gl_Position = vec4(vertexPosition, 1.0);
                }
               "#,

           fragment_shader:
               r#"
                // Original code: https://asliceofrendering.com/scene%20helper/2020/01/05/InfiniteGrid/
                // Fixed and adapted for Fyrox.

                uniform vec4 diffuseColor;

                uniform mat4 fyrox_viewProjectionMatrix;
                uniform float fyrox_zNear;
                uniform float fyrox_zFar;

                out vec4 FragColor;

                in vec3 nearPoint;
                in vec3 farPoint;

                vec4 grid(vec3 fragPos3D, float scale) {
                    vec2 coord = fragPos3D.xz * scale;
                    vec2 derivative = fwidth(coord);
                    vec2 grid = abs(fract(coord - 0.5) - 0.5) / derivative;
                    float line = min(grid.x, grid.y);
                    float minimumz = min(derivative.y, 1);
                    float minimumx = min(derivative.x, 1);
                    vec4 color = vec4(0.2, 0.2, 0.2, 1.0 - min(line, 1.0));

                    // z axis
                    if (fragPos3D.x > -0.1 * minimumx && fragPos3D.x < 0.1 * minimumx) {
                        color.z = 1.0;
                    }

                    // x axis
                    if (fragPos3D.z > -0.1 * minimumz && fragPos3D.z < 0.1 * minimumz) {
                        color.x = 1.0;
                    }

                    return color;
                }

                float computeDepth(vec3 pos) {
                    vec4 clip_space_pos = fyrox_viewProjectionMatrix * vec4(pos.xyz, 1.0);
                    return (clip_space_pos.z / clip_space_pos.w);
                }

                float computeLinearDepth(vec3 pos) {
                    vec4 clip_space_pos = fyrox_viewProjectionMatrix * vec4(pos.xyz, 1.0);
                    float clip_space_depth = (clip_space_pos.z / clip_space_pos.w) * 2.0 - 1.0;
                    float denom = (fyrox_zFar + fyrox_zNear - clip_space_depth * (fyrox_zFar - fyrox_zNear));
                    float linearDepth = (2.0 * fyrox_zNear * fyrox_zFar) / max(denom, 0.00001);
                    return linearDepth / fyrox_zFar;
                }

                void main()
                {
                    float t = -nearPoint.y / (farPoint.y - nearPoint.y);

                    vec3 fragPos3D = nearPoint + t * (farPoint - nearPoint);

                    gl_FragDepth = (((fyrox_zFar - fyrox_zNear) * computeDepth(fragPos3D)) +
                                    fyrox_zNear + fyrox_zFar) / 2.0;

                    float linearDepth = computeLinearDepth(fragPos3D);
                    float fading = max(0, (0.25 - linearDepth));

                    FragColor = grid(fragPos3D, 1.0);
                    FragColor.a *= fading * float(t > 0);
                }
               "#,
        ),
    ],
)