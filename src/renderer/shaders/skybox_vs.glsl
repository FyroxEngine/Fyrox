#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;

uniform mat4 worldViewProjection;

out vec3 texCoord;

void main()
{
    texCoord = vertexPosition;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}
