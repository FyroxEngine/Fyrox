uniform sampler2D depthSampler;
// Warning! All coordinates are given in *view* space.
uniform vec3 lightPosition;
uniform vec3 lightDirection;
uniform float coneAngleCos;
uniform mat4 invProj;
uniform vec3 lightColor;
uniform vec3 scatterFactor;
uniform float intensity;

out vec4 FragColor;

in vec2 texCoord;

void main()
{
    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthSampler, texCoord).r), invProj);
    float fragmentDepth = length(fragmentPosition);
    vec3 viewDirection = fragmentPosition / fragmentDepth;

    // Ray-cone intersection
    float sqrConeAngleCos = coneAngleCos * coneAngleCos;
    vec3  CO = -lightPosition;
    float DdotV = dot(viewDirection, lightDirection);
    float COdotV = dot(CO, lightDirection);
    float a = DdotV * DdotV - sqrConeAngleCos;
    float b = 2.0 * (DdotV * COdotV - dot(viewDirection, CO) * sqrConeAngleCos);
    float c = COdotV * COdotV - dot(CO, CO) * sqrConeAngleCos;

    // Find intersection
    vec3 scatter = vec3(0.0);
    float minDepth, maxDepth;
    if (S_SolveQuadraticEq(a, b, c, minDepth, maxDepth))
    {
        float dt1 = dot((minDepth * viewDirection) - lightPosition, lightDirection);
        float dt2 = dot((maxDepth * viewDirection) - lightPosition, lightDirection);

        // Discard points on reflected cylinder and perform depth test.
        if ((dt1 > 0.0 || dt2 > 0.0) && (minDepth > 0.0 || fragmentDepth > minDepth))
        {
            if (dt1 > 0.0 && dt2 < 0.0)
            {
                // Closest point is on cylinder, farthest on reflected.
                maxDepth = minDepth;
                minDepth = 0.0;
            }
            else if (dt1 < 0.0 && dt2 > 0.0)
            {
                // Farthest point is on cylinder, closest on reflected.
                minDepth = maxDepth;
                maxDepth = fragmentDepth;
            }

            minDepth = max(minDepth, 0.0);
            maxDepth = clamp(maxDepth, 0.0, fragmentDepth);

            scatter = scatterFactor * S_InScatter(viewDirection * minDepth, viewDirection, lightPosition, maxDepth - minDepth);
        }
    }

    FragColor = vec4(lightColor * pow(clamp(intensity * scatter, 0.0, 1.0), vec3(2.2)), 1.0);
}