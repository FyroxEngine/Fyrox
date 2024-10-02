uniform sampler2D visibilityBuffer;

layout (std140) uniform Uniforms {
    mat4 viewProjection;
    int tileSize;
};

out uint optimizedVisibilityMask;

void main()
{
    int tileX = int(gl_FragCoord.x);
    int tileY = int(gl_FragCoord.y);

    int beginX = tileX * tileSize;
    int beginY = tileY * tileSize;

    int endX = (tileX + 1) * tileSize;
    int endY = (tileY + 1) * tileSize;

    int visibilityMask = 0;
    for (int y = beginY; y < endY; ++y) {
        for (int x = beginX; x < endX; ++x) {
            ivec4 mask = ivec4(texelFetch(visibilityBuffer, ivec2(x, y), 0) * 255.0);
            visibilityMask |= (mask.a << 24) | (mask.b << 16) | (mask.g << 8) | mask.r;
        }
    }
    optimizedVisibilityMask = uint(visibilityMask);
}