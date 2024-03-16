uniform sampler2D lumSampler;
uniform vec2 invSize;

in vec2 texCoord;

out float outLum;

void main() {
    float lum0 = texture(lumSampler, texCoord).r;
    float lum1 = texture(lumSampler, texCoord + vec2(invSize.x, 0.0)).r;
    float lum2 = texture(lumSampler, texCoord + invSize).r;
    float lum3 = texture(lumSampler, texCoord + vec2(0.0, invSize.y)).r;
    outLum = (lum0 + lum1 + lum2 + lum3) / 4.0;
}
