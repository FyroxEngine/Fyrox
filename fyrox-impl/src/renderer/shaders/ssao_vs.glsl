layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

#define KERNEL_SIZE 32

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 inverseProjectionMatrix;
    mat4 projectionMatrix;
    vec3 kernel[KERNEL_SIZE];
    vec2 noiseScale;
    mat3 viewMatrix;
    float radius;
};

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}