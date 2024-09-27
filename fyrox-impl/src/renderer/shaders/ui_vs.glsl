layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;
layout (location = 2) in vec4 vertexColor;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    vec4 solidColor;
    vec4 gradientColors[16];
    float gradientStops[16];
    // Begin point of linear gradient *or* center of radial gradient in normalized coordinates.
    vec2 gradientOrigin;
    // End point of linear gradient in normalized coordinates.
    vec2 gradientEnd;
    vec2 resolution;
    vec2 boundsMin;
    vec2 boundsMax;

    bool isFont;
    float opacity;
    int brushType;
    int gradientPointCount;
};

out vec2 texCoord;
out vec4 color;

void main()
{
    texCoord = vertexTexCoord;
    color = vertexColor;
    gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
}