#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec2 vertexSecondTexCoord;
layout(location = 3) in vec3 vertexNormal;
layout(location = 4) in vec4 vertexTangent;
layout(location = 5) in vec4 boneWeights;
layout(location = 6) in vec4 boneIndices;

uniform sampler2D matrixStorage;
uniform sampler2D colorStorage;

uniform vec4 matrixStorageSize; // vec4(1/w, 1/h, w, h)
uniform vec4 colorStorageSize; // vec4(1/w, 1/h, w, h)
uniform bool useSkeletalAnimation;
uniform int matrixBufferStride;

out vec3 normal;
out vec2 texCoord;
out vec3 tangent;
out vec3 binormal;
out vec2 secondTexCoord;
out vec4 diffuseColor;

mat4 ReadMatrix(int id)
{
    float k = 4.0 * float(id);

    float x = k - matrixStorageSize.z * floor(k * matrixStorageSize.x); // k % matrixStorageWidth
    float y = k * matrixStorageSize.x; // k / matrixStorageWidth

    float ty = (y + 0.5) * matrixStorageSize.y;

    vec4 col1 = texture(matrixStorage, vec2((x + 0.5) * matrixStorageSize.x, ty));
    vec4 col2 = texture(matrixStorage, vec2((x + 1.5) * matrixStorageSize.x, ty));
    vec4 col3 = texture(matrixStorage, vec2((x + 2.5) * matrixStorageSize.x, ty));
    vec4 col4 = texture(matrixStorage, vec2((x + 3.5) * matrixStorageSize.x, ty));

    return mat4(col1, col2, col3, col4);
}

vec4 ReadColor(int id)
{
    float k = float(id);

    float x = k - colorStorageSize.z * floor(k * colorStorageSize.x); // k % colorStorageWidth
    float y = k * colorStorageSize.x; // k / colorStorageWidth

    float ty = (y + 0.5) * colorStorageSize.y;

    return texture(colorStorage, vec2((x + 0.5) * colorStorageSize.x, ty));
}

void main()
{
    vec4 localPosition = vec4(0);
    vec3 localNormal = vec3(0);
    vec3 localTangent = vec3(0);

    int matrixIndexOrigin = gl_InstanceID * matrixBufferStride;

    mat4 worldMatrix = ReadMatrix(matrixIndexOrigin);
    mat4 worldViewProjection = ReadMatrix(matrixIndexOrigin+1);

    if (useSkeletalAnimation)
    {
        vec4 vertex = vec4(vertexPosition, 1.0);

        int i0 = int(boneIndices.x);
        int i1 = int(boneIndices.y);
        int i2 = int(boneIndices.z);
        int i3 = int(boneIndices.w);

        int boneIndexOrigin = matrixIndexOrigin + 2;
        mat4 m0 = ReadMatrix(boneIndexOrigin + i0);
        mat4 m1 = ReadMatrix(boneIndexOrigin + i1);
        mat4 m2 = ReadMatrix(boneIndexOrigin + i2);
        mat4 m3 = ReadMatrix(boneIndexOrigin + i3);

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
    gl_Position = worldViewProjection * localPosition;
    normal = normalize(mat3(worldMatrix) * localNormal);
    tangent = normalize(mat3(worldMatrix) * localTangent);
    binormal = normalize(vertexTangent.w * cross(tangent, normal));
    texCoord = vertexTexCoord;
    secondTexCoord = vertexSecondTexCoord;
    diffuseColor = ReadColor(gl_InstanceID);
}