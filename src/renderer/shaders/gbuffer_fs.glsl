#version 330 core

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outNormal;
layout(location = 2) out vec4 outAmbient;

uniform sampler2D diffuseTexture;
uniform sampler2D normalTexture;
uniform sampler2D specularTexture;
uniform sampler2D lightmapTexture;
uniform sampler2D roughnessTexture;
uniform samplerCube environmentMap;
uniform vec4 diffuseColor;
uniform vec3 cameraPosition;

in vec3 position;
in vec3 normal;
in vec2 texCoord;
in vec3 tangent;
in vec3 binormal;
in vec2 secondTexCoord;

void main()
{
    outColor = diffuseColor * texture(diffuseTexture, texCoord);
    if (outColor.a < 0.5) discard;
    outColor.a = 1;
    vec4 n = normalize(texture(normalTexture, texCoord) * 2.0 - 1.0);
    mat3 tangentSpace = mat3(tangent, binormal, normal);
    outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
    outNormal.w = texture(specularTexture, texCoord).r;
    outAmbient = vec4(texture(lightmapTexture, secondTexCoord).rgb, 1.0);

    // reflection mapping
    float roughness = texture(roughnessTexture, texCoord).r;
    vec3 reflectionTexCoord = reflect(normalize(position-cameraPosition), normalize(n.xyz));
    outColor = (1-roughness) * outColor + roughness * vec4(texture(environmentMap, reflectionTexCoord).rgb, outColor.a);
}
