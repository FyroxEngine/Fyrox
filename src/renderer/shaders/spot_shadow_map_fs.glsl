#version 330 core

uniform sampler2D diffuseTexture;

in vec2 texCoord;

void main()
{
   if(texture(diffuseTexture, texCoord).a < 0.2) discard;
}