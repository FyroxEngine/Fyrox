// Simple 4x4 box blur.

uniform sampler2D inputTexture;

out float FragColor;

in vec2 texCoord;

void main()
{
    vec2 texelSize = 1.0 / vec2(textureSize(inputTexture, 0));
    float result = 0.0;
    for (int y = -2; y < 2; ++y)
    {
        for (int x = -2; x < 2; ++x)
        {
            vec2 offset = vec2(float(x), float(y)) * texelSize;
            result += texture(inputTexture, texCoord + offset).r;
        }
    }
    FragColor = result / 16.0;
}