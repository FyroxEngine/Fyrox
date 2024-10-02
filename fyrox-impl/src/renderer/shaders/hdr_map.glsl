uniform sampler2D hdrSampler;
uniform sampler2D lumSampler;
uniform sampler2D bloomSampler;
uniform sampler3D colorMapSampler;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    bool useColorGrading;
    float keyValue;
    float minLuminance;
    float maxLuminance;
    bool autoExposure;
    float fixedExposure;
};

in vec2 texCoord;

out vec4 outLdrColor;

vec3 ColorGrading(vec3 color) {
    const float lutSize = 16.0;
    const float a = (lutSize - 1.0) / lutSize;
    const float b = 1.0 / (2.0 * lutSize);
    vec3 scale = vec3(a);
    vec3 offset = vec3(b);
    return texture(colorMapSampler, scale * color + offset).rgb;
}

void main() {
    vec4 hdrColor = texture(hdrSampler, texCoord);

    hdrColor += texture(bloomSampler, texCoord);

    float luminance = texture(lumSampler, vec2(0.5, 0.5)).r;

    float exposure;
    if (autoExposure) {
        exposure = keyValue / clamp(luminance, minLuminance, maxLuminance);
    } else {
        exposure = fixedExposure;
    }

    vec4 ldrColor = vec4(vec3(1.0) - exp(-hdrColor.rgb * exposure), hdrColor.a);

    if (useColorGrading) {
        outLdrColor = vec4(ColorGrading(S_LinearToSRGB(ldrColor).rgb), ldrColor.a);
    } else {
        outLdrColor = S_LinearToSRGB(ldrColor);
    }
}
