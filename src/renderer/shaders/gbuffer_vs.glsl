#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec2 vertexSecondTexCoord;
layout(location = 3) in vec3 vertexNormal;
layout(location = 4) in vec4 vertexTangent;
layout(location = 5) in vec4 boneWeights;
layout(location = 6) in vec4 boneIndices;

uniform mat4 worldMatrix;
uniform mat4 worldViewProjection;
uniform bool useSkeletalAnimation;
uniform mat4 boneMatrices[60];

out vec3 position;
out vec3 normal;
out vec2 texCoord;
out vec3 tangent;
out vec3 binormal;
out vec2 secondTexCoord;

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

        localPosition += boneMatrices[i0] * vertex * boneWeights.x;
        localPosition += boneMatrices[i1] * vertex * boneWeights.y;
        localPosition += boneMatrices[i2] * vertex * boneWeights.z;
        localPosition += boneMatrices[i3] * vertex * boneWeights.w;

        localNormal += mat3(boneMatrices[i0]) * vertexNormal * boneWeights.x;
        localNormal += mat3(boneMatrices[i1]) * vertexNormal * boneWeights.y;
        localNormal += mat3(boneMatrices[i2]) * vertexNormal * boneWeights.z;
        localNormal += mat3(boneMatrices[i3]) * vertexNormal * boneWeights.w;

        localTangent += mat3(boneMatrices[i0]) * vertexTangent.xyz * boneWeights.x;
        localTangent += mat3(boneMatrices[i1]) * vertexTangent.xyz * boneWeights.y;
        localTangent += mat3(boneMatrices[i2]) * vertexTangent.xyz * boneWeights.z;
        localTangent += mat3(boneMatrices[i3]) * vertexTangent.xyz * boneWeights.w;
    }
    else
    {
        localPosition = vec4(vertexPosition, 1.0);
        localNormal = vertexNormal;
        localTangent = vertexTangent.xyz;
    }
    gl_Position = worldViewProjection * localPosition;
    normal = normalize(mat3(worldMatrix) * localNormal);
    tangent = normalize(mat3(worldMatrix) * localTangent);
    binormal = normalize(vertexTangent.w * cross(tangent, normal));
    texCoord = vertexTexCoord;
    secondTexCoord = vertexSecondTexCoord;
    position = vec3(worldMatrix * localPosition);
}
