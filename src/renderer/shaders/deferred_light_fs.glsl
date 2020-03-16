#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D spotShadowTexture;
uniform samplerCube pointShadowTexture;

uniform mat4 lightViewProjMatrix;
uniform vec3 lightPos;
uniform float lightRadius;
uniform vec4 lightColor;
uniform vec3 lightDirection;
uniform float halfHotspotConeAngleCos;
uniform float halfConeAngleCos;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform int lightType;
uniform bool softShadows;
uniform float shadowMapInvSize;

in vec2 texCoord;
out vec4 FragColor;

vec3 GetProjection(vec3 worldPosition, mat4 viewProjectionMatrix)
{
    vec4 projPos = viewProjectionMatrix * vec4(worldPosition, 1);
    projPos /= projPos.w;
    return vec3(projPos.x * 0.5 + 0.5, projPos.y * 0.5 + 0.5, projPos.z * 0.5 + 0.5);
}

void main()
{
    vec4 normalSpecular = texture2D(normalTexture, texCoord);
    vec3 normal = normalize(normalSpecular.xyz * 2.0 - 1.0);

    vec4 screenPosition;
    screenPosition.x = texCoord.x * 2.0 - 1.0;
    screenPosition.y = texCoord.y * 2.0 - 1.0;
    screenPosition.z = texture2D(depthTexture, texCoord).r;
    screenPosition.w = 1.0;

    vec4 worldPosition = invViewProj * screenPosition;
    worldPosition /= worldPosition.w;

    vec3 lightVector = lightPos - worldPosition.xyz;
    float distanceToLight = length(lightVector);
    float d = min(distanceToLight, lightRadius);
    vec3 normLightVector = lightVector / d;
    vec3 h = normalize(lightVector + (cameraPosition - worldPosition.xyz));
    vec3 specular = normalSpecular.w * vec3(0.4 * pow(clamp(dot(normal, h), 0.0, 1.0), 80));

    float k = max(dot(normal, normLightVector), 0);
    float attenuation = 1.0 + cos((d / lightRadius) * 3.14159);

    float spotAngleCos = dot(lightDirection, normLightVector);

    attenuation *= smoothstep(halfConeAngleCos, halfHotspotConeAngleCos, spotAngleCos);

    float shadow = 1.0;
    if (lightType == 2) /* Spot light shadows */
    {
        vec3 lightSpacePosition = GetProjection(worldPosition.xyz, lightViewProjMatrix);
        const float bias = 0.00005;
        if (softShadows)
        {
            for (float y = -1.5; y <= 1.5; y += 0.5)
            {
                for (float x = -1.5; x <= 1.5; x += 0.5)
                {
                    vec2 fetchTexCoord = lightSpacePosition.xy + vec2(x, y) * shadowMapInvSize;
                    if (lightSpacePosition.z - bias > texture(spotShadowTexture, fetchTexCoord).r)
                    {
                        shadow += 1.0;
                    }
                }
            }

            shadow = clamp(1.0 - shadow / 9.0, 0.0, 1.0);
        }
        else
        {
            if (lightSpacePosition.z - bias > texture(spotShadowTexture, lightSpacePosition.xy).r)
            {
                shadow = 0.0;
            }
        }
    }
    else if (lightType == 0) /* Point light shadows */
    {
        const float bias = 0.01;
        if (softShadows)
        {
            const int samples = 20;

            const vec3 directions[samples] = vec3[samples] (
            vec3(1, 1, 1), vec3(1, -1, 1), vec3(-1, -1, 1), vec3(-1, 1, 1),
            vec3(1, 1, -1), vec3(1, -1, -1), vec3(-1, -1, -1), vec3(-1, 1, -1),
            vec3(1, 1, 0), vec3(1, -1, 0), vec3(-1, -1, 0), vec3(-1, 1, 0),
            vec3(1, 0, 1), vec3(-1, 0, 1), vec3(1, 0, -1), vec3(-1, 0, -1),
            vec3(0, 1, 1), vec3(0, -1, 1), vec3(0, -1, -1), vec3(0, 1, -1)
            );

            const float diskRadius = 0.0025;

            for (int i = 0; i < samples; ++i)
            {
                vec3 fetchDirection = -normLightVector + directions[i] * diskRadius;
                float shadowDistanceToLight = texture(pointShadowTexture, fetchDirection).r;
                if (distanceToLight - bias > shadowDistanceToLight)
                {
                    shadow += 1.0;
                }
            }

            shadow = clamp(1.0 - shadow / float(samples), 0.0, 1.0);
        }
        else
        {
            float shadowDistanceToLight = texture(pointShadowTexture, -normLightVector).r;
            if (distanceToLight - bias > shadowDistanceToLight)
            {
                shadow = 0.0;
            }
        }
    }

    FragColor = texture2D(colorTexture, texCoord);
    FragColor.xyz += specular;
    FragColor *= k * shadow * attenuation * lightColor;
}