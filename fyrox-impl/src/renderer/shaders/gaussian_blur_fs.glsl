uniform sampler2D image;
uniform vec2 pixelSize;
uniform bool horizontal;

in vec2 texCoord;

out vec4 outColor;

void main()
{
    const float weights[5] = float[] (0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);

    vec4 center = texture(image, texCoord);

    vec3 result = center.rgb * weights[0];

    if (horizontal) {
        for(int i = 1; i < 5; ++i) {
            float fi = float(i);

            result += texture(image, texCoord + vec2(pixelSize.x * fi, 0.0)).rgb * weights[i];
            result += texture(image, texCoord - vec2(pixelSize.x * fi, 0.0)).rgb * weights[i];
        }
    } else {
        for(int i = 1; i < 5; ++i) {
            float fi = float(i);

            result += texture(image, texCoord + vec2(0.0, pixelSize.y * fi)).rgb * weights[i];
            result += texture(image, texCoord - vec2(0.0, pixelSize.y * fi)).rgb * weights[i];
        }
    }

    outColor = vec4(result, center.a);
}