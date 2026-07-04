#version 450

// ─── Varyings ───────────────────────────────────────────────────────────────
layout(location = 0) in vec2  v_uv;
layout(location = 1) in float v_height;   // 0–1 normalised
layout(location = 2) in float v_moisture;

// ─── Uniforms ───────────────────────────────────────────────────────────────
layout(set = 0, binding = 0) uniform SimParams {
    float gravity;
    float precipitation;
    float seed;
    float time;
} u_sim;

layout(set = 0, binding = 1) uniform TerrainParams {
    float height_scale;
    float noise_freq;
    float lacunarity;
    float persistence;
    int   octaves;
    float sea_level;
} u_terrain;

// ─── Output ─────────────────────────────────────────────────────────────────
layout(location = 0) out vec4 frag_color;

// ─── Biome colour ramps ────────────────────────────────────────────────────
vec3 ocean_color()   { return vec3(0.05, 0.20, 0.45); }
vec3 wetland_color() { return vec3(0.12, 0.38, 0.22); }
vec3 arid_color()    { return vec3(0.76, 0.64, 0.38); }
vec3 grass_color()   { return vec3(0.25, 0.55, 0.15); }
vec3 forest_color()  { return vec3(0.10, 0.38, 0.10); }
vec3 rock_color()    { return vec3(0.45, 0.40, 0.35); }
vec3 snow_color()    { return vec3(0.92, 0.95, 1.00); }

void main() {
    float h  = v_height;
    float m  = v_moisture;

    vec3 col;

    if (h < u_terrain.sea_level) {
        // Ocean / deep water — blend depth
        float depth = 1.0 - (h / u_terrain.sea_level);
        col = mix(ocean_color() * 1.3, ocean_color() * 0.4, depth);

    } else if (m > 0.72 && h < u_terrain.sea_level + 0.12) {
        // Wetland / marsh
        col = wetland_color();

    } else if (m < 0.28) {
        // Arid / desert
        float sand_lerp = clamp((h - u_terrain.sea_level) * 4.0, 0.0, 1.0);
        col = mix(arid_color() * 1.1, rock_color(), sand_lerp * 0.4);

    } else if (h > 0.82) {
        // Snow-capped peaks
        float snow_blend = clamp((h - 0.82) * 8.0, 0.0, 1.0);
        col = mix(rock_color(), snow_color(), snow_blend);

    } else if (h > 0.68) {
        // Rocky high terrain
        float rock_blend = clamp((h - 0.68) * 7.0, 0.0, 1.0);
        col = mix(forest_color(), rock_color(), rock_blend);

    } else if (m > 0.55) {
        // Tropical / temperate forest
        col = mix(grass_color(), forest_color(), (m - 0.55) * 2.2);

    } else {
        // Grassland
        col = grass_color();
    }

    // Simple ambient + diffuse approximation (no light uniform required)
    float ambient  = 0.35;
    float diffuse  = 0.65 * clamp(1.0 - abs(h - 0.5), 0.0, 1.0);
    col = col * (ambient + diffuse);

    frag_color = vec4(col, 1.0);
}
