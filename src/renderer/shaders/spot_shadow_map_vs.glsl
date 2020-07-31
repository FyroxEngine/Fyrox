#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 5) in vec4 boneWeights;
layout(location = 6) in vec4 boneIndices;

uniform mat4 worldViewProjection;
uniform bool useSkeletalAnimation;
uniform mat4 boneMatrices[60];

out vec2 texCoord;

void main()
{
    vec4 localPosition = vec4(0);

    if (useSkeletalAnimation)
    {
        vec4 vertex = vec4(vertexPosition, 1.0);

        localPosition += boneMatrices[int(boneIndices.x)] * vertex * boneWeights.x;
        localPosition += boneMatrices[int(boneIndices.y)] * vertex * boneWeights.y;
        localPosition += boneMatrices[int(boneIndices.z)] * vertex * boneWeights.z;
        localPosition += boneMatrices[int(boneIndices.w)] * vertex * boneWeights.w;
    }
    else
    {
        localPosition = vec4(vertexPosition, 1.0);
    }

    gl_Position = worldViewProjection * localPosition;
    texCoord = vertexTexCoord;
}