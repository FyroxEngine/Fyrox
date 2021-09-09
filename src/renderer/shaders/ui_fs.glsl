// IMPORTANT: UI is rendered in sRGB color space!

uniform sampler2D diffuseTexture;

uniform bool isFont;
uniform vec4 solidColor;
uniform float opacity;

uniform int brushType;

uniform int gradientPointCount;
uniform vec4 gradientColors[16];
uniform float gradientStops[16];

// Begin point of linear gradient *or* center of radial gradient
// in normalized coordinates
uniform vec2 gradientOrigin;

// End point of linear gradient in normalized coordinates.
uniform vec2 gradientEnd;

uniform vec2 resolution;
uniform vec2 boundsMin;
uniform vec2 boundsMax;

out vec4 fragColor;

in vec2 texCoord;
in vec4 color;

float project_point(vec2 a, vec2 b, vec2 p) {
    vec2 ab = b - a;
    return clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
}

int find_stop_index(float t) {
    int idx = 0;

    for(int i = 0; i < gradientPointCount; ++i) {
        if (t > gradientStops[i]) {
            idx = i;
        }
    }

    return idx;
}

void main()
{
    vec2 size = vec2(boundsMax.x - boundsMin.x, boundsMax.y - boundsMin.y);
    vec2 localPosition = (vec2(gl_FragCoord.x, resolution.y - gl_FragCoord.y) - boundsMin) / size;

    if (brushType == 0) {
        // Solid color
        fragColor = solidColor;
    } else {
        // Gradient brush
        float t = 0.0;

        if (brushType == 1) {
            // Linear gradient
            t = project_point(gradientOrigin, gradientEnd, localPosition);
        } else if (brushType == 2) {
            // Radial gradient
            t = clamp(length(localPosition - gradientOrigin), 0.0, 1.0);
        }

        int current = find_stop_index(t);
        int next = min(current + 1, gradientPointCount);
        float delta = gradientStops[next] - gradientStops[current];
        float mix_factor = (t - gradientStops[current]) / delta;
        fragColor = mix(gradientColors[current], gradientColors[next], mix_factor);
    }

    vec4 diffuseColor = texture(diffuseTexture, texCoord);

    if (isFont)
    {
        fragColor.a *= diffuseColor.r;
    }
    else
    {
        fragColor *= diffuseColor;
    }

    fragColor.a *= opacity;

    fragColor *= color;
}