#version 330 core

uniform samplerCube cubemapTexture;

out vec4 FragColor;

in vec3 texCoord;

void main()
{
    FragColor = texture(cubemapTexture, texCoord);
}
