layout(location = 0) in vec3 vertexPosition;

uniform mat4 worldViewProjection;

out vec4 clipSpacePosition;

void main()
{
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
    clipSpacePosition = gl_Position;
}