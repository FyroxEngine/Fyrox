#version 330

uniform sampler2D frameSampler;
uniform vec2 invSize;

in vec2 texCoord;

out float outLum;

float Luminance(vec3 x) {
    return dot(x, vec3(0.299, 0.587, 0.114));
}

void main() {
    float totalLum = 0.0;
    for (float y = -0.5; y < 0.5; y += 0.5) {
        for (float x = -0.5; x < 0.5; x += 0.5) {
            totalLum += Luminance(texture(frameSampler, texCoord - vec2(x, y) * invSize).xyz);
        }
    }
    outLum = totalLum / 9.0;
}
