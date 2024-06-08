(
    name: "GridShader",

    properties: [
        (
            name: "diffuseColor",
            kind: Color(r: 40, g: 40, b: 40, a: 255),
        ),
        (
            name: "xAxisColor",
            kind: Color(r: 255, g: 0, b: 0, a: 255),
        ),
        (
            name: "zAxisColor",
            kind: Color(r: 0, g: 0, b: 255, a: 255),
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
                uniform vec4 xAxisColor;
                uniform vec4 zAxisColor;

                uniform mat4 fyrox_viewProjectionMatrix;
                uniform float fyrox_zNear;
                uniform float fyrox_zFar;
                uniform vec3 fyrox_cameraPosition;

                out vec4 FragColor;

                in vec3 nearPoint;
                in vec3 farPoint;

                vec4 grid(vec3 fragPos3D, float scale) {
                    vec2 coord = fragPos3D.xz * scale;
                    vec2 derivative = fwidth(coord);
                    vec2 grid = abs(fract(coord - 0.5) - 0.5) / derivative;
                    float line = min(grid.x, grid.y);
                    float minZ = 0.5 * min(derivative.y, 1);
                    float minX = 0.5 * min(derivative.x, 1);

                    vec4 color = diffuseColor;
                    float alpha = 1.0 - min(line, 1.0);
                    // Sharpen lines a bit.
                    color.a = alpha >= 0.5 ? 1.0 : 0.0;

                    if (fragPos3D.x > -minX && fragPos3D.x < minX) {
                        // z axis
                        color.xyz = zAxisColor.xyz;
                    } else if (fragPos3D.z > -minZ && fragPos3D.z < minZ) {
                        // x axis
                        color.xyz = xAxisColor.xyz;
                    } else {
                        vec3 viewDir = fragPos3D - fyrox_cameraPosition;
                        // This helps to negate moire pattern at large distances.
                        float cosAngle = abs(dot(vec3(0.0, 1.0, 0.0), normalize(viewDir)));
                        color.a *= cosAngle;
                    }

                    return color;
                }

                float computeDepth(vec3 pos) {
                    vec4 clip_space_pos = fyrox_viewProjectionMatrix * vec4(pos.xyz, 1.0);
                    return (clip_space_pos.z / clip_space_pos.w);
                }

                void main()
                {
                    float t = -nearPoint.y / max(farPoint.y - nearPoint.y, 0.000001);

                    vec3 fragPos3D = nearPoint + t * (farPoint - nearPoint);

                    float depth = computeDepth(fragPos3D);
                    gl_FragDepth = ((gl_DepthRange.diff * depth) + gl_DepthRange.near + gl_DepthRange.far) / 2.0;

                    FragColor = grid(fragPos3D, 1.0);
                    FragColor.a *= float(t > 0);

                    // Alpha test to prevent blending issues.
                    if (FragColor.a < 0.01) {
                        discard;
                    }
                }
               "#,
        ),
    ],
)