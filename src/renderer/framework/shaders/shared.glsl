// Shared functions for all shaders in the engine. Contents of this
// file will be *automatically* included in all shaders!

const float PI = 3.14159;

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
    return attenuation;
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

float S_DistributionGGX(vec3 N, vec3 H, float roughness)
{
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float nom = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return nom / denom;
}

float S_GeometrySchlickGGX(float NdotV, float roughness)
{
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;

    float nom = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return nom / denom;
}

// Calculates occlusion factor using given material properties (normal + roughness),
// viewer position (V) and light-to-fragment vector (L).
float S_GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness)
{
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = S_GeometrySchlickGGX(NdotV, roughness);
    float ggx1 = S_GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

// Fresnel law approximation using Fresnel-Schlick formula.
vec3 S_FresnelSchlick(float cosTheta, vec3 F0)
{
    return F0 + (1.0 - F0) * pow(max(1.0 - cosTheta, 0.0), 5.0);
}

struct TPBRContext {
    vec3 lightColor;
    vec3 viewVector;
    vec3 fragmentToLight;
    vec3 fragmentNormal;
    float metallic;
    float roughness;
    vec3 albedo;
};

// Calculates physically-correct lighting using provided light and fragment parameters.
// Does not apply any distance or direction attenuation! Attenuation depends on the
// light source and appied in separate shaders.
vec3 S_PBR_CalculateLight(TPBRContext ctx) {
    vec3 F0 = mix(vec3(0.04), ctx.albedo, ctx.metallic);

    vec3 L = ctx.fragmentToLight;
    vec3 H = normalize(ctx.viewVector + L);

    // Cook-Torrance BRDF
    float NDF = S_DistributionGGX(ctx.fragmentNormal, H, ctx.roughness);
    float G = S_GeometrySmith(ctx.fragmentNormal, ctx.viewVector, L, ctx.roughness);
    vec3 F = S_FresnelSchlick(max(dot(H, ctx.viewVector), 0.0), F0);

    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(ctx.fragmentNormal, ctx.viewVector), 0.0) * max(dot(ctx.fragmentNormal, L), 0.0) + 0.001; // 0.001 to prevent divide by zero.
    vec3 specular = numerator / denominator;

    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - ctx.metallic;

    float NdotL = max(dot(ctx.fragmentNormal, L), 0.0);

    return (kD * ctx.albedo / PI + specular) * ctx.lightColor * NdotL;
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

float Internal_FetchHeight(in sampler2D heightTexture, vec2 texCoords, float center) {
    return clamp(texture(heightTexture, texCoords).r - center, 0.0, 1.0);
}

vec2 S_ComputeParallaxTextureCoordinates(in sampler2D heightTexture, vec3 eyeVec, vec2 texCoords, float center, float scale) {
    const float minLayers = 8.0;
    const float maxLayers = 15.0;
    const int maxIterations = 15;

    float t = max(0.0, abs(dot(vec3(0.0, 0.0, 1.0), eyeVec)));
    float numLayers = mix(maxLayers, minLayers, t);
    float layerDepth = 1.0 / numLayers;
    float currentLayerDepth = 0.0;

    vec2 deltaTexCoords = scale * eyeVec.xy / numLayers;

    vec2 currentTexCoords = texCoords;
    float currentDepthMapValue = Internal_FetchHeight(heightTexture, currentTexCoords, center);

    for (int i = 0; i < maxIterations; i++) {
        if (currentLayerDepth < currentDepthMapValue) {
            currentTexCoords -= deltaTexCoords;
            currentDepthMapValue = Internal_FetchHeight(heightTexture, currentTexCoords, center);
            currentLayerDepth += layerDepth;
        } else {
            break;
        }
    }

    vec2 prev = currentTexCoords + deltaTexCoords;
    float nextH = currentDepthMapValue - currentLayerDepth;
    float prevH = Internal_FetchHeight(heightTexture, prev, center) - currentLayerDepth + layerDepth;

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

float S_Luminance(vec3 x) {
    return dot(x, vec3(0.299, 0.587, 0.114));
}

ivec2 S_LinearIndexToPosition(int index, int textureWidth) {
    int y = index / textureWidth;
    int x = index - textureWidth * y; // index % textureWidth
    return ivec2(x, y);
}

mat4 S_FetchMatrix(in sampler2D storage, int index) {
    int textureWidth = textureSize(storage, 0).x;
    ivec2 pos = S_LinearIndexToPosition(4 * index, textureWidth);

    vec4 col1 = texelFetch(storage, pos, 0);
    vec4 col2 = texelFetch(storage, ivec2(pos.x + 1, pos.y), 0);
    vec4 col3 = texelFetch(storage, ivec2(pos.x + 2, pos.y), 0);
    vec4 col4 = texelFetch(storage, ivec2(pos.x + 3, pos.y), 0);

    return mat4(col1, col2, col3, col4);
}

struct TBlendShapeOffsets {
    vec3 position;
    vec3 normal;
    vec3 tangent;
};

TBlendShapeOffsets S_FetchBlendShapeOffsets(in sampler3D storage, int vertexIndex, int blendShapeIndex) {
    int textureWidth = textureSize(storage, 0).x;
    ivec3 pos = ivec3(S_LinearIndexToPosition(3 * vertexIndex, textureWidth), blendShapeIndex);
    vec3 position = texelFetch(storage, pos, 0).xyz;
    vec3 normal = texelFetch(storage, ivec3(pos.x + 1, pos.y, pos.z), 0).xyz;
    vec3 tangent = texelFetch(storage, ivec3(pos.x + 2, pos.y, pos.z), 0).xyz;
    return TBlendShapeOffsets(position, normal, tangent);
}