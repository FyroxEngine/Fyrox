use crate::bitflags::bitflags;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

fn make_vertex_shader_source(features: UberShaderFeatures) -> String {
    let mut source = "#version 330 core\n".to_owned();

    if features.contains(UberShaderFeatures::INSTANCING) {
        if features.contains(UberShaderFeatures::LIGHTMAP) {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;            
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 6) in vec2 vertexSecondTexCoord;
            layout(location = 7) in vec4 instanceColor;
            layout(location = 8) in mat4 worldMatrix;
            layout(location = 12) in float depthOffset;
            "#;
        } else {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 7) in vec4 instanceColor;
            layout(location = 8) in mat4 worldMatrix;
            layout(location = 12) in float depthOffset;
            "#;
        }

        source += r#"
            uniform sampler2D matrixStorage;
            uniform vec4 matrixStorageSize; // vec4(1/w, 1/h, w, h)
            uniform mat4 viewProjectionMatrix;
            uniform int matrixBufferStride;

            vec2 IdToCoords(float k, float w, float inv_w) {
                float y = floor(k * inv_w); // floor(k / w)
                float x = k - w * y; // k % w
                return vec2(x, y);
            }

            mat4 ReadMatrix(int id)
            {
                float w = matrixStorageSize.z;
                float inv_w = matrixStorageSize.x;
                float inv_h = matrixStorageSize.y;

                vec2 coords = IdToCoords(4.0 * float(id), w, inv_w);

                float ty = (coords.y + 0.5) * inv_h;

                vec4 col1 = texture(matrixStorage, vec2((coords.x + 0.5) * inv_w, ty));
                vec4 col2 = texture(matrixStorage, vec2((coords.x + 1.5) * inv_w, ty));
                vec4 col3 = texture(matrixStorage, vec2((coords.x + 2.5) * inv_w, ty));
                vec4 col4 = texture(matrixStorage, vec2((coords.x + 3.5) * inv_w, ty));

                return mat4(col1, col2, col3, col4);
            }
        "#;
    } else {
        if features.contains(UberShaderFeatures::LIGHTMAP) {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;            
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            layout(location = 6) in vec2 vertexSecondTexCoord;
            "#;
        } else {
            source += r#"
            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
            "#;
        }

        source += r#"
            uniform mat4 worldMatrix;
            uniform mat4 worldViewProjection;
            uniform mat4 boneMatrices[60];
        "#;
    }

    source += r#"
        uniform bool useSkeletalAnimation;

        out vec3 position;
        out vec3 normal;
        out vec2 texCoord;
        out vec3 tangent;
        out vec3 binormal;        
    "#;

    if features.contains(UberShaderFeatures::LIGHTMAP) {
        source += r#"
        out vec2 secondTexCoord;
    "#;
    }

    if features.contains(UberShaderFeatures::INSTANCING) {
        source += r#"
        out vec4 diffuseColor;
    "#;
    }

    source += r#"
        void main()
        {
            vec4 localPosition = vec4(0);
            vec3 localNormal = vec3(0);
            vec3 localTangent = vec3(0);
    
            if (useSkeletalAnimation)
            {
                vec4 vertex = vec4(vertexPosition, 1.0);
    
                int i0 = int(boneIndices.x);
                int i1 = int(boneIndices.y);
                int i2 = int(boneIndices.z);
                int i3 = int(boneIndices.w);
                "#;

    if features.contains(UberShaderFeatures::INSTANCING) {
        source += r#"
                int boneIndexOrigin = gl_InstanceID * matrixBufferStride;

                mat4 m0 = ReadMatrix(boneIndexOrigin + i0);
                mat4 m1 = ReadMatrix(boneIndexOrigin + i1);
                mat4 m2 = ReadMatrix(boneIndexOrigin + i2);
                mat4 m3 = ReadMatrix(boneIndexOrigin + i3);
                "#;
    } else {
        source += r#"
                mat4 m0 = boneMatrices[i0];
                mat4 m1 = boneMatrices[i1];
                mat4 m2 = boneMatrices[i2];
                mat4 m3 = boneMatrices[i3];
                "#;
    }

    source += r#"
                localPosition += m0 * vertex * boneWeights.x;
                localPosition += m1 * vertex * boneWeights.y;
                localPosition += m2 * vertex * boneWeights.z;
                localPosition += m3 * vertex * boneWeights.w;
                
                localNormal += mat3(m0) * vertexNormal * boneWeights.x;
                localNormal += mat3(m1) * vertexNormal * boneWeights.y;
                localNormal += mat3(m2) * vertexNormal * boneWeights.z;
                localNormal += mat3(m3) * vertexNormal * boneWeights.w;
                
                localTangent += mat3(m0) * vertexTangent.xyz * boneWeights.x;
                localTangent += mat3(m1) * vertexTangent.xyz * boneWeights.y;
                localTangent += mat3(m2) * vertexTangent.xyz * boneWeights.z;
                localTangent += mat3(m3) * vertexTangent.xyz * boneWeights.w;             
            }
            else
            {
                localPosition = vec4(vertexPosition, 1.0);
                localNormal = vertexNormal;
                localTangent = vertexTangent.xyz;
            }

            mat3 nm = mat3(worldMatrix);
            normal = normalize(nm * localNormal);
            tangent = normalize(nm * localTangent);
            binormal = normalize(vertexTangent.w * cross(tangent, normal));
            texCoord = vertexTexCoord;
            position = vec3(worldMatrix * localPosition);            
            "#;

    if features.contains(UberShaderFeatures::LIGHTMAP) {
        source += r#"
        secondTexCoord = vertexSecondTexCoord;
    "#;
    }

    if features.contains(UberShaderFeatures::INSTANCING) {
        source += r#"
            mat4 viewProj = viewProjectionMatrix;
            viewProj[3].z -= depthOffset;
            gl_Position = (viewProj * worldMatrix) * localPosition;
            diffuseColor = instanceColor;
        "#;
    } else {
        source += r#"
            gl_Position = worldViewProjection * localPosition;
        "#;
    }

    source += r#"}"#;

    source
}

fn make_fragment_shader_source(features: UberShaderFeatures) -> String {
    let mut source = r#"#version 330 core
    "#
    .to_owned();

    source += r#"
        layout(location = 0) out vec4 outColor;
        layout(location = 1) out vec4 outNormal;
        layout(location = 2) out vec4 outAmbient;

        uniform sampler2D diffuseTexture;
        uniform sampler2D normalTexture;
        uniform sampler2D specularTexture;
        uniform sampler2D roughnessTexture;
        uniform sampler2D heightTexture;
        uniform samplerCube environmentMap;
        uniform vec3 cameraPosition;
        uniform bool usePOM;
        uniform vec2 texCoordScale;
    "#;

    source += r#"
        in vec3 position;
        in vec3 normal;
        in vec2 texCoord;
        in vec3 tangent;
        in vec3 binormal;
    "#;

    if features.contains(UberShaderFeatures::LIGHTMAP) {
        source += r#"
        in vec2 secondTexCoord;
        uniform sampler2D lightmapTexture;
        "#;
    }

    if features.contains(UberShaderFeatures::TERRAIN) {
        source += r#"
        uniform sampler2D maskTexture;        
        "#;
    }

    if features.contains(UberShaderFeatures::INSTANCING) {
        source += r#"
        in vec4 diffuseColor;
        "#;
    } else {
        source += r#"
        uniform vec4 diffuseColor;
        "#;
    }

    source += r#"
    void main()
    {
        mat3 tangentSpace = mat3(tangent, binormal, normal);
        vec3 toFragment = normalize(position - cameraPosition);

        vec2 tc;
        if (usePOM) {
            vec3 toFragmentTangentSpace = normalize(transpose(tangentSpace) * toFragment);
            tc = S_ComputeParallaxTextureCoordinates(heightTexture, toFragmentTangentSpace, texCoord * texCoordScale, normal);
        } else {
            tc = texCoord * texCoordScale;
        }
    "#;

    if features.contains(UberShaderFeatures::TERRAIN) {
        source += r#"      
        // No alpha test, we need alpha for blending.
        outColor = diffuseColor * texture(diffuseTexture, tc);
        "#;
    } else {
        source += r#"
        outColor = diffuseColor * texture(diffuseTexture, tc);
        // Alpha test.
        if (outColor.a < 0.5) {
            discard;
        }
        outColor.a = 1.0;
        "#;
    }

    source += r#"
        outColor = diffuseColor * texture(diffuseTexture, tc);
        if (outColor.a < 0.5) {
            discard;
        }       
        vec4 n = normalize(texture(normalTexture, tc) * 2.0 - 1.0);
        outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
        outNormal.w = texture(specularTexture, tc).r;
        "#;

    if features.contains(UberShaderFeatures::LIGHTMAP) {
        source += r#"outAmbient = vec4(texture(lightmapTexture, secondTexCoord).rgb, 1.0);"#;
    } else {
        source += r#"outAmbient = vec4(0.0, 0.0, 0.0, 1.0);"#;
    }

    source += r#"
        // reflection mapping
        float roughness = texture(roughnessTexture, tc).r;
        vec3 reflectionTexCoord = reflect(toFragment, normalize(n.xyz));
        outColor = (1.0 - roughness) * outColor + roughness * vec4(texture(environmentMap, reflectionTexCoord).rgb, outColor.a);
    "#;

    if features.contains(UberShaderFeatures::TERRAIN) {
        source += r#"   
        // In case of terrain we'll use alpha for blending.   
        float mask = texture(maskTexture, texCoord).r;
        outColor.a *= mask;     
        outAmbient.a *= mask;         
        outNormal.a *= mask;        
        "#;
    }

    source += r#"
    }
    "#;

    source
}

pub struct UberShader {
    pub program: GpuProgram,
    pub use_skeletal_animation: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub specular_texture: UniformLocation,
    pub roughness_texture: UniformLocation,
    pub tex_coord_scale: UniformLocation,
    pub lightmap_texture: Option<UniformLocation>,
    pub matrix_buffer_stride: Option<UniformLocation>,
    pub matrix_storage_size: Option<UniformLocation>,
    pub matrix_storage: Option<UniformLocation>,
    pub environment_map: UniformLocation,
    pub camera_position: UniformLocation,
    pub view_projection_matrix: Option<UniformLocation>,
    pub use_pom: UniformLocation,
    pub height_texture: UniformLocation,
    // Non-instanced parts.
    pub world_matrix: Option<UniformLocation>,
    pub wvp_matrix: Option<UniformLocation>,
    pub bone_matrices: Option<UniformLocation>,
    pub diffuse_color: Option<UniformLocation>,
    // Terrain.
    pub mask_texture: Option<UniformLocation>,
}

bitflags! {
    pub struct UberShaderFeatures: u32 {
        const DEFAULT = 0;
        const INSTANCING = 0b0001;
        const LIGHTMAP = 0b0010;
        const TERRAIN = 0b0100;
        const COUNT = (1 << 3) + 1;
    }
}

fn make_name(features: UberShaderFeatures) -> String {
    let mut name = "GBuffer".to_owned();
    if features.contains(UberShaderFeatures::TERRAIN) {
        name += "Terrain";
    }
    if features.contains(UberShaderFeatures::LIGHTMAP) {
        name += "Lightmap";
    }
    if features.contains(UberShaderFeatures::INSTANCING) {
        name += "Instancing";
    }
    name += "Shader";
    name
}

impl UberShader {
    pub fn new(
        state: &mut PipelineState,
        features: UberShaderFeatures,
    ) -> Result<Self, FrameworkError> {
        let name = make_name(features);
        let fragment_source = make_fragment_shader_source(features);
        let vertex_source = make_vertex_shader_source(features);
        let program = GpuProgram::from_source(state, &name, &vertex_source, &fragment_source)?;
        let instancing = features.contains(UberShaderFeatures::INSTANCING);
        let lightmap = features.contains(UberShaderFeatures::LIGHTMAP);
        let terrain = features.contains(UberShaderFeatures::TERRAIN);
        Ok(Self {
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            tex_coord_scale: program.uniform_location(state, "texCoordScale")?,
            world_matrix: if !instancing {
                Some(program.uniform_location(state, "worldMatrix")?)
            } else {
                None
            },
            wvp_matrix: if !instancing {
                Some(program.uniform_location(state, "worldViewProjection")?)
            } else {
                None
            },
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            normal_texture: program.uniform_location(state, "normalTexture")?,
            specular_texture: program.uniform_location(state, "specularTexture")?,
            roughness_texture: program.uniform_location(state, "roughnessTexture")?,
            lightmap_texture: if lightmap {
                Some(program.uniform_location(state, "lightmapTexture")?)
            } else {
                None
            },
            matrix_buffer_stride: if instancing {
                Some(program.uniform_location(state, "matrixBufferStride")?)
            } else {
                None
            },
            matrix_storage_size: if instancing {
                Some(program.uniform_location(state, "matrixStorageSize")?)
            } else {
                None
            },
            matrix_storage: if instancing {
                Some(program.uniform_location(state, "matrixStorage")?)
            } else {
                None
            },
            environment_map: program.uniform_location(state, "environmentMap")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            view_projection_matrix: if instancing {
                Some(program.uniform_location(state, "viewProjectionMatrix")?)
            } else {
                None
            },
            use_pom: program.uniform_location(state, "usePOM")?,
            height_texture: program.uniform_location(state, "heightTexture")?,
            diffuse_color: if !instancing {
                Some(program.uniform_location(state, "diffuseColor")?)
            } else {
                None
            },
            bone_matrices: if !instancing {
                Some(program.uniform_location(state, "boneMatrices")?)
            } else {
                None
            },
            mask_texture: if terrain {
                Some(program.uniform_location(state, "maskTexture")?)
            } else {
                None
            },
            program,
        })
    }
}
