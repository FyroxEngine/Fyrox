uniform int tileSize;
uniform sampler2D visibilityBuffer;

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
            visibilityMask |= int(texelFetch(visibilityBuffer, ivec2(x, y), 0).r);
        }
    }
    optimizedVisibilityMask = uint(visibilityMask);
}