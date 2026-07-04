#version 450

// ─── Uniforms ──────────────────────────────────────────────────────────────
layout(set = 0, binding = 0) uniform SimParams {
    float gravity;       // 9.81 default, range 1–30
    float precipitation; // 0.0–1.0
    float seed;          // 0.0–100.0
    float time;          // elapsed world-time in seconds
} u_sim;

layout(set = 0, binding = 1) uniform TerrainParams {
    float height_scale;  // NRPN 101/102
    float noise_freq;    // base noise frequency (default 0.03)
    float lacunarity;    // fBm lacunarity (default 2.5)
    float persistence;   // fBm persistence (default 0.5)
    int   octaves;       // fBm octave count (default 3)
    float sea_level;     // 0.0–1.0 fraction of height_scale
} u_terrain;

// ─── Vertex inputs ──────────────────────────────────────────────────────────
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec2 a_uv;

// ─── Vertex outputs ─────────────────────────────────────────────────────────
layout(location = 0) out vec2 v_uv;
layout(location = 1) out float v_height;  // 0–1 normalised
layout(location = 2) out float v_moisture;

// ─── Simplex 2D noise (MIT license, Stefan Gustavson) ───────────────────────
vec3 permute(vec3 x) { return mod(((x * 34.0) + 1.0) * x, 289.0); }

float snoise2(vec2 v) {
    const vec4 C = vec4(0.211324865405187, 0.366025403784439,
                       -0.577350269189626, 0.024390243902439);
    vec2 i  = floor(v + dot(v, C.yy));
    vec2 x0 = v - i + dot(i, C.xx);
    vec2 i1  = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
    vec4 x12 = x0.xyxy + C.xxzz;
    x12.xy  -= i1;
    i = mod(i, 289.0);
    vec3 p = permute(permute(i.y + vec3(0.0, i1.y, 1.0))
                   + i.x + vec3(0.0, i1.x, 1.0));
    vec3 m = max(0.5 - vec3(dot(x0, x0), dot(x12.xy, x12.xy),
                             dot(x12.zw, x12.zw)), 0.0);
    m = m * m; m = m * m;
    vec3 x_  = 2.0 * fract(p * C.www) - 1.0;
    vec3 h   = abs(x_) - 0.5;
    vec3 ox  = floor(x_ + 0.5);
    vec3 a0  = x_ - ox;
    m *= 1.79284291400159 - 0.85373472095314 * (a0 * a0 + h * h);
    vec3 g;
    g.x  = a0.x  * x0.x   + h.x  * x0.y;
    g.yz = a0.yz * x12.xz + h.yz * x12.yw;
    return 130.0 * dot(m, g);
}

float fbm(vec2 p) {
    float val = 0.0;
    float amp = 1.0;
    float freq = u_terrain.noise_freq;
    for (int i = 0; i < u_terrain.octaves; i++) {
        val  += amp * snoise2(p * freq + u_sim.seed + u_sim.time * 0.001);
        amp  *= u_terrain.persistence;
        freq *= u_terrain.lacunarity;
    }
    return val * 0.5 + 0.5;  // remap to 0–1
}

// ─── Main ───────────────────────────────────────────────────────────────────
layout(set = 1, binding = 0) uniform mat4 u_model;
layout(set = 1, binding = 1) uniform mat4 u_view_proj;

void main() {
    vec2 world_xz = a_position.xz;

    float h = fbm(world_xz);

    // Gravity flattens terrain: higher gravity → reduced vertical range
    float gravity_factor = clamp(9.81 / u_sim.gravity, 0.1, 3.0);
    h *= gravity_factor;

    // Precipitation raises sea level and smooths valleys
    float water_push = u_sim.precipitation * 0.15;
    h = mix(h, h + water_push, u_sim.precipitation);

    float world_h = h * u_terrain.height_scale;
    v_height  = h;
    v_moisture = clamp(u_sim.precipitation + h * 0.3, 0.0, 1.0);
    v_uv = a_uv;

    vec4 pos = u_model * vec4(a_position.x, world_h, a_position.z, 1.0);
    gl_Position = u_view_proj * pos;
}
