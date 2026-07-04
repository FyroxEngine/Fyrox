/// Splatmap channel assignment — must match atlas-terrain.wgsl channel order.
///
/// R = Sand/Arid  G = Grass/Temperate  B = Rock/Peak  A = Snow/Arctic
pub struct SplatChannel;
impl SplatChannel {
    pub const SAND:  usize = 0;
    pub const GRASS: usize = 1;
    pub const ROCK:  usize = 2;
    pub const SNOW:  usize = 3;
}

use myth_atlas::types::BiomeType;

/// Convert a BiomeType to a 4-channel RGBA splatmap weight.
/// Returns [sand, grass, rock, snow] as 0–255 u8 values.
pub fn biome_to_splat(biome: &BiomeType) -> [u8; 4] {
    match biome {
        BiomeType::Desert   | BiomeType::Savanna                    => [230, 25,  0,   0  ],
        BiomeType::Grassland | BiomeType::Temperate | BiomeType::Boreal => [0,   200, 55,  0  ],
        BiomeType::Tundra   | BiomeType::Volcanic                   => [10,  30,  200, 15 ],
        BiomeType::Arctic                                           => [0,   0,   80,  175],
        BiomeType::Tropical | BiomeType::Wetland                    => [0,   210, 45,  0  ],
        BiomeType::Coastal  | BiomeType::Ocean                      => [180, 30,  40,  5  ],
    }
}
