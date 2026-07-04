// Atlas Terrain Splatmap Shader — Bevy WGSL
// Channels: R=Sand  G=Grass  B=Rock  A=Snow
// Slope override: steep angles (>30°) force Rock blend.

struct TerrainMaterial {
    height_scale:     f32,
    slope_sharpness:  f32,   // NRPN 104
    sea_level:        f32,
    _pad:             f32,
}

@group(1) @binding(0)
var<uniform> material: TerrainMaterial;

@group(1) @binding(1)
var t_splat: texture_2d<f32>;
@group(1) @binding(2)
var s_splat: sampler;

@group(1) @binding(3)
var t_sand:  texture_2d<f32>;
@group(1) @binding(4)
var s_sand:  sampler;

@group(1) @binding(5)
var t_grass: texture_2d<f32>;
@group(1) @binding(6)
var s_grass: sampler;

@group(1) @binding(7)
var t_rock:  texture_2d<f32>;
@group(1) @binding(8)
var s_rock:  sampler;

@group(1) @binding(9)
var t_snow:  texture_2d<f32>;
@group(1) @binding(10)
var s_snow:  sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       world_normal:  vec3<f32>,
    @location(1)       uv:            vec2<f32>,
    @location(2)       world_height:  f32,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the biome splatmap (R=Sand G=Grass B=Rock A=Snow)
    let splat = textureSample(t_splat, s_splat, in.uv);

    // Slope-based rock override: dot(normal, up) < threshold → mix in rock
    let up        = vec3<f32>(0.0, 1.0, 0.0);
    let slope     = 1.0 - clamp(dot(in.world_normal, up), 0.0, 1.0);
    let rock_push = smoothstep(0.3 - material.slope_sharpness * 0.3,
                               0.3 + material.slope_sharpness * 0.3,
                               slope);

    var weights = splat;
    weights.b   = clamp(weights.b + rock_push, 0.0, 1.0);
    // Re-normalise
    let total = weights.r + weights.g + weights.b + weights.a + 0.0001;
    weights  /= total;

    // Tile each texture and blend by weights
    let tile  = in.uv * 32.0;
    let c_sand  = textureSample(t_sand,  s_sand,  tile);
    let c_grass = textureSample(t_grass, s_grass, tile);
    let c_rock  = textureSample(t_rock,  s_rock,  tile);
    let c_snow  = textureSample(t_snow,  s_snow,  tile);

    var col = c_sand  * weights.r
            + c_grass * weights.g
            + c_rock  * weights.b
            + c_snow  * weights.a;

    // Sub-sea tint
    if in.world_height < material.sea_level * material.height_scale {
        col = mix(col, vec4<f32>(0.05, 0.20, 0.45, 1.0), 0.75);
    }

    return col;
}
