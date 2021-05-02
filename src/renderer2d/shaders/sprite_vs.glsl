#version 330 core

layout(location = 0) in vec2 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;

uniform mat4 worldViewProjection;

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition.x, vertexPosition.y, 0.0, 1.0);
}