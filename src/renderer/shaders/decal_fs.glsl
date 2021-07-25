#version 330 core

uniform sampler2D sceneDepth;
uniform sampler2D diffuseTexture;
uniform sampler2D normalTexture;
uniform mat4 invViewProj;
uniform mat3 normalMatrixDecal;
uniform mat4 invWorldDecal;
uniform vec2 resolution;

layout(location = 0) out vec4 outDiffuseMap;
layout(location = 1) out vec4 outNormalMap;

in vec4 clipSpacePosition;

void main()
{
    vec2 screenPos = clipSpacePosition.xy / clipSpacePosition.w;

    vec2 texCoord = vec2(
        (1.0 + screenPos.x) / 2.0 + (0.5 / resolution.x),
        (1.0 + screenPos.y) / 2.0 + (0.5 / resolution.y)
    );

    float sceneDepth = texture(sceneDepth, texCoord).r;

    vec3 sceneWorldPosition = S_UnProject(vec3(texCoord, sceneDepth), invViewProj);

    vec3 decalSpacePosition = (invWorldDecal * vec4(sceneWorldPosition, 1.0)).xyz;

    // Check if scene pixel is not inside decal bounds.
    vec3 dpos = vec3(0.5) - abs(decalSpacePosition.xyz);
    if (dpos.x < 0.0 || dpos.y < 0.0 || dpos.z < 0.0) {
        discard;
    }

    vec2 decalTexCoord = decalSpacePosition.xz + 0.5;

    outDiffuseMap = texture(diffuseTexture, decalTexCoord);

    vec3 dFdxWp = dFdx(sceneWorldPosition);
    vec3 dFdyWp = dFdy(sceneWorldPosition);

    mat3 tangentSpace;
    tangentSpace[0] = normalize(normalMatrixDecal * dFdyWp); // Tangent
    tangentSpace[1] = normalize(normalMatrixDecal * dFdxWp); // Binormal
    tangentSpace[2] = normalize(normalMatrixDecal * cross(dFdyWp, dFdxWp)); // Normal

    outNormalMap = vec4(tangentSpace * (texture(normalTexture, decalTexCoord) * 2.0 - 1.0).xyz, 1.0);
}