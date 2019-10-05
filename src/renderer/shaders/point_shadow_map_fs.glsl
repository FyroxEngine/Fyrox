#version 330 core

uniform sampler2D diffuseTexture;
uniform vec3 lightPosition;

in vec2 texCoord;
in vec3 worldPosition;

layout(location = 0) out float depth;

void main() 
{
   if(texture(diffuseTexture, texCoord).a < 0.2) discard;
   depth = length(lightPosition - worldPosition);
}