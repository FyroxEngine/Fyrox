// Shared functions for all shaders in the engine.
// WGSL version for wgpu backend.

const PI: f32 = 3.14159;

fn inverse_mat4(m: mat4x4f) -> mat4x4f {
    let m00 = m[0][0]; let m01 = m[0][1]; let m02 = m[0][2]; let m03 = m[0][3];
    let m10 = m[1][0]; let m11 = m[1][1]; let m12 = m[1][2]; let m13 = m[1][3];
    let m20 = m[2][0]; let m21 = m[2][1]; let m22 = m[2][2]; let m23 = m[2][3];
    let m30 = m[3][0]; let m31 = m[3][1]; let m32 = m[3][2]; let m33 = m[3][3];

    let b00 = m00 * m11 - m01 * m10;
    let b01 = m00 * m12 - m02 * m10;
    let b02 = m00 * m13 - m03 * m10;
    let b03 = m01 * m12 - m02 * m11;
    let b04 = m01 * m13 - m03 * m11;
    let b05 = m02 * m13 - m03 * m12;
    let b06 = m20 * m31 - m21 * m30;
    let b07 = m20 * m32 - m22 * m30;
    let b08 = m20 * m33 - m23 * m30;
    let b09 = m21 * m32 - m22 * m31;
    let b10 = m21 * m33 - m23 * m31;
    let b11 = m22 * m33 - m23 * m32;

    var inv = mat4x4f();
    inv[0][0] = m11 * b11 - m12 * b10 + m13 * b09;
    inv[0][1] = m02 * b10 - m01 * b11 - m03 * b09;
    inv[0][2] = m31 * b05 - m32 * b04 + m33 * b03;
    inv[0][3] = m22 * b04 - m21 * b05 - m23 * b03;
    inv[1][0] = m12 * b08 - m10 * b11 - m13 * b07;
    inv[1][1] = m00 * b11 - m02 * b08 + m03 * b07;
    inv[1][2] = m32 * b02 - m30 * b05 - m33 * b01;
    inv[1][3] = m20 * b05 - m22 * b02 + m23 * b01;
    inv[2][0] = m10 * b10 - m11 * b08 + m13 * b06;
    inv[2][1] = m01 * b08 - m00 * b10 - m03 * b06;
    inv[2][2] = m30 * b04 - m31 * b02 + m33 * b00;
    inv[2][3] = m21 * b02 - m20 * b04 - m23 * b00;
    inv[3][0] = m11 * b07 - m10 * b09 - m12 * b06;
    inv[3][1] = m00 * b09 - m01 * b07 + m02 * b06;
    inv[3][2] = m31 * b01 - m30 * b03 - m32 * b00;
    inv[3][3] = m20 * b03 - m21 * b01 + m22 * b00;

    let det = m00 * b00 - m01 * b01 + m02 * b02 - m03 * b03;
    let invDet = 1.0 / det;
    return inv * invDet;
}

fn S_SolveQuadraticEq(a: f32, b: f32, c: f32, minT: ptr<function, f32>, maxT: ptr<function, f32>) -> bool {
    let twoA = 2.0 * a;
    let det = b * b - 2.0 * twoA * c;
    if (det < 0.0) { *minT = 0.0; *maxT = 0.0; return false; }
    let sqrtDet = sqrt(det);
    let root1 = (-b - sqrtDet) / twoA;
    let root2 = (-b + sqrtDet) / twoA;
    *minT = min(root1, root2);
    *maxT = max(root1, root2);
    return true;
}

fn S_LightDistanceAttenuation(distance: f32, radius: f32) -> f32 {
    return clamp(1.0 - distance * distance / (radius * radius), 0.0, 1.0);
}

fn S_Project(worldPosition: vec3f, matrix: mat4x4f) -> vec3f {
    let screenPos = matrix * vec4f(worldPosition, 1.0);
    return (screenPos.xyz / screenPos.w) * 0.5 + 0.5;
}

fn S_UnProject(screenPos: vec3f, matrix: mat4x4f) -> vec3f {
    let clipSpacePos = vec4f(screenPos * 2.0 - 1.0, 1.0);
    let position = matrix * clipSpacePos;
    return position.xyz / position.w;
}

fn S_DistributionGGX(N: vec3f, H: vec3f, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let nom = a2;
    var denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;
    return nom / denom;
}

fn S_GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    let nom = NdotV;
    let denom = NdotV * (1.0 - k) + k;
    return nom / denom;
}

fn S_GeometrySmith(N: vec3f, V: vec3f, L: vec3f, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = S_GeometrySchlickGGX(NdotV, roughness);
    let ggx1 = S_GeometrySchlickGGX(NdotL, roughness);
    return ggx1 * ggx2;
}

fn S_FresnelSchlick(cosTheta: f32, F0: vec3f) -> vec3f {
    return F0 + (1.0 - F0) * pow(max(1.0 - cosTheta, 0.0), 5.0);
}

fn S_FresnelSchlickRoughness(cosTheta: f32, F0: vec3f, roughness: f32) -> vec3f {
    return F0 + (max(vec3f(1.0 - roughness), F0) - F0) * pow(1.0 - cosTheta, 5.0);
}

struct TPBRContext {
    lightColor: vec3f,
    viewVector: vec3f,
    fragmentToLight: vec3f,
    fragmentNormal: vec3f,
    metallic: f32,
    roughness: f32,
    albedo: vec3f,
};

fn S_PBR_CalculateLight(ctx: TPBRContext) -> vec3f {
    let F0 = mix(vec3f(0.04), ctx.albedo, ctx.metallic);
    let L = ctx.fragmentToLight;
    let H = normalize(ctx.viewVector + L);
    let NDF = S_DistributionGGX(ctx.fragmentNormal, H, ctx.roughness);
    let G = S_GeometrySmith(ctx.fragmentNormal, ctx.viewVector, L, ctx.roughness);
    let F = S_FresnelSchlick(max(dot(H, ctx.viewVector), 0.0), F0);
    let numerator = NDF * G * F;
    let denominator = 4.0 * max(dot(ctx.fragmentNormal, ctx.viewVector), 0.0) * max(dot(ctx.fragmentNormal, L), 0.0) + 0.001;
    let specular = numerator / denominator;
    let kS = F;
    var kD = vec3f(1.0) - kS;
    kD *= 1.0 - ctx.metallic;
    let NdotL = max(dot(ctx.fragmentNormal, L), 0.0);
    return (kD * ctx.albedo / PI + specular) * ctx.lightColor * NdotL;
}

fn S_InScatter(start: vec3f, dir: vec3f, lightPos: vec3f, d: f32) -> f32 {
    let q = start - lightPos;
    let b = dot(dir, q);
    let c = dot(q, q);
    let s = 1.0 / sqrt(c - b * b);
    let l = s * (atan((d + b) * s) - atan(b * s));
    return l;
}

fn S_RayleighScatter(start: vec3f, dir: vec3f, lightPos: vec3f, d: f32) -> vec3f {
    let scatter = S_InScatter(start, dir, lightPos, d);
    return vec3f(0.55, 0.75, 1.0) * scatter;
}

fn S_RaySphereIntersection(origin: vec3f, dir: vec3f, center: vec3f, radius: f32, minT: ptr<function, f32>, maxT: ptr<function, f32>) -> bool {
    let d = origin - center;
    let a = dot(dir, dir);
    let b = 2.0 * dot(dir, d);
    let c = dot(d, d) - radius * radius;
    return S_SolveQuadraticEq(a, b, c, minT, maxT);
}

fn S_PointShadow(
    shadowsEnabled: bool,
    softShadows: bool,
    fragmentDistance: f32,
    shadowBias: f32,
    toLight: vec3f,
    shadowMap_tex: texture_cube<f32>,
    shadowMap_samp: sampler) -> f32
{
    if (shadowsEnabled) {
        let biasedFragmentDistance = fragmentDistance - shadowBias;

        if (softShadows) {
            const directions = array<vec3f, 20>(
                vec3f(1, 1, 1), vec3f(1, -1, 1), vec3f(-1, -1, 1), vec3f(-1, 1, 1),
                vec3f(1, 1, -1), vec3f(1, -1, -1), vec3f(-1, -1, -1), vec3f(-1, 1, -1),
                vec3f(1, 1, 0), vec3f(1, -1, 0), vec3f(-1, -1, 0), vec3f(-1, 1, 0),
                vec3f(1, 0, 1), vec3f(-1, 0, 1), vec3f(1, 0, -1), vec3f(-1, 0, -1),
                vec3f(0, 1, 1), vec3f(0, -1, 1), vec3f(0, -1, -1), vec3f(0, 1, -1)
            );

            const diskRadius = 0.0025;

            var accumulator = 0.0;

            for (var i = 0; i < 20; i++) {
                let fetchDirection = -toLight + directions[i] * diskRadius;
                let shadowDistanceToLight = textureSample(shadowMap_tex, shadowMap_samp, fetchDirection).r;
                if (biasedFragmentDistance > shadowDistanceToLight) {
                    accumulator += 1.0;
                }
            }

            return clamp(1.0 - accumulator / 20.0, 0.0, 1.0);
        } else {
            let shadowDistanceToLight = textureSample(shadowMap_tex, shadowMap_samp, -toLight).r;
            return select(1.0, 0.0, biasedFragmentDistance > shadowDistanceToLight);
        }
    } else {
        return 1.0;
    }
}

fn S_SpotShadowFactor(
    shadowsEnabled: bool,
    softShadows: bool,
    shadowBias: f32,
    fragmentPosition: vec3f,
    lightViewProjMatrix: mat4x4f,
    shadowMapInvSize: f32,
    spotShadowTexture_tex: texture_2d<f32>,
    spotShadowTexture_samp: sampler) -> f32
{
    if (shadowsEnabled) {
        let lightSpacePosition = S_Project(fragmentPosition, lightViewProjMatrix);

        let biasedLightSpaceFragmentDepth = lightSpacePosition.z - shadowBias;

        if (softShadows) {
            var accumulator = 0.0;

            let stepSize = 0.5;
            let kernelHalfSize = 2.0;
            let kernelSize = 2.0 * kernelHalfSize;
            let totalSamples = pow(kernelSize / stepSize, 2.0);

            var y = -kernelHalfSize;
            loop {
                if (y > kernelHalfSize) { break; }
                var x = -kernelHalfSize;
                loop {
                    if (x > kernelHalfSize) { break; }
                    let fetchTexCoord = lightSpacePosition.xy + vec2f(x, y) * shadowMapInvSize;
                    if (biasedLightSpaceFragmentDepth > textureSample(spotShadowTexture_tex, spotShadowTexture_samp, fetchTexCoord).r) {
                        accumulator += 1.0;
                    }
                    x += stepSize;
                }
                y += stepSize;
            }

            return clamp(1.0 - accumulator / totalSamples, 0.0, 1.0);
        } else {
            return select(1.0, 0.0, biasedLightSpaceFragmentDepth > textureSample(spotShadowTexture_tex, spotShadowTexture_samp, lightSpacePosition.xy).r);
        }
    } else {
        return 1.0;
    }
}

// Depth-texture variant of S_PointShadow (textureSample returns f32, no .r needed)
fn S_PointShadow_Depth(
    shadowsEnabled: bool,
    softShadows: bool,
    fragmentDistance: f32,
    shadowBias: f32,
    toLight: vec3f,
    shadowMap_tex: texture_depth_cube,
    shadowMap_samp: sampler) -> f32
{
    if (shadowsEnabled) {
        let biasedFragmentDistance = fragmentDistance - shadowBias;
        if (softShadows) {
            const directions = array<vec3f, 20>(
                vec3f(1, 1, 1), vec3f(1, -1, 1), vec3f(-1, -1, 1), vec3f(-1, 1, 1),
                vec3f(1, 1, -1), vec3f(1, -1, -1), vec3f(-1, -1, -1), vec3f(-1, 1, -1),
                vec3f(1, 1, 0), vec3f(1, -1, 0), vec3f(-1, -1, 0), vec3f(-1, 1, 0),
                vec3f(1, 0, 1), vec3f(-1, 0, 1), vec3f(1, 0, -1), vec3f(-1, 0, -1),
                vec3f(0, 1, 1), vec3f(0, -1, 1), vec3f(0, -1, -1), vec3f(0, 1, -1)
            );
            const diskRadius = 0.0025;
            var accumulator = 0.0;
            for (var i = 0; i < 20; i++) {
                let fetchDirection = -toLight + directions[i] * diskRadius;
                let shadowDistanceToLight = textureSample(shadowMap_tex, shadowMap_samp, fetchDirection);
                if (biasedFragmentDistance > shadowDistanceToLight) { accumulator += 1.0; }
            }
            return clamp(1.0 - accumulator / 20.0, 0.0, 1.0);
        } else {
            let shadowDistanceToLight = textureSample(shadowMap_tex, shadowMap_samp, -toLight);
            return select(1.0, 0.0, biasedFragmentDistance > shadowDistanceToLight);
        }
    } else { return 1.0; }
}

// Depth-texture variant of S_SpotShadowFactor
fn S_SpotShadowFactor_Depth(
    shadowsEnabled: bool,
    softShadows: bool,
    shadowBias: f32,
    fragmentPosition: vec3f,
    lightViewProjMatrix: mat4x4f,
    shadowMapInvSize: f32,
    spotShadowTexture_tex: texture_depth_2d,
    spotShadowTexture_samp: sampler) -> f32
{
    if (shadowsEnabled) {
        let lightSpacePosition = S_Project(fragmentPosition, lightViewProjMatrix);
        let biasedLightSpaceFragmentDepth = lightSpacePosition.z - shadowBias;
        if (softShadows) {
            var accumulator = 0.0;
            let stepSize = 0.5;
            let kernelHalfSize = 2.0;
            let kernelSize = 2.0 * kernelHalfSize;
            let totalSamples = pow(kernelSize / stepSize, 2.0);
            var y = -kernelHalfSize;
            loop {
                if (y > kernelHalfSize) { break; }
                var x = -kernelHalfSize;
                loop {
                    if (x > kernelHalfSize) { break; }
                    let fetchTexCoord = lightSpacePosition.xy + vec2f(x, y) * shadowMapInvSize;
                    if (biasedLightSpaceFragmentDepth > textureSample(spotShadowTexture_tex, spotShadowTexture_samp, fetchTexCoord)) { accumulator += 1.0; }
                    x += stepSize;
                }
                y += stepSize;
            }
            return clamp(1.0 - accumulator / totalSamples, 0.0, 1.0);
        } else {
            return select(1.0, 0.0, biasedLightSpaceFragmentDepth > textureSample(spotShadowTexture_tex, spotShadowTexture_samp, lightSpacePosition.xy));
        }
    } else { return 1.0; }
}

fn Internal_FetchHeight(heightTexture_tex: texture_2d<f32>, heightTexture_samp: sampler, texCoords: vec2f, center: f32) -> f32 {
    return clamp(textureSample(heightTexture_tex, heightTexture_samp, texCoords).r - center, 0.0, 1.0);
}

fn S_ComputeParallaxTextureCoordinates(heightTexture_tex: texture_2d<f32>, heightTexture_samp: sampler, eyeVec: vec3f, texCoords: vec2f, center: f32, scale: f32) -> vec2f {
    const minLayers = 8.0;
    const maxLayers = 15.0;
    const maxIterations = 15;

    let t = max(0.0, abs(dot(vec3f(0.0, 0.0, 1.0), eyeVec)));
    let numLayers = mix(maxLayers, minLayers, t);
    let layerDepth = 1.0 / numLayers;
    var currentLayerDepth = 0.0;

    let deltaTexCoords = scale * eyeVec.xy / numLayers;

    var currentTexCoords = texCoords;
    var currentDepthMapValue = Internal_FetchHeight(heightTexture_tex, heightTexture_samp, currentTexCoords, center);

    for (var i = 0; i < maxIterations; i++) {
        if (currentLayerDepth < currentDepthMapValue) {
            currentTexCoords -= deltaTexCoords;
            currentDepthMapValue = Internal_FetchHeight(heightTexture_tex, heightTexture_samp, currentTexCoords, center);
            currentLayerDepth += layerDepth;
        } else {
            break;
        }
    }

    let prev = currentTexCoords + deltaTexCoords;
    let nextH = currentDepthMapValue - currentLayerDepth;
    let prevH = Internal_FetchHeight(heightTexture_tex, heightTexture_samp, prev, center) - currentLayerDepth + layerDepth;

    let weight = nextH / (nextH - prevH);

    return prev * weight + currentTexCoords * (1.0 - weight);
}

fn S_LinearIndexToPosition(index: i32, textureWidth: i32) -> vec2i {
    let y = index / textureWidth;
    let x = index - textureWidth * y;
    return vec2i(x, y);
}

fn S_FetchMatrix(storage_tex: texture_2d<f32>, storage_samp: sampler, index: i32) -> mat4x4f {
    let textureWidth = i32(textureDimensions(storage_tex, 0).x);
    let pos = S_LinearIndexToPosition(4 * index, textureWidth);

    let col1 = textureLoad(storage_tex, pos, 0);
    let col2 = textureLoad(storage_tex, vec2i(pos.x + 1, pos.y), 0);
    let col3 = textureLoad(storage_tex, vec2i(pos.x + 2, pos.y), 0);
    let col4 = textureLoad(storage_tex, vec2i(pos.x + 3, pos.y), 0);

    return mat4x4f(col1, col2, col3, col4);
}

struct TBlendShapeOffsets {
    position: vec3f,
    normal: vec3f,
    tangent: vec3f,
};

fn S_FetchBlendShapeOffsets(storage_tex: texture_3d<f32>, storage_samp: sampler, vertexIndex: i32, blendShapeIndex: i32) -> TBlendShapeOffsets {
    let textureWidth = i32(textureDimensions(storage_tex, 0).x);
    let pos = vec3i(S_LinearIndexToPosition(3 * vertexIndex, textureWidth), blendShapeIndex);
    let position = textureLoad(storage_tex, pos, 0).xyz;
    let normal = textureLoad(storage_tex, vec3i(pos.x + 1, pos.y, pos.z), 0).xyz;
    let tangent = textureLoad(storage_tex, vec3i(pos.x + 2, pos.y, pos.z), 0).xyz;
    return TBlendShapeOffsets(position, normal, tangent);
}

fn S_LinearToSRGB(color: vec4f) -> vec4f {
    let a = 12.92 * color.rgb;
    let b = 1.055 * pow(color.rgb, vec3f(1.0 / 2.4)) - 0.055;
    let c = step(vec3f(0.0031308), color.rgb);
    return vec4f(mix(a, b, c), color.a);
}

fn S_SRGBToLinear(color: vec4f) -> vec4f {
    let a = color.rgb / 12.92;
    let b = pow((color.rgb + 0.055) / 1.055, vec3f(2.4));
    let c = step(vec3f(0.04045), color.rgb);
    return vec4f(mix(a, b, c), color.a);
}

fn S_Luminance(x: vec3f) -> f32 {
    return dot(x, vec3f(0.2125, 0.7154, 0.0721));
}

fn S_RotateVec2(v: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    let m = mat2x2f(c, -s, s, c);
    return m * v;
}

fn S_ConvertRgbToXyz(rgb: vec3f) -> vec3f {
    var xyz: vec3f;
    xyz.x = dot(vec3f(0.4124564, 0.3575761, 0.1804375), rgb);
    xyz.y = dot(vec3f(0.2126729, 0.7151522, 0.0721750), rgb);
    xyz.z = dot(vec3f(0.0193339, 0.1191920, 0.9503041), rgb);
    return xyz;
}

fn S_ConvertXyzToRgb(xyz: vec3f) -> vec3f {
    var rgb: vec3f;
    rgb.x = dot(vec3f(3.2404542, -1.5371385, -0.4985314), xyz);
    rgb.y = dot(vec3f(-0.9692660, 1.8760108, 0.0415560), xyz);
    rgb.z = dot(vec3f(0.0556434, -0.2040259, 1.0572252), xyz);
    return rgb;
}

fn S_ConvertXyzToYxy(xyz: vec3f) -> vec3f {
    let inv = 1.0 / dot(xyz, vec3f(1.0, 1.0, 1.0));
    return vec3f(xyz.y, xyz.x * inv, xyz.y * inv);
}

fn S_ConvertYxyToXyz(Yxy: vec3f) -> vec3f {
    var xyz: vec3f;
    xyz.x = Yxy.x * Yxy.y / Yxy.z;
    xyz.y = Yxy.x;
    xyz.z = Yxy.x * (1.0 - Yxy.y - Yxy.z) / Yxy.z;
    return xyz;
}

fn S_ConvertRgbToYxy(rgb: vec3f) -> vec3f {
    return S_ConvertXyzToYxy(S_ConvertRgbToXyz(rgb));
}

fn S_ConvertYxyToRgb(Yxy: vec3f) -> vec3f {
    return S_ConvertXyzToRgb(S_ConvertYxyToXyz(Yxy));
}
