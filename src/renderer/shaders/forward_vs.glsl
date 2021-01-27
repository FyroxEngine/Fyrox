#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
// TODO: Put rest of vertex attributes here.
layout(location = 5) in vec4 boneWeights;
layout(location = 6) in vec4 boneIndices;

uniform mat4 worldViewProjection;
uniform bool useSkeletalAnimation;
uniform mat4 boneMatrices[60];

out vec3 position;
out vec2 texCoord;

void main()
{
    vec4 localPosition = vec4(0);
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
    }
    else
    {
        localPosition = vec4(vertexPosition, 1.0);
    }
    gl_Position = worldViewProjection * localPosition;
    texCoord = vertexTexCoord;
}
