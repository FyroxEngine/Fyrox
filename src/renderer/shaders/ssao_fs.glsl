#define KERNEL_SIZE 32

uniform sampler2D depthSampler;
uniform sampler2D normalSampler;
uniform sampler2D noiseSampler;

uniform float radius;
uniform mat4 inverseProjectionMatrix;
uniform mat4 projectionMatrix;
uniform vec3 kernel[KERNEL_SIZE];
uniform vec2 noiseScale;
uniform mat3 viewMatrix;

out float finalOcclusion;

in vec2 texCoord;

vec3 GetViewSpacePosition(vec2 screenCoord) {
    return S_UnProject(vec3(screenCoord, texture(depthSampler, screenCoord).r), inverseProjectionMatrix);
}

void main() {
    vec3 fragPos = GetViewSpacePosition(texCoord);
    vec3 worldSpaceNormal = texture(normalSampler, texCoord).xyz * 2.0 - 1.0;
    vec3 viewSpaceNormal = normalize(viewMatrix * worldSpaceNormal);
    vec3 randomVec = normalize(texture(noiseSampler, texCoord * noiseScale).xyz * 2.0 - 1.0);

    vec3 tangent = normalize(randomVec - viewSpaceNormal * dot(randomVec, viewSpaceNormal));
    vec3 bitangent = normalize(cross(viewSpaceNormal, tangent));
    mat3 TBN = mat3(tangent, bitangent, viewSpaceNormal);

    float occlusion = 0.0;
    for (int i = 0; i < KERNEL_SIZE; ++i) {
        vec3 samplePoint = fragPos.xyz + TBN * kernel[i] * radius;

        vec4 offset = projectionMatrix * vec4(samplePoint, 1.0);
        offset.xy /= offset.w;
        offset.xy = offset.xy * 0.5 + 0.5;

        vec3 position = GetViewSpacePosition(offset.xy);

        float rangeCheck = smoothstep(0.0, 1.0, radius / abs(fragPos.z - position.z));
        occlusion += rangeCheck * ((position.z > samplePoint.z + 0.04) ? 1.0 : 0.0);
    }

    finalOcclusion = 1.0 - occlusion / float(KERNEL_SIZE);
}