layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 lightViewProjMatrix;
    mat4 invViewProj;
    vec3 lightPos;
    vec4 lightColor;
    vec3 cameraPosition;
    vec3 lightDirection;
    float lightRadius;
    float halfHotspotConeAngleCos;
    float halfConeAngleCos;
    float shadowMapInvSize;
    float shadowBias;
    float lightIntensity;
    float shadowAlpha;
    bool cookieEnabled;
    bool shadowsEnabled;
    bool softShadows;
};

out vec2 texCoord;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    texCoord = vertexTexCoord;
}
