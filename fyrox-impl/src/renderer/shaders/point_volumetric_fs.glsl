uniform sampler2D depthSampler;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 invProj;
    // Warning! All coordinates are given in *view* space.
    vec3 lightPosition;
    vec3 lightColor;
    vec3 scatterFactor;
    float intensity;
    float lightRadius;
};

out vec4 FragColor;

in vec2 texCoord;

void main()
{
    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthSampler, texCoord).r), invProj);
    float fragmentDepth = length(fragmentPosition);
    vec3 viewDirection = fragmentPosition / fragmentDepth;

    // Find intersection
    vec3 scatter = vec3(0.0);
    float minDepth, maxDepth;
    if (S_RaySphereIntersection(vec3(0.0), viewDirection, lightPosition, lightRadius, minDepth, maxDepth))
    {
        // Perform depth test.
        if (minDepth > 0.0 || fragmentDepth > minDepth)
        {
            minDepth = max(minDepth, 0.0);
            maxDepth = clamp(maxDepth, 0.0, fragmentDepth);

            vec3 closestPoint = viewDirection * minDepth;

            scatter = scatterFactor * S_InScatter(closestPoint, viewDirection, lightPosition, maxDepth - minDepth);
        }
    }

    FragColor = vec4(lightColor * pow(clamp(intensity * scatter, 0.0, 1.0), vec3(2.2)), 1.0);
}