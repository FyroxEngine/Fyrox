#version 330 core

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec4 outNormal;

uniform sampler2D diffuseTexture;
uniform sampler2D normalTexture;
uniform sampler2D specularTexture;
uniform vec4 diffuseColor;

in vec3 normal;
in vec2 texCoord;
in vec3 tangent;
in vec3 binormal;

void main()
{
    outColor = diffuseColor * texture2D(diffuseTexture, texCoord);
    if (outColor.a < 0.5) discard;
    outColor.a = 1;
    vec4 n = normalize(texture2D(normalTexture, texCoord) * 2.0 - 1.0);
    mat3 tangentSpace = mat3(tangent, binormal, normal);
    outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
    outNormal.w = texture2D(specularTexture, texCoord).r;
}