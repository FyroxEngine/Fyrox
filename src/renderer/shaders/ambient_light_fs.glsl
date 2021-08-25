#version 330 core

uniform sampler2D diffuseTexture;
uniform sampler2D aoSampler;
uniform sampler2D ambientTexture;
uniform vec4 ambientColor;

out vec4 FragColor;
in vec2 texCoord;

void main()
{
    float ambientOcclusion = texture(aoSampler, texCoord).r;
    vec4 ambientPixel = texture(ambientTexture, texCoord);
    FragColor = (ambientColor + ambientPixel) * texture(diffuseTexture, texCoord);
    FragColor.rgb *= ambientOcclusion;
    FragColor.a = ambientPixel.a;

    // TODO: Implement IBL.
}