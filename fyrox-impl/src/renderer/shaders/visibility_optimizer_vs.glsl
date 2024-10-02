layout (location = 0) in vec3 vertexPosition;

layout (std140) uniform Uniforms {
    mat4 viewProjection;
    int tileSize;
};

void main()
{
    gl_Position = viewProjection * vec4(vertexPosition, 1.0);
}