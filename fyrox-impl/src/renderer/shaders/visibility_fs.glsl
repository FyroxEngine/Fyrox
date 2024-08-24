uniform int tileSize;
uniform usampler2D tileBuffer;
uniform float frameBufferHeight;

out vec4 FragColor;

flat in int instanceId;

void main()
{
    int x = int(gl_FragCoord.x) / tileSize;
    int y = int(frameBufferHeight - gl_FragCoord.y) / tileSize;

    // TODO: Replace with binary search.
    int bitIndex = -1;
    for (int i = 0; i < 32; ++i) {
        uint objectIndex = uint(texelFetch(tileBuffer, ivec2(x * tileSize + i, y), 0).x);
        if (objectIndex == uint(instanceId)) {
            bitIndex = i;
            break;
        }
    }

    if (bitIndex < 0) {
        FragColor = vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        FragColor = vec4(float(1 << bitIndex), 0.0, 0.0, 0.0);
    }
}