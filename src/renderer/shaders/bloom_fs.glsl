uniform sampler2D hdrSampler;

in vec2 texCoord;

out vec4 outBrightColor;

void main() {
    vec3 hdrPixel = texture(hdrSampler, texCoord).rgb;

    if (S_Luminance(hdrPixel) > 1.0) {
        outBrightColor = vec4(hdrPixel, 0.0);
    } else {
        outBrightColor = vec4(0.0);
    }
}
