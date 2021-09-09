layout(location = 0) in vec2 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec4 vertexColor;
layout(location = 3) in mat4 worldMatrix;

uniform mat4 viewProjection;

out vec2 texCoord;
out vec4 color;
out vec2 fragmentPosition;

void main()
{
    texCoord = vertexTexCoord;
    vec4 worldPosition = worldMatrix * vec4(vertexPosition.x, vertexPosition.y, 0.0, 1.0);
    fragmentPosition = worldPosition.xy;
    gl_Position = viewProjection * worldPosition;
    color = vertexColor;
}