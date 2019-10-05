#version 330 core

uniform sampler2D diffuseTexture;

out vec4 FragColor;
in vec2 texCoord;
in vec4 color;

void main()
{
	FragColor = color;
	FragColor.a *= texture(diffuseTexture, texCoord).r;
}