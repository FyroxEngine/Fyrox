#version 330 core

uniform sampler2D diffuseTexture;
uniform sampler2D depthBufferTexture;
uniform vec2 invScreenSize;
uniform vec2 projParams;

out vec4 FragColor;
in vec2 texCoord;
in vec4 color;

float toProjSpace(float z)
{
    float far = projParams.x;
    float near = projParams.y;
    return (far * near) / (far - z * (far + near));
}

void main()
{
    float sceneDepth = toProjSpace(texture(depthBufferTexture, gl_FragCoord.xy * invScreenSize).r);
    float depthOpacity = clamp((sceneDepth - gl_FragCoord.z / gl_FragCoord.w) * 2.0f, 0.0, 1.0);
    FragColor = color * texture(diffuseTexture, texCoord).r;
    FragColor.a *= depthOpacity;
}