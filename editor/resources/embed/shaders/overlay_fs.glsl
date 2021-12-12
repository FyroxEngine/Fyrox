uniform sampler2D diffuseTexture;

out vec4 FragColor;

in vec2 texCoord;

void main()
{
    FragColor = texture(diffuseTexture, texCoord);
}