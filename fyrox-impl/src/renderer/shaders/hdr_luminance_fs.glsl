uniform sampler2D frameSampler;

layout (std140) uniform Uniforms {
    mat4 worldViewProjection;
    vec2 invSize;
};

in vec2 texCoord;

out float outLum;

void main() {
    float totalLum = 0.0;
    for (float y = -0.5; y < 0.5; y += 0.5) {
        for (float x = -0.5; x < 0.5; x += 0.5) {
            totalLum += S_Luminance(texture(frameSampler, texCoord - vec2(x, y) * invSize).xyz);
        }
    }
    outLum = totalLum / 9.0;
}
