uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D materialTexture;
uniform sampler2D spotShadowTexture;
uniform sampler2D cookieTexture;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    mat4 lightViewProjMatrix;
    mat4 invViewProj;
    vec3 lightPos;
    vec4 lightColor;
    vec3 cameraPosition;
    vec3 lightDirection;
    float lightRadius;
    float halfHotspotConeAngleCos;
    float halfConeAngleCos;
    float shadowMapInvSize;
    float shadowBias;
    float lightIntensity;
    float shadowAlpha;
    bool cookieEnabled;
    bool shadowsEnabled;
    bool softShadows;
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

    float spotAngleCos = dot(lightDirection, ctx.fragmentToLight);
    float coneFactor = smoothstep(halfConeAngleCos, halfHotspotConeAngleCos, spotAngleCos);

    float shadow = S_SpotShadowFactor(
        shadowsEnabled, softShadows, shadowBias, fragmentPosition,
        lightViewProjMatrix, shadowMapInvSize, spotShadowTexture);
    float finalShadow = mix(1.0, shadow, shadowAlpha);

    vec4 cookieAttenuation = vec4(1.0);
    if (cookieEnabled) {
        vec2 texCoords = S_Project(fragmentPosition, lightViewProjMatrix).xy;
        cookieAttenuation = texture(cookieTexture, texCoords);
    }

    FragColor = cookieAttenuation * vec4(distanceAttenuation * lightIntensity * coneFactor * finalShadow * lighting, diffuseColor.a);
}
