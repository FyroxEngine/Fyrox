layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec4 vertexColor;

uniform mat4 worldViewProjection;

out vec2 texCoord;
out vec4 color;

void main()
{
    texCoord = vertexTexCoord;
    color = vertexColor;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}