#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform samplerCube pointShadowTexture;

uniform vec3 lightPos;
uniform float lightRadius;
uniform vec4 lightColor;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform bool softShadows;
uniform bool shadowsEnabled;
uniform float shadowBias;

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
    ctx.specularPower = 80.0;
    TBlinnPhong lighting = S_BlinnPhong(ctx);

    float shadow = 1.0;

    if (shadowsEnabled)
    {
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
                vec3 fetchDirection = -lighting.direction + directions[i] * diskRadius;
                float shadowDistanceToLight = texture(pointShadowTexture, fetchDirection).r;
                if (lighting.distance - shadowBias > shadowDistanceToLight)
                {
                    shadow += 1.0;
                }
            }

            shadow = clamp(1.0 - shadow / float(samples), 0.0, 1.0);
        }
        else
        {
            float shadowDistanceToLight = texture(pointShadowTexture, -lighting.direction).r;
            if (lighting.distance - shadowBias > shadowDistanceToLight)
            {
                shadow = 0.0;
            }
        }
    }

    FragColor = texture(colorTexture, texCoord);
    FragColor.xyz += 0.4 * lighting.specular;
    FragColor *= lighting.attenuation * shadow * lightColor;
}
