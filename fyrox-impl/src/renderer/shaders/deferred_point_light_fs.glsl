uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D materialTexture;
uniform samplerCube pointShadowTexture;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 invViewProj;
    vec4 lightColor;
    vec3 lightPos;
    vec3 cameraPosition;
    float lightRadius;
    float shadowBias;
    float lightIntensity;
    float shadowAlpha;
    bool softShadows;
    bool shadowsEnabled;
};

in vec2 texCoord;
out vec4 FragColor;

void main()
{
    vec3 material = texture(materialTexture, texCoord).rgb;

    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    vec3 fragmentToLight = lightPos - fragmentPosition;
    float distance = length(fragmentToLight);

    vec4 diffuseColor = texture(colorTexture, texCoord);

    TPBRContext ctx;
    ctx.albedo = S_SRGBToLinear(diffuseColor).rgb;
    ctx.fragmentToLight = fragmentToLight / distance;
    ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    ctx.lightColor = lightColor.rgb;
    ctx.metallic = material.x;
    ctx.roughness = material.y;
    ctx.viewVector = normalize(cameraPosition - fragmentPosition);

    vec3 lighting = S_PBR_CalculateLight(ctx);

    float distanceAttenuation = S_LightDistanceAttenuation(distance, lightRadius);

    float shadow = S_PointShadow(
        shadowsEnabled, softShadows, distance, shadowBias, ctx.fragmentToLight, pointShadowTexture);
    float finalShadow = mix(1.0, shadow, shadowAlpha);

    FragColor = vec4(lightIntensity * distanceAttenuation * finalShadow * lighting, diffuseColor.a);
}
