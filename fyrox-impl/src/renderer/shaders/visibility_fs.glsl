uniform usampler2D tileBuffer;

layout (std140) uniform Uniforms {
    mat4 viewProjection;
    int tileSize;
    float frameBufferHeight;
};

out vec4 FragColor;

flat in uint objectIndex;

void main()
{
    int x = int(gl_FragCoord.x) / tileSize;
    int y = int(frameBufferHeight - gl_FragCoord.y) / tileSize;

    int bitIndex = -1;
    int tileDataIndex = x * 33;
    int count = int(texelFetch(tileBuffer, ivec2(tileDataIndex, y), 0).x);
    int objectsListStartIndex = tileDataIndex + 1;
    for (int i = 0; i < count; ++i) {
        uint pixelObjectIndex = uint(texelFetch(tileBuffer, ivec2(objectsListStartIndex + i, y), 0).x);
        if (pixelObjectIndex == objectIndex) {
            bitIndex = i;
            break;
        }
    }

    if (bitIndex < 0) {
        FragColor = vec4(0.0, 0.0, 0.0, 0.0);
    } else {
        uint outMask = uint(1 << bitIndex);
        float r = float(outMask & 255u) / 255.0;
        float g = float((outMask & 65280u) >> 8) / 255.0;
        float b = float((outMask & 16711680u) >> 16) / 255.0;
        float a = float((outMask & 4278190080u) >> 24) / 255.0;
        FragColor = vec4(r, g, b, a);
    }
}