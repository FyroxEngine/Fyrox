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
    vec4 normalSpecular = texture(normalTexture, texCoord);
    TBlinnPhongContext ctx;
    ctx.lightPosition = lightPos;
    ctx.lightRadius = lightRadius;
    ctx.fragmentNormal = normalize(normalSpecular.xyz * 2.0 - 1.0);
    ctx.fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    ctx.cameraPosition = cameraPosition;
    ctx.specularPower = 80.0;
    TBlinnPhong lighting = S_BlinnPhong(ctx);

    float shadow = S_PointShadow(
        shadowsEnabled, softShadows, lighting.distance, shadowBias, lighting.direction, pointShadowTexture);

    FragColor = lighting.attenuation * shadow *
            (lightColor * lighting.specular * normalSpecular.w + lightColor * texture(colorTexture, texCoord));
}
