uniform sampler2D diffuseTexture;

uniform int lightCount;
uniform vec4 lightColorRadius[16]; // xyz - color, w = radius
uniform vec3 lightPosition[16];
uniform vec3 lightDirection[16];
uniform vec2 lightParameters[16]; // x = hotspot angle, y - full cone angle delta
uniform vec3 ambientLightColor;

out vec4 FragColor;

in vec2 texCoord;
in vec4 color;
in vec3 fragmentPosition;

void main()
{
    vec3 lighting = ambientLightColor;
    for(int i = 0; i < lightCount; ++i) {
        // "Unpack" light parameters.
        float halfHotspotAngleCos = lightParameters[i].x;
        float halfConeAngleCos = lightParameters[i].y;
        vec3 lightColor = lightColorRadius[i].xyz;
        float radius = lightColorRadius[i].w;
        vec3 lightPosition = lightPosition[i];
        vec3 direction = lightDirection[i];

        // Calculate lighting.
        vec3 toFragment = fragmentPosition - lightPosition;
        float distance = length(toFragment);
        vec3 toFragmentNormalized = toFragment / distance;
        float distanceAttenuation = S_LightDistanceAttenuation(distance, radius);
        float spotAngleCos = dot(toFragmentNormalized, direction);
        float directionalAttenuation = smoothstep(halfConeAngleCos, halfHotspotAngleCos, spotAngleCos);
        lighting += lightColor * (distanceAttenuation * directionalAttenuation);
    }

    FragColor = vec4(lighting, 1.0) * color * S_SRGBToLinear(texture(diffuseTexture, texCoord));
}