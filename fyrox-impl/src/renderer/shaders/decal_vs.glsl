layout (location = 0) in vec3 vertexPosition;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 invViewProj;
    mat4 invWorldDecal;
    vec2 resolution;
    vec4 color;
    uint layerIndex;
};

out vec4 clipSpacePosition;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    clipSpacePosition = gl_Position;
}