#version 330 core

uniform sampler2D diffuseTexture;
uniform bool isFont;

out vec4 FragColor;
in vec2 texCoord;
in vec4 color;

void main()
{
	if (isFont)
	{
		FragColor = color;
		FragColor.a *= texture(diffuseTexture, texCoord).r;
	}
	else
	{
		FragColor = color * texture(diffuseTexture, texCoord);
	}
}