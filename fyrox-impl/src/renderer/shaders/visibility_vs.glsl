layout (location = 0) in vec3 vertexPosition;

uniform mat4 viewProjection;
uniform sampler2D matrices;

flat out uint objectIndex;

void main()
{
    objectIndex = uint(gl_InstanceID);
    gl_Position = (viewProjection * S_FetchMatrix(matrices, gl_InstanceID)) * vec4(vertexPosition, 1.0);
}