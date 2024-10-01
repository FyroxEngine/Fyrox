layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 invProj;
    vec3 lightPosition;
    vec3 lightDirection;
    vec3 lightColor;
    vec3 scatterFactor;
    float intensity;
    float coneAngleCos;
};

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}