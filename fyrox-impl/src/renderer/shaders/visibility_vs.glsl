layout (location = 0) in vec3 vertexPosition;

uniform mat4 viewProjection;
uniform sampler2D instanceMatrices;

flat out int instanceId;

void main()
{
    instanceId = gl_InstanceID;
    gl_Position = (viewProjection * S_FetchMatrix(instanceMatrices, gl_InstanceID)) * vec4(vertexPosition, 1.0);
}