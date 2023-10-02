uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D materialTexture;

uniform vec3 lightDirection;
uniform vec4 lightColor;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform float lightIntensity;
uniform mat4 viewMatrix;

#define NUM_CASCADES 3

uniform float cascadeDistances[NUM_CASCADES];
uniform mat4 lightViewProjMatrices[NUM_CASCADES];

uniform sampler2D shadowCascade0;
uniform sampler2D shadowCascade1;
uniform sampler2D shadowCascade2;

uniform bool shadowsEnabled;
uniform float shadowBias;
uniform bool softShadows;
uniform float shadowMapInvSize;

in vec2 texCoord;
out vec4 FragColor;

// Returns **inverted** shadow factor where 1 - fully bright, 0 - fully in shadow.
float CsmGetShadow(in sampler2D sampler, in vec3 fragmentPosition, in mat4 lightViewProjMatrix)
{
    return S_SpotShadowFactor(shadowsEnabled, softShadows, shadowBias, fragmentPosition, lightViewProjMatrix, shadowMapInvSize, sampler);
}

void main()
{
    vec3 material = texture(materialTexture, texCoord).rgb;

    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    vec4 diffuseColor = texture(colorTexture, texCoord);

    TPBRContext ctx;
    ctx.albedo = S_SRGBToLinear(diffuseColor).rgb;
    ctx.fragmentToLight = lightDirection;
    ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    ctx.lightColor = lightColor.rgb;
    ctx.metallic = material.x;
    ctx.roughness = material.y;
    ctx.viewVector = normalize(cameraPosition - fragmentPosition);

    vec3 lighting = S_PBR_CalculateLight(ctx);

    float fragmentZViewSpace = abs((viewMatrix * vec4(fragmentPosition, 1.0)).z);

    float shadow = 1.0;
    if (fragmentZViewSpace <= cascadeDistances[0]) {
        shadow = CsmGetShadow(shadowCascade0, fragmentPosition, lightViewProjMatrices[0]);
    } else if (fragmentZViewSpace <= cascadeDistances[1]) {
        shadow = CsmGetShadow(shadowCascade1, fragmentPosition, lightViewProjMatrices[1]);
    } else if (fragmentZViewSpace <= cascadeDistances[2]) {
        shadow = CsmGetShadow(shadowCascade2, fragmentPosition, lightViewProjMatrices[2]);
    }

    FragColor = shadow * vec4(lightIntensity * lighting, diffuseColor.a);
}
