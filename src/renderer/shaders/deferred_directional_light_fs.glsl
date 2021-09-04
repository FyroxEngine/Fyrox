#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D materialTexture;

uniform vec3 lightDirection;
uniform vec4 lightColor;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform float lightIntensity;

in vec2 texCoord;
out vec4 FragColor;

void main()
{
    vec3 material = texture(materialTexture, texCoord).rgb;

    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);

    TPBRContext ctx;
    ctx.albedo = texture(colorTexture, texCoord).rgb;
    ctx.fragmentToLight = lightDirection;
    ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    ctx.lightColor = lightColor.rgb;
    ctx.metallic = material.x;
    ctx.roughness = material.y;
    ctx.viewVector = normalize(cameraPosition - fragmentPosition);

    vec3 lighting = S_PBR_CalculateLight(ctx);

    FragColor = vec4(lightIntensity * lighting, 1.0);
}
