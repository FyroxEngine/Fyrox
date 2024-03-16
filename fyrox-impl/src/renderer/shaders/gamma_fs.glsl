uniform sampler2D screenTexture;

in vec2 texCoord;
out vec4 fragColor;

void main() {
    fragColor = S_LinearToSRGB(texture(screenTexture, texCoord));
}
