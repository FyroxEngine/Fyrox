layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;

uniform mat4 worldViewProjection;

out vec2 texCoord;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    texCoord = vertexTexCoord;
}
