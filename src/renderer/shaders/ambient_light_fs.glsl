#version 330 core

uniform sampler2D diffuseTexture;
uniform sampler2D aoSampler;
uniform sampler2D ambientTexture;
uniform vec4 ambientColor;

out vec4 FragColor;
in vec2 texCoord;

void main()
{
    float ambientOcclusion =  texture(aoSampler, texCoord).r;
    FragColor = ambientColor * texture(diffuseTexture, texCoord) ;
    FragColor.rgb *= texture(ambientTexture, texCoord).rgb;
    FragColor.rgb *= ambientOcclusion;
}