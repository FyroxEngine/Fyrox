#version 330 core

uniform sampler2D diffuseTexture;
uniform vec4 color;

out vec4 FragColor;

in vec2 texCoord;

void main()
{
    FragColor = color * texture(diffuseTexture, texCoord);
}