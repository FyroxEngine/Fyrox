(
    name: "Decal",
    resources: [
        (
            name: "sceneDepth",
            kind: Texture(kind: DepthSampler2D, fallback: White),
            binding: 0
        ),
        (
            name: "diffuseTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 1
        ),
        (
            name: "normalTexture",
            kind: Texture(kind: Sampler2D, fallback: White),
            binding: 2
        ),
        (
            name: "decalMask",
            kind: Texture(kind: USampler2D, fallback: White),
            binding: 3
        ),
        (
            name: "properties",
            kind: PropertyGroup([
                (name: "worldViewProjection", kind: Matrix4()),
                (name: "invViewProj", kind: Matrix4()),
                (name: "invWorldDecal", kind: Matrix4()),
                (name: "resolution", kind: Vector2()),
                (name: "color", kind: Vector4()),
                (name: "layerIndex", kind: UInt()),
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
                scissor_box: None
            ),

            vertex_shader:
                r#"
                    struct VertexInput {
                        @location(0) vertex_position: vec3f,
                    }

                    struct VertexOutput {
                        @builtin(position) position: vec4f,
                        @location(0) clip_space_position: vec4f,
                    }

                    @vertex fn vs_main(input: VertexInput) -> VertexOutput {
                        var output: VertexOutput;
                        output.position = properties.worldViewProjection * vec4f(input.vertex_position, 1.0);
                        output.clip_space_position = output.position;
                        return output;
                    }
                "#,

            fragment_shader:
                r#"
                    struct FragmentOutput {
                        @location(0) out_diffuse_map: vec4f,
                        @location(1) out_normal_map: vec4f,
                    }

                    @fragment fn fs_main(@location(0) clip_space_position: vec4f) -> FragmentOutput {
                        let screen_pos = clip_space_position.xy / clip_space_position.w;

                        let tex_coord = vec2f(
                            (1.0 + screen_pos.x) / 2.0 + (0.5 / properties.resolution.x),
                            (1.0 + screen_pos.y) / 2.0 + (0.5 / properties.resolution.y)
                        );

                        let mask_index = textureLoad(decalMask_tex, vec2i(tex_coord * vec2f(textureDimensions(decalMask_tex))), 0);

                        // Masking.
                        if (mask_index.r != properties.layerIndex) {
                            discard;
                        }

                        let scene_depth = textureSample(sceneDepth_tex, sceneDepth_samp, tex_coord);

                        let scene_world_position = S_UnProject(vec3f(tex_coord, scene_depth), properties.invViewProj);

                        let decal_space_position = (properties.invWorldDecal * vec4f(scene_world_position, 1.0)).xyz;

                        // Check if scene pixel is not inside decal bounds.
                        let dpos = vec3f(0.5) - abs(decal_space_position);
                        if (dpos.x < 0.0 || dpos.y < 0.0 || dpos.z < 0.0) {
                            discard;
                        }

                        let decal_tex_coord = decal_space_position.xz + 0.5;

                        let diffuse = properties.color * textureSample(diffuseTexture_tex, diffuseTexture_samp, decal_tex_coord);

                        let fragment_tangent = dpdx(scene_world_position);
                        let fragment_binormal = dpdy(scene_world_position);
                        let fragment_normal = cross(fragment_tangent, fragment_binormal);

                        var tangent_to_world: mat3x3f;
                        tangent_to_world[0] = normalize(fragment_tangent); // Tangent
                        tangent_to_world[1] = normalize(fragment_binormal); // Binormal
                        tangent_to_world[2] = normalize(fragment_normal); // Normal

                        let raw_normal = (textureSample(normalTexture_tex, normalTexture_samp, decal_tex_coord) * 2.0 - 1.0).xyz;
                        let world_space_normal = tangent_to_world * raw_normal;

                        var output: FragmentOutput;
                        output.out_diffuse_map = diffuse;
                        output.out_normal_map = vec4f(world_space_normal * 0.5 + 0.5, diffuse.a);
                        return output;
                    }
                "#,
        )
    ]
)