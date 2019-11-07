#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in float particleSize;
layout(location = 3) in float particleRotation;
layout(location = 4) in vec4 vertexColor;

uniform mat4 viewProjectionMatrix;
uniform mat4 worldMatrix;
uniform vec3 cameraUpVector;
uniform vec3 cameraSideVector;

out vec2 texCoord;
out vec4 color;

vec2 rotateVec2(vec2 v, float angle)
{
    float c = cos(angle);
    float s = sin(angle);
    mat2 m = mat2(c, -s, s, c);
    return m * v;
}

void main()
{
    color = vertexColor;
    texCoord = vertexTexCoord;
    vec2 vertexOffset = rotateVec2(vertexTexCoord * 2.0 - 1.0, particleRotation);
    vec4 worldPosition = worldMatrix * vec4(vertexPosition, 1.0);
    vec3 offset = (vertexOffset.x * cameraSideVector + vertexOffset.y * cameraUpVector) * particleSize;
    gl_Position = viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
}