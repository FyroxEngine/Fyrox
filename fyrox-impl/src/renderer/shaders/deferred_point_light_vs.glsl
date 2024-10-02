layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 invViewProj;
    vec4 lightColor;
    vec3 lightPos;
    vec3 cameraPosition;
    float lightRadius;
    float shadowBias;
    float lightIntensity;
    float shadowAlpha;
    bool softShadows;
    bool shadowsEnabled;
};

out vec2 texCoord;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    texCoord = vertexTexCoord;
}
