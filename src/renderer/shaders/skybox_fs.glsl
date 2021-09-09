uniform samplerCube cubemapTexture;

out vec4 FragColor;

in vec3 texCoord;

void main()
{
    FragColor = S_SRGBToLinear(texture(cubemapTexture, texCoord));
}
