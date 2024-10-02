layout (location = 0) in vec3 vertexPosition;

uniform sampler2D matrices;

layout (std140) uniform Uniforms {
    mat4 viewProjection;
    int tileSize;
    float frameBufferHeight;
};

flat out uint objectIndex;

void main()
{
    objectIndex = uint(gl_InstanceID);
    gl_Position = (viewProjection * S_FetchMatrix(matrices, gl_InstanceID)) * vec4(vertexPosition, 1.0);
}