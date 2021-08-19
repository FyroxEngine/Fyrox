#version 330

uniform sampler2D oldLumSampler;
uniform sampler2D newLumSampler;
uniform float speed;

in vec2 texCoord;

out float outLum;

void main() {
    float oldLum = texture(oldLumSampler, texCoord).r;
    float newLum = texture(newLumSampler, texCoord).r;
    outLum = oldLum + (newLum - oldLum) * speed;
}
