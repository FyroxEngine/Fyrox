layout (location = 0) in vec3 vertexPosition;

uniform mat4 viewProjection;

void main()
{
    gl_Position = viewProjection * vec4(vertexPosition, 1.0);
}