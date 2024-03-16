uniform sampler2D oldLumSampler;
uniform sampler2D newLumSampler;
uniform float speed;

out float outLum;

void main() {
    float oldLum = texture(oldLumSampler, vec2(0.5, 0.5)).r;
    float newLum = texture(newLumSampler, vec2(0.5, 0.5)).r;
    outLum = clamp(oldLum + (newLum - oldLum) * speed, 0.0, newLum);
}
