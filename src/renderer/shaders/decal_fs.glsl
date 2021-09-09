uniform sampler2D sceneDepth;
uniform sampler2D diffuseTexture;
uniform sampler2D normalTexture;
uniform usampler2D decalMask;
uniform mat4 invViewProj;
uniform mat4 invWorldDecal;
uniform vec2 resolution;
uniform vec4 color;
uniform uint layerIndex;

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

    uvec4 maskIndex = texture(decalMask, texCoord);

    // Masking.
    if (maskIndex.r != layerIndex) {
        discard;
    }

    float sceneDepth = texture(sceneDepth, texCoord).r;

    vec3 sceneWorldPosition = S_UnProject(vec3(texCoord, sceneDepth), invViewProj);

    vec3 decalSpacePosition = (invWorldDecal * vec4(sceneWorldPosition, 1.0)).xyz;

    // Check if scene pixel is not inside decal bounds.
    vec3 dpos = vec3(0.5) - abs(decalSpacePosition.xyz);
    if (dpos.x < 0.0 || dpos.y < 0.0 || dpos.z < 0.0) {
        discard;
    }

    vec2 decalTexCoord = decalSpacePosition.xz + 0.5;

    outDiffuseMap = color * texture(diffuseTexture, decalTexCoord);

    vec3 fragmentTangent = dFdx(sceneWorldPosition);
    vec3 fragmentBinormal = dFdy(sceneWorldPosition);
    vec3 fragmentNormal = cross(fragmentTangent, fragmentBinormal);

    mat3 tangentToWorld;
    tangentToWorld[0] = normalize(fragmentTangent); // Tangent
    tangentToWorld[1] = normalize(fragmentBinormal); // Binormal
    tangentToWorld[2] = normalize(fragmentNormal); // Normal

    vec3 rawNormal = (texture(normalTexture, decalTexCoord) * 2.0 - 1.0).xyz;
    vec3 worldSpaceNormal = tangentToWorld * rawNormal;
    outNormalMap = vec4(worldSpaceNormal * 0.5 + 0.5, outDiffuseMap.a);
}