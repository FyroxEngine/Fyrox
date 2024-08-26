layout (location = 0) in vec4 vertexPosition;

uniform mat4 viewProjection;

flat out uint objectIndex;

void main()
{
    objectIndex = uint(vertexPosition.w);
    gl_Position = viewProjection * vec4(vertexPosition.xyz, 1.0);
}