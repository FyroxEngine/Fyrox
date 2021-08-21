#version 330 core

uniform sampler2D hdrSampler;
uniform sampler2D lumSampler;

in vec2 texCoord;

out vec4 outLdrColor;

void main() {
    vec4 hdrColor = texture(hdrSampler, texCoord);

    float lum = texture(lumSampler, vec2(0.5, 0.5)).r;

    // 255 / 32768 = 0.00778
    float exposure = 0.012 / max(lum, 0.00778);

    vec4 ldrColor = vec4(1.0) - exp(-hdrColor * exposure);

    outLdrColor = S_LinearToSRGB(ldrColor);
}
