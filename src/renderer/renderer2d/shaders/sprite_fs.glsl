#version 330 core

uniform sampler2D diffuseTexture;

uniform int lightCount;
uniform vec4 lightColorRadius[16]; // xyz - color, w = radius
uniform vec4 lightPositionDirection[16]; // xy = position, zw - direction
uniform vec2 lightParameters[16]; // x = hotspot angle, y - full cone angle delta
uniform vec3 ambientLightColor;

out vec4 FragColor;

in vec2 texCoord;
in vec4 color;
in vec2 fragmentPosition;

void main()
{
    vec3 lighting = ambientLightColor;
    for(int i = 0; i < lightCount; ++i) {
        // "Unpack" light parameters.
        float halfHotspotAngleCos = lightParameters[i].x;
        float halfConeAngleCos = lightParameters[i].y;
        vec3 lightColor = lightColorRadius[i].xyz;
        float radius = lightColorRadius[i].w;
        vec2 lightPosition = lightPositionDirection[i].xy;
        vec2 direction = lightPositionDirection[i].zw;

        // Calculate lighting.
        vec2 toFragment = fragmentPosition - lightPosition;
        float distance = length(toFragment);
        vec2 toFragmentNormalized = toFragment / distance;
        float distanceAttenuation = S_LightDistanceAttenuation(distance, radius);
        float spotAngleCos = dot(toFragmentNormalized, direction);
        float directionalAttenuation = smoothstep(halfConeAngleCos, halfHotspotAngleCos, spotAngleCos);
        lighting += lightColor * (distanceAttenuation * directionalAttenuation);
    }

    FragColor = vec4(lighting, 1.0) * color * texture(diffuseTexture, texCoord);
}