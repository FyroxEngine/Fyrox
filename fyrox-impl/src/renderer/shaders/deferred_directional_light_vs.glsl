layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

#define NUM_CASCADES 3

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 viewMatrix;
    mat4 invViewProj;
    mat4 lightViewProjMatrices[NUM_CASCADES];
    vec4 lightColor;
    vec3 lightDirection;
    vec3 cameraPosition;
    float lightIntensity;
    bool shadowsEnabled;
    float shadowBias;
    bool softShadows;
    float shadowMapInvSize;
    float cascadeDistances[NUM_CASCADES];
};

out vec2 texCoord;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    texCoord = vertexTexCoord;
}
