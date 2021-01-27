#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D spotShadowTexture;
uniform sampler2D cookieTexture;

uniform mat4 lightViewProjMatrix;
uniform vec3 lightPos;
uniform float lightRadius;
uniform vec4 lightColor;
uniform vec3 lightDirection;
uniform float halfHotspotConeAngleCos;
uniform float halfConeAngleCos;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform bool shadowsEnabled;
uniform bool softShadows;
uniform float shadowMapInvSize;
uniform float shadowBias;
uniform bool cookieEnabled;

in vec2 texCoord;
out vec4 FragColor;

void main()
{
    vec4 normalSpecular = texture(normalTexture, texCoord);

    TBlinnPhongContext ctx;
    ctx.lightPosition = lightPos;
    ctx.lightRadius = lightRadius;
    ctx.fragmentNormal = normalize(normalSpecular.xyz * 2.0 - 1.0);
    ctx.fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    ctx.cameraPosition = cameraPosition;
    ctx.specularPower = 80.0;
    TBlinnPhong lighting = S_BlinnPhong(ctx);

    float spotAngleCos = dot(lightDirection, lighting.direction);
    float coneFactor = smoothstep(halfConeAngleCos, halfHotspotConeAngleCos, spotAngleCos);

    float shadow = S_SpotShadowFactor(
        shadowsEnabled, softShadows, shadowBias, ctx.fragmentPosition,
            lightViewProjMatrix, shadowMapInvSize, spotShadowTexture);

    vec4 cookieAttenuation = vec4(1.0);
    if (cookieEnabled) {
        vec2 texCoords = S_Project(ctx.fragmentPosition, lightViewProjMatrix).xy;
        cookieAttenuation = texture(cookieTexture, texCoords);
    }

    FragColor = cookieAttenuation * coneFactor * lighting.attenuation * shadow *
        (lightColor * lighting.specular * normalSpecular.w + lightColor * texture(colorTexture, texCoord));
}
