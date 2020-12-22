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
    TBlinnPhongContext ctx;
    ctx.lightPosition = lightPos;
    ctx.lightRadius = lightRadius;
    ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    ctx.fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    ctx.cameraPosition = cameraPosition;
    ctx.specularPower = 255.0 * texture(normalTexture, texCoord).w;
    TBlinnPhong lighting = S_BlinnPhong(ctx);

    float spotAngleCos = dot(lightDirection, lighting.direction);
    float coneFactor = smoothstep(halfConeAngleCos, halfHotspotConeAngleCos, spotAngleCos);

    float shadow = 1.0;
    if (shadowsEnabled)
    {
        vec3 lightSpacePosition = S_Project(ctx.fragmentPosition, lightViewProjMatrix);
        if (softShadows)
        {
            for (float y = -1.5; y <= 1.5; y += 0.5)
            {
                for (float x = -1.5; x <= 1.5; x += 0.5)
                {
                    vec2 fetchTexCoord = lightSpacePosition.xy + vec2(x, y) * shadowMapInvSize;
                    if (lightSpacePosition.z - shadowBias > texture(spotShadowTexture, fetchTexCoord).r)
                    {
                        shadow += 1.0;
                    }
                }
            }

            shadow = clamp(1.0 - shadow / 9.0, 0.0, 1.0);
        }
        else
        {
            if (lightSpacePosition.z - shadowBias > texture(spotShadowTexture, lightSpacePosition.xy).r)
            {
                shadow = 0.0;
            }
        }
    }

    vec4 cookieAttenuation = vec4(1.0);
    if (cookieEnabled) {
        vec2 texCoords = S_Project(ctx.fragmentPosition, lightViewProjMatrix).xy;
        cookieAttenuation = texture(cookieTexture, texCoords);
    }

    FragColor = texture(colorTexture, texCoord);
    FragColor.rgb += 0.4 * lighting.specular;
    FragColor *= cookieAttenuation * coneFactor * shadow * lighting.attenuation * lightColor;
}
