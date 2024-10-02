layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    vec2 pixelSize;
    bool horizontal;
};

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}