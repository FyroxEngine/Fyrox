#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform samplerCube pointShadowTexture;

uniform vec3 lightDirection;
uniform vec4 lightColor;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;

in vec2 texCoord;
out vec4 FragColor;

void main()
{
    vec3 fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    float specularPower = 255.0 * texture(normalTexture, texCoord).w;

    vec3 h = normalize(lightDirection + (cameraPosition - fragmentPosition));
    float specular = pow(clamp(dot(fragmentNormal, h), 0.0, 1.0), specularPower);

    float lambertian = max(dot(fragmentNormal, lightDirection), 0);

    FragColor = texture(colorTexture, texCoord);
    FragColor.xyz += 0.4 * specular;
    FragColor *= lambertian * lightColor;
}
