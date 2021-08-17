// Shared functions for all shaders in the engine. Contents of this
// file will be *automatically* included in all shaders!

// Tries to solve quadratic equation. Returns true iff there are any real roots.
bool S_SolveQuadraticEq(float a, float b, float c, out float minT, out float maxT)
{
    float twoA = 2.0 * a;
    float det = b * b - 2.0 * twoA * c;

    if (det < 0.0)
    {
        minT = 0.0;
        maxT = 0.0;

        return false;
    }

    float sqrtDet = sqrt(det);

    float root1 = (-b - sqrtDet) / twoA;
    float root2 = (-b + sqrtDet) / twoA;

    minT = min(root1, root2);
    maxT = max(root1, root2);

    return true;
}

// Returns attenuation in inverse square model. It falls to zero at given radius.
float S_LightDistanceAttenuation(float distance, float radius)
{
    float attenuation = clamp(1.0 - distance * distance / (radius * radius), 0.0, 1.0);
    return attenuation * attenuation;
}

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

    float specular = S_SpecularFactor(lightVector, ctx.cameraPosition, ctx.fragmentPosition, ctx.fragmentNormal, ctx.specularPower);

    float lambertian = max(dot(ctx.fragmentNormal, lightVector), 0.0);

    float distance_attenuation = S_LightDistanceAttenuation(distance, ctx.lightRadius);

    float attenuation = lambertian * distance_attenuation;

    return TBlinnPhong(attenuation, specular, distance, lightVector);
}

// Returns scatter amount for given parameters.
// https://cseweb.ucsd.edu/~ravir/papers/singlescat/scattering.pdf
// https://blog.mmacklin.com/2010/05/29/in-scattering-demo/
float S_InScatter(vec3 start, vec3 dir, vec3 lightPos, float d)
{
    // light to ray origin
    vec3 q = start - lightPos;

    // coefficients
    float b = dot(dir, q);
    float c = dot(q, q);

    // evaluate integral
    float s = 1.0 / sqrt(c - b*b);
    float l = s * (atan((d + b) * s) - atan(b*s));

    return l;
}

// https://en.wikipedia.org/wiki/Rayleigh_scattering
vec3 S_RayleighScatter(vec3 start, vec3 dir, vec3 lightPos, float d)
{
    float scatter = S_InScatter(start, dir, lightPos, d);

    // Apply simple version of Rayleigh scattering. Just increase
    // intensity of blue light over other colors.
    return vec3(0.55, 0.75, 1.0) * scatter;
}

// Tries to find intersection of given ray with specified sphere. If there is an intersection, returns true.
// In out parameters minT, maxT will be min and max ray parameters of intersection.
bool S_RaySphereIntersection(vec3 origin, vec3 dir, vec3 center, float radius, out float minT, out float maxT)
{
    vec3 d = origin - center;
    float a = dot(dir, dir);
    float b = 2.0 * dot(dir, d);
    float c = dot(d, d) - radius * radius;
    return S_SolveQuadraticEq(a, b, c, minT, maxT);
}

// Calculates point shadow factor where 1.0 - no shadow, 0.0 - fully in shadow.
// Why value is inversed? To be able to directly multiply color to shadow factor.
float S_PointShadow(
    bool shadowsEnabled,
    bool softShadows,
    float fragmentDistance,
    float shadowBias,
    vec3 toLight,
    in samplerCube shadowMap)
{
    if (shadowsEnabled)
    {
        float biasedFragmentDistance = fragmentDistance - shadowBias;

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

            float accumulator = 0.0;

            for (int i = 0; i < samples; ++i)
            {
                vec3 fetchDirection = -toLight + directions[i] * diskRadius;
                float shadowDistanceToLight = texture(shadowMap, fetchDirection).r;
                if (biasedFragmentDistance > shadowDistanceToLight)
                {
                    accumulator += 1.0;
                }
            }

            return clamp(1.0 - accumulator / float(samples), 0.0, 1.0);
        }
        else
        {
            float shadowDistanceToLight = texture(shadowMap, -toLight).r;
            return biasedFragmentDistance > shadowDistanceToLight ? 0.0 : 1.0;
        }
    } else {
        return 1.0; // No shadow
    }
}

// Calculates spot light shadow factor where 1.0 - no shadow, 0.0 - fully in shadow.
// Why value is inversed? To be able to directly multiply color to shadow factor.
float S_SpotShadowFactor(
    bool shadowsEnabled,
    bool softShadows,
    float shadowBias,
    vec3 fragmentPosition,
    mat4 lightViewProjMatrix,
    float shadowMapInvSize,
    in sampler2D spotShadowTexture)
{
    if (shadowsEnabled)
    {
        vec3 lightSpacePosition = S_Project(fragmentPosition, lightViewProjMatrix);

        float biasedLightSpaceFragmentDepth = lightSpacePosition.z - shadowBias;

        if (softShadows)
        {
            float accumulator = 0.0;

            for (float y = -0.5; y <= 0.5; y += 0.5)
            {
                for (float x = -0.5; x <= 0.5; x += 0.5)
                {
                    vec2 fetchTexCoord = lightSpacePosition.xy + vec2(x, y) * shadowMapInvSize;
                    if (biasedLightSpaceFragmentDepth > texture(spotShadowTexture, fetchTexCoord).r)
                    {
                        accumulator += 1.0;
                    }
                }
            }

            return clamp(1.0 - accumulator / 9.0, 0.0, 1.0);
        }
        else
        {
            return biasedLightSpaceFragmentDepth > texture(spotShadowTexture, lightSpacePosition.xy).r ? 0.0 : 1.0;
        }
    } else {
        return 1.0; // No shadow
    }
}

float Internal_FetchHeight(in sampler2D heightTexture, vec2 texCoords) {
    return texture(heightTexture, texCoords).r;
}

vec2 S_ComputeParallaxTextureCoordinates(in sampler2D heightTexture, vec3 eyeVec, vec2 texCoords, vec3 normal) {
    const float minLayers = 8.0;
    const float maxLayers = 15.0;
    const int maxIterations = 15;
    const float parallaxScale = 0.05;

    float numLayers = mix(maxLayers, minLayers, abs(dot(normal, eyeVec)));

    float layerHeight = 1.0 / numLayers;
    float curLayerHeight = 0.0;
    vec2 dtex = parallaxScale * eyeVec.xy / numLayers;

    vec2 currentTexCoords = texCoords;

    float height = Internal_FetchHeight(heightTexture, currentTexCoords);

    for (int i = 0; i < maxIterations; i++) {
        if (height > curLayerHeight) {
            curLayerHeight += layerHeight;
            currentTexCoords -= dtex;
            height = Internal_FetchHeight(heightTexture, currentTexCoords);
        } else {
            break;
        }
    }

    vec2 prev = currentTexCoords + dtex;
    float nextH = height - curLayerHeight;
    float prevH = Internal_FetchHeight(heightTexture, prev) - curLayerHeight + layerHeight;

    float weight = nextH / (nextH - prevH);

    return prev * weight + currentTexCoords * (1.0 - weight);
}

vec4 S_LinearToSRGB(vec4 color) {
    vec3 a = 12.92 * color.rgb;
    vec3 b = 1.055 * pow(color.rgb, vec3(1.0 / 2.4)) - 0.055;
    vec3 c = step(vec3(0.0031308), color.rgb);
    vec3 rgb = mix(a, b, c);
    return vec4(rgb, color.a);
}

vec4 S_SRGBToLinear(vec4 color) {
    vec3 a = color.rgb / 12.92;
    vec3 b = pow((color.rgb + 0.055) / 1.055, vec3(2.4));
    vec3 c = step(vec3(0.04045), color.rgb);
    vec3 rgb = mix(a, b, c);
    return vec4(rgb, color.a);
}