// Shared functions for all shaders in the engine. Contents of this
// file will be *automatically* included in all shaders!

// Projects world space position (typical use case) by given matrix.
vec3 S_Project(vec3 worldPosition, mat4 matrix)
{
    vec4 screenPos = matrix * vec4(worldPosition, 1);

    screenPos.xyz /= screenPos.w;

    return screenPos.xyz * 0.5 + 0.5;
}

// Returns matrix-space position from given screen position.
// Real space of returned value is defined by matrix and can
// be any, but there are few common use cases:
//  - To get position in view space pass inverse projection matrix.
//  - To get position in world space pass inverse view-projection matrix.
vec3 S_UnProject(vec3 screenPos, mat4 matrix)
{
    vec4 clipSpacePos = vec4(screenPos * 2.0 - 1.0, 1.0);

    vec4 position = matrix * clipSpacePos;

    return position.xyz / position.w;
}

// Calculates specular factor using Blinn-Phong model.
float S_SpecularFactor(vec3 lightVector, vec3 cameraPosition, vec3 fragmentPosition, vec3 fragmentNormal, float power)
{
    vec3 v = cameraPosition - fragmentPosition;
    vec3 h = normalize(lightVector + v);
    return pow(clamp(dot(fragmentNormal, h), 0.0, 1.0), power);
}

// Blinn-Phong lighting model input parameters.
struct TBlinnPhongContext {
    // Light position in world coordinates.
    vec3 lightPosition;
    float lightRadius;
    vec3 fragmentNormal;
    // Fragment position on world coordinates.
    vec3 fragmentPosition;
    vec3 cameraPosition;
    float specularPower;
};

// Blinn-Phong lighting output parameters.
struct TBlinnPhong {
    // Total "brightness" of fragment.
    float attenuation;
    // Specular component of lighting.
    float specular;
    // Distance from light to fragment.
    float distance;
    // Normalized vector from fragment position to light.
    // It can be useful if you need this vector later on,
    // for other calculations.
    vec3 direction;
};

// Calculates lighting parameters for point light using Blinn-Phong model.
// This function also suitable for calculations of spot lighting, because
// spot light is a point light but with defined lighting cone.
TBlinnPhong S_BlinnPhong(TBlinnPhongContext ctx)
{
    vec3 lightVector = ctx.lightPosition - ctx.fragmentPosition;
    float distance = length(lightVector);
    lightVector = lightVector / distance;

    float clampedDistance = min(distance, ctx.lightRadius);

    float specular = S_SpecularFactor(lightVector, ctx.cameraPosition, ctx.fragmentPosition, ctx.fragmentNormal, ctx.specularPower);

    float lambertian = max(dot(ctx.fragmentNormal, lightVector), 0);

    float distance_attenuation = 1.0 + cos((clampedDistance / ctx.lightRadius) * 3.14159);

    float attenuation = lambertian * distance_attenuation;

    return TBlinnPhong(attenuation, specular, distance, lightVector);
}
