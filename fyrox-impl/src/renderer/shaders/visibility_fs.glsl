uniform int tileSize;
uniform sampler2D tileBuffer;
uniform float frameBufferHeight;

out vec4 FragColor;

flat in int instanceId;

void main()
{
    int x = int(gl_FragCoord.x) / tileSize;
    int y = int(frameBufferHeight - gl_FragCoord.y) / tileSize;

    // TODO: Replace with binary search.
    // TODO: Handle empty pixels.
    int bitIndex = -1;
    for (int i = 0; i < 32; ++i) {
        int objectIndex = int(texelFetch(tileBuffer, ivec2(x + i, y), 0).x);
        if (objectIndex == instanceId) {
            bitIndex = i;
            break;
        }
    }

    if (bitIndex < 0) {
        discard;
    }

    float value = float(1 << bitIndex);
    FragColor = vec4(value, 0, 0, 0);
}