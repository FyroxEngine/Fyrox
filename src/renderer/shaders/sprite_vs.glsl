#version 330 core

layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;

uniform mat4 viewProjectionMatrix;
uniform mat4 worldMatrix;
uniform vec3 cameraUpVector;
uniform vec3 cameraSideVector;
uniform float size;
uniform float rotation;

out vec2 texCoord;

vec2 rotateVec2(vec2 v, float angle)
{
   float c = cos(angle);
   float s = sin(angle);
   mat2 m = mat2(c, -s, s, c);
   return m * v;
}

void main()
{
	texCoord = vertexTexCoord;
	vec2 vertexOffset = rotateVec2(vertexTexCoord * 2.0 - 1.0, rotation);
	vec4 worldPosition = worldMatrix * vec4(vertexPosition, 1.0);
	vec3 offset = (vertexOffset.x * cameraSideVector + vertexOffset.y * cameraUpVector) * size;
	gl_Position = viewProjectionMatrix * (worldPosition + vec4(offset.x, offset.y, offset.z, 0.0));
}