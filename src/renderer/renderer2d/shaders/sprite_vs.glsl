layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec4 vertexColor;
layout(location = 3) in mat4 worldMatrix;

uniform mat4 viewProjection;

out vec2 texCoord;
out vec4 color;
out vec3 fragmentPosition;

void main()
{
    texCoord = vertexTexCoord;
    vec4 worldPosition = worldMatrix * vec4(vertexPosition, 1.0);
    fragmentPosition = worldPosition.xyz;
    gl_Position = viewProjection * worldPosition;
    color = vertexColor;
}