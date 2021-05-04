#version 330 core

layout(location = 0) in vec2 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec4 color;
layout(location = 3) in mat4 worldMatrix;

uniform mat4 viewProjection;

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldMatrix * viewProjection * vec4(vertexPosition.x, vertexPosition.y, 0.0, 1.0);
}