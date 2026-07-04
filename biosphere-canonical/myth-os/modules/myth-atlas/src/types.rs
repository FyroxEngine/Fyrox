use myth_wire::WireType;
use myth_controls::ControlDef;

pub const CRATE_NAME: &str = "myth-atlas";
pub const CREST: &str = "Atlas";
pub const COLOR: &str = "#1e8cff";
pub const LAW: &str = "Space";
pub const DEPARTMENT: &str = "WorldConstruction";

// ─── Layer designations ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AtlasLayer {
    Coordinate,   // Layer I  — spatial anchoring, grid, transforms
    Cartographic, // Layer II — heightmaps, watersheds, biomes, geology
    Navigation,   // Layer III — pathfinding, zones, travel cost, encounter
    Intelligence, // Layer IV — regional oracle, hazards, resources, memory
}

// ─── Simulation parameters (from GLSL uniforms) ──────────────────────────────

/// The runtime simulation parameters that correspond to the GLSL shader uniforms.
/// These are the "knobs" on the Atlas instrument — tunable per-world.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AtlasSimParams {
    /// World gravity in m/s². Default 9.81. Range 1.0–30.0.
    /// Drives tectonic compression: higher gravity = flatter terrain.
    pub gravity: f32,
    /// Global precipitation level. 0.0 = arid, 1.0 = maximum moisture.
    /// Drives biome coloring and watershed density.
    pub precipitation: f32,
    /// Noise seed offset. Range 0.0–100.0.
    pub seed: f32,
    /// Elapsed world-time in seconds for animated noise drift.
    pub time: f32,
}

impl Default for AtlasSimParams {
    fn default() -> Self {
        Self { gravity: 9.81, precipitation: 0.5, seed: 0.0, time: 0.0 }
    }
}

// ─── Main config ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BiomeWeight {
    pub biome: BiomeType,
    pub weight: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AtlasConfig {
    pub sim: AtlasSimParams,
    pub terrain_seed: u64,
    pub grid_resolution: u32,       // hex grid cells across (default 256)
    pub chunk_size: u32,            // units per chunk
    pub elevation_scale: f32,       // max height in world units (default 30.0)
    pub sea_level: f32,             // 0.0–1.0 fraction of elevation_scale
    pub noise_octaves: u8,          // fBm octaves 1–8 (default 3)
    pub noise_lacunarity: f32,      // frequency multiplier per octave (default 2.5)
    pub noise_persistence: f32,     // amplitude multiplier per octave (default 0.5)
    pub noise_frequency: f32,       // base coordinate scale (default 0.03)
    pub erosion_strength: f32,      // hydraulic/thermal erosion 0.0–1.0
    pub erosion_iterations: u32,
    pub biome_weights: Vec<BiomeWeight>,
    pub spawn_density: f32,
    pub pathfinding_resolution: u32,
    pub river_count: u32,
    pub mountain_count: u32,
    /// NRPN 101/102: Global Height Scale coarse/fine
    pub nrpn_height_scale: f32,
    /// NRPN 103: Flora Density Threshold
    pub nrpn_flora_density: f32,
    /// NRPN 104: Slope/Rock Transition Sharpness
    pub nrpn_slope_sharpness: f32,
}

impl Default for AtlasConfig {
    fn default() -> Self {
        Self {
            sim: AtlasSimParams::default(),
            terrain_seed: 42,
            grid_resolution: 256,
            chunk_size: 64,
            elevation_scale: 30.0,
            sea_level: 0.35,
            noise_octaves: 3,
            noise_lacunarity: 2.5,
            noise_persistence: 0.5,
            noise_frequency: 0.03,
            erosion_strength: 0.3,
            erosion_iterations: 50,
            biome_weights: vec![],
            spawn_density: 1.0,
            pathfinding_resolution: 4,
            river_count: 8,
            mountain_count: 12,
            nrpn_height_scale: 1.0,
            nrpn_flora_density: 0.6,
            nrpn_slope_sharpness: 0.3,
        }
    }
}

// ─── Biomes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BiomeType {
    Arctic, Tundra, Boreal, Temperate, Grassland,
    Desert, Savanna, Tropical, Wetland, Coastal, Ocean, Volcanic,
}

// ─── Packet payloads ─────────────────────────────────────────────────────────

/// Outgoing SPA packet — one chunk of generated terrain.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerrainChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// grid_resolution × grid_resolution height values
    pub heightmap: Vec<f32>,
    pub biome_map: Vec<BiomeType>,
    pub passable: Vec<bool>,
    pub moisture: Vec<f32>,
}

/// Outgoing EVT packet — a world spawn point.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpawnPoint {
    pub position: [f32; 3],
    pub biome: BiomeType,
    pub surface_normal: [f32; 3],
}

/// Outgoing DAT packet — path mesh for navigation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PathMesh {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub vertices: Vec<[f32; 3]>,
    pub edges: Vec<[u32; 2]>,
    pub costs: Vec<f32>,
}

// ─── Module registry ─────────────────────────────────────────────────────────
//
// The canonical whitepaper data for all 256 addressable sub-module nodes
// of the Atlas instrument. Use `atlas_module_spec()` to get the full registry.
//
// Note: UIX wire type from the original whitepaper maps to WireType::Visual
// in our system (display-layer output is a visual signal).

#[derive(Debug, Clone)]
pub struct SubModuleSpec {
    pub name: &'static str,
    pub symbol: &'static str,
    pub wire_out: WireType,
    /// Optional instrument panel widget definition for this ATOM.
    /// `None` = passive processor (no interactive control on the panel).
    pub control: Option<ControlDef>,
}

#[derive(Debug, Clone)]
pub struct ContainerSpec {
    pub index: u32,
    pub name: &'static str,
    pub symbol: &'static str,
    pub wire_out: WireType,
    pub layer: AtlasLayer,
    pub sub_modules: [SubModuleSpec; 16],
}

/// Returns the complete 256-sub-module specification for the Atlas instrument.
/// All 16 containers × 16 sub-modules. Source: MYTH-01 Atlas Whitepaper.
pub fn atlas_module_spec() -> [ContainerSpec; 16] {
    [
        ContainerSpec {
            index: 1, name: "World Origin Anchor", symbol: "WOA",
            wire_out: WireType::Spatial, layer: AtlasLayer::Coordinate,
            sub_modules: [
                SubModuleSpec { name: "Absolute Center Calibrator",   symbol: "ACC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Floating-Point Shift Handler", symbol: "FSH", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Universal Time-Space Sync",    symbol: "UTS", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Origin Offset Matrix",         symbol: "OOM", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Planetary Core Locator",       symbol: "PCL", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Celestial Body Anchor",        symbol: "CBA", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Global Datum Registry",        symbol: "GDR", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Reference Frame Lock",         symbol: "RFL", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Multiverse Origin Bridger",    symbol: "MOB", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Chunk Zero Initializer",       symbol: "CZI", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Persistence State Anchor",     symbol: "PSA", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Axis Alignment Prover",        symbol: "AAP", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Sub-atomic Scale Mapper",      symbol: "SSM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Macro-Cosmic Scale Mapper",    symbol: "MSM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Origin Drift Compensator",     symbol: "ODC", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Base Reality Validator",       symbol: "BRV", wire_out: WireType::Energy, control: None },
            ],
        },
        ContainerSpec {
            index: 2, name: "Hex Grid Mapper", symbol: "HGM",
            wire_out: WireType::Spatial, layer: AtlasLayer::Coordinate,
            sub_modules: [
                SubModuleSpec { name: "Hexagon Tessellation Engine",  symbol: "HTE", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Neighbor Adjacency Query",     symbol: "NAQ", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Sub-Hex Granular Divider",     symbol: "SHG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Spherical Hex Projector",      symbol: "SHP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Hex Edge Renderer",            symbol: "HER", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Vertex Collision Detector",    symbol: "VCD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Grid State Serializer",        symbol: "GSS", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Hex Coordinate Indexer",       symbol: "HCI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Pentagonal Seam Stitcher",     symbol: "PSS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Multi-Scale Zoom Gridder",     symbol: "MZG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Hex-to-Square Converter",      symbol: "HSC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Topographical Grid Deformer",  symbol: "TGD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Isometric Hex Camera",         symbol: "IHC", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Grid-Snapping Magnet",         symbol: "GSM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Hex Area Calculator",          symbol: "HAC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Dynamic Hex Culling",          symbol: "DHC", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 3, name: "Strata Depth Calculator", symbol: "SDC",
            wire_out: WireType::Spatial, layer: AtlasLayer::Coordinate,
            sub_modules: [
                SubModuleSpec { name: "Crust Thickness Evaluator",    symbol: "CTE", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Mantle Gradient Mapper",       symbol: "MGM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Underworld Void Generator",    symbol: "UVG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Skybox Stratosphere Index",    symbol: "SSI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Z-Axis Coordinate Binder",     symbol: "ZCB", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Subterranean Cave Plotter",    symbol: "SCP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Sea-Level Baseline Anchor",    symbol: "SBA", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Atmospheric Pressure Modeler", symbol: "APM", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Deep Core Pressure Engine",    symbol: "DCP", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Strata Transition Blocker",    symbol: "STB", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Voxel Depth Integrator",       symbol: "VDI", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Geological Age Layering",      symbol: "GAL", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Water Table Depth Finder",     symbol: "WTF", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Excavation Void Tracker",      symbol: "EVT", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Bedrock Collision Mesh",       symbol: "BCM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Orbital Altitude Tracker",     symbol: "OAT", wire_out: WireType::Spatial, control: None },
            ],
        },
        ContainerSpec {
            index: 4, name: "Coordinate Transform Engine", symbol: "CTE",
            wire_out: WireType::Spatial, layer: AtlasLayer::Coordinate,
            sub_modules: [
                SubModuleSpec { name: "Lat-Lon to Hex Translator",    symbol: "LLH", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Polar to Cartesian Solver",    symbol: "PCS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "2D-Map to 3D-Globe Projector", symbol: "MGP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Vector Normalization Filter",   symbol: "VNF", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Euclidean Distance Prover",    symbol: "EDP", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Non-Euclidean Warp Engine",    symbol: "NEW", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Raycast Intersection Math",    symbol: "RIM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Spatial Hash Encrypter",       symbol: "SHE", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "GPS Real-World Importer",      symbol: "GRI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Mercator Distortion Corrector", symbol: "MDC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Quantum Superposition Plotter", symbol: "QSP", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Local-to-Global Space Matrix",  symbol: "LGS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Elevation Exaggeration Scaler", symbol: "EES", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Quaternion Rotation Solver",   symbol: "QRS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Matrix Inversion Cache",       symbol: "MIC", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Axis Flipping Utility",        symbol: "AFU", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 5, name: "Heightmap Generator", symbol: "HMG",
            wire_out: WireType::Spatial, layer: AtlasLayer::Cartographic,
            sub_modules: [
                SubModuleSpec { name: "Perlin Noise Synthesizer",     symbol: "PNS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Simplex Fractal Layering",     symbol: "SFL", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Thermal Erosion Simulator",    symbol: "TES", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Hydraulic Wear Modeler",       symbol: "HWM", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Peak Sharpness Filter",        symbol: "PSF", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Valley Smoothing Algorithm",   symbol: "VSA", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Terraced Stepping Generator",  symbol: "TSG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Crater Impact Seeder",         symbol: "CIS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Ridge-line Extractor",         symbol: "RLE", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Heightmap-to-Mesh Baker",      symbol: "HMB", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Base Elevation Offset",        symbol: "BEO", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "16-bit Grayscale Exporter",    symbol: "BGE", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Canyon Carving Spline",        symbol: "CCS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Micro-Detail Bump Mapper",     symbol: "MBM", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Overhang & Arch Generator",    symbol: "OAG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Coastal Cliff Extruder",       symbol: "CCE", wire_out: WireType::Spatial, control: None },
            ],
        },
        ContainerSpec {
            index: 6, name: "Watershed Tracer", symbol: "WST",
            wire_out: WireType::Spatial, layer: AtlasLayer::Cartographic,
            sub_modules: [
                SubModuleSpec { name: "Rainfall Accumulation Node",   symbol: "RAN", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Gravity Flow Director",        symbol: "GFD", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "River Meander Algorithm",      symbol: "RMA", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Lake Basin Filler",            symbol: "LBF", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Ocean Level Flooder",          symbol: "OLF", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Delta & Estuary Former",       symbol: "DEF", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Waterfall Drop Calculator",    symbol: "WDC", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Aquifer Permeability Tester",  symbol: "APT", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Evaporation Cycle Modeler",    symbol: "ECM", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Floodplain Expansion Zone",    symbol: "FEZ", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Tributary Merge Logic",        symbol: "TML", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Salinity Gradient Tracker",    symbol: "SGT", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Glacial Melt Router",          symbol: "GMR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Subterranean River Tracer",    symbol: "SRT", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Marshland Silt Depositor",     symbol: "MSD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Current Velocity Vectoring",   symbol: "CVV", wire_out: WireType::Spatial, control: None },
            ],
        },
        ContainerSpec {
            index: 7, name: "Biome Boundary Painter", symbol: "BBP",
            wire_out: WireType::Spatial, layer: AtlasLayer::Cartographic,
            sub_modules: [
                SubModuleSpec { name: "Temp-Moisture Matrix",         symbol: "TMM", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Voronoi Cell Biome Seeder",    symbol: "VBS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Transition Zone Blender",      symbol: "TZB", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Flora Density Allocator",      symbol: "FDA", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Fauna Habitat Zoner",          symbol: "FHZ", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Altitude Tree-line Clipper",   symbol: "ATC", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Rain-shadow Desertifier",      symbol: "RSD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Soil Type Descriptor",         symbol: "STD", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Micro-Climate Injector",       symbol: "MCI", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Seasonal Shift Modifier",      symbol: "SSM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Canopy Overlap Renderer",      symbol: "COR", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Undergrowth Foliage Spawner",  symbol: "UFS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Anomaly Biome Injector",       symbol: "ABI", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Biomass Yield Calculator",     symbol: "BYC", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Latitudinal Climate Bands",    symbol: "LCB", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Urban Sprawl Overwriter",      symbol: "USO", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 8, name: "Geological Fault Seeder", symbol: "GFS",
            wire_out: WireType::Spatial, layer: AtlasLayer::Cartographic,
            sub_modules: [
                SubModuleSpec { name: "Tectonic Plate Generator",     symbol: "TPG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Subduction Zone Modeler",      symbol: "SZM", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Volcanic Hotspot Placer",      symbol: "VHP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Seismic Stress Accumulator",   symbol: "SSA", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Rift Valley Spreader",         symbol: "RVS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Continental Drift Animator",   symbol: "CDA", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Orogenic Mountain Folder",     symbol: "OMF", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Earthquake Epicenter Rigger",  symbol: "EER", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Magma Chamber Volumetrics",    symbol: "MCV", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Fault-line Fissure Renderer",  symbol: "FFR", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Geothermal Vent Spawner",      symbol: "GVS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Metamorphic Rock Coder",       symbol: "MRC", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Crustal Fracture Network",     symbol: "CFN", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Island Arc Creator",           symbol: "IAC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Mineral Vein Extruder",        symbol: "MVE", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Tsunami Generation Node",      symbol: "TGN", wire_out: WireType::Energy, control: None },
            ],
        },
        ContainerSpec {
            index: 9, name: "Pathfinding Solver", symbol: "PFS",
            wire_out: WireType::Spatial, layer: AtlasLayer::Navigation,
            sub_modules: [
                SubModuleSpec { name: "A-Star Node Evaluator",        symbol: "ANE", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Dijkstra Network Mapper",      symbol: "DNM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Navmesh Polygon Baker",        symbol: "NPB", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Dynamic Obstacle Avoider",     symbol: "DOA", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Elevation Penalty Ponderer",   symbol: "EPP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Heuristic Distance Guesser",   symbol: "HDG", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Multi-Agent Swarm Router",     symbol: "MSR", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Flight-Path Volumetric Solver", symbol: "FVS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Aquatic Routing Protocol",     symbol: "ARP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Road-Snap Magnetizer",         symbol: "RSM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Path Smoothing Spliner",       symbol: "PSS", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Chokepoint Identifier",        symbol: "CPI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Wall-Climbing Raycaster",      symbol: "WCR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Portal Traversal Linker",      symbol: "PTL", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Off-Grid Ray-Stepper",         symbol: "OGR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Memory-Optimized Path Cacher", symbol: "MOP", wire_out: WireType::Data, control: None },
            ],
        },
        ContainerSpec {
            index: 10, name: "Zone Transition Gate", symbol: "ZTG",
            wire_out: WireType::Control, layer: AtlasLayer::Navigation,
            sub_modules: [
                SubModuleSpec { name: "Seamless Chunk Streamer",      symbol: "SCS", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Loading Screen Interpolator",  symbol: "LSI", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Instance Merge Resolver",      symbol: "IMR", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Fast-Travel Wormhole",         symbol: "FTW", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Boundary Pre-Loader Cache",    symbol: "BPC", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Multi-Server Handshake",       symbol: "MSH", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Skybox Swap Coordinator",      symbol: "SSC", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Physics State Preserver",      symbol: "PSP", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Aggro-Drop Boundary",          symbol: "ADB", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Quest Phase Transitioner",     symbol: "QPT", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Biome Audio Crossfader",       symbol: "BAC", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Weather State Synchronizer",   symbol: "WSS", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Party Tether Manager",         symbol: "PTM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Collision Mesh Swapper",       symbol: "CMS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Render Distance Fogger",       symbol: "RDF", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Fallback Safe-Spawn",          symbol: "FSS", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 11, name: "Travel Cost Calculator", symbol: "TCC",
            wire_out: WireType::Spatial, layer: AtlasLayer::Navigation,
            sub_modules: [
                SubModuleSpec { name: "Slope Incline Penalizer",      symbol: "SIP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Surface Friction Evaluator",   symbol: "SFE", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Encumbrance Mass Modifier",    symbol: "EMM", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Stamina Drain Integrator",     symbol: "SDI", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Weather Headwind Force",       symbol: "WHF", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Fluid Viscosity Drag",         symbol: "FVD", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Mount Speed Multiplier",       symbol: "MSM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Road-Pavement Bonus",          symbol: "RPB", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Stealth Crawl Reducer",        symbol: "SCR", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Fatigue State Accumulator",    symbol: "FSA", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Caloric Burn Estimator",       symbol: "CBE", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Vehicle Suspension Check",     symbol: "VSC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Footwear Grip Modifier",       symbol: "FGM", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Temporal Time-Dilation",       symbol: "TTD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Gravity Variance Calculator",  symbol: "GVC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Fast-Travel Currency Assessor", symbol: "FCA", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 12, name: "Encounter Radius Probe", symbol: "ERP",
            wire_out: WireType::Spatial, layer: AtlasLayer::Navigation,
            sub_modules: [
                SubModuleSpec { name: "Concentric Threat Rings",      symbol: "CTR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Line-of-Sight Raycaster",      symbol: "LSR", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Acoustic Noise Emitter",       symbol: "ANE", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Olfactory Scent Trailer",      symbol: "OST", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Stealth Camouflage Defeater",  symbol: "SCD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Mob Spawn Trigger Zone",       symbol: "MST", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Point-of-Interest Pinger",     symbol: "POP", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Radar Sweep Pulsar",           symbol: "RSP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Ambush Probability Engine",    symbol: "APE", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Day/Night Vision Cone",        symbol: "DVC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Z-Axis Aggro Cylinder",        symbol: "ZAC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Friendly NPC Greeter",         symbol: "FNG", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Trap Proximity Detonator",     symbol: "TPD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Magic Aura Resonator",         symbol: "MAR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Environmental Trigger Hook",   symbol: "ETH", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Probe Refresh Rate Timer",     symbol: "PRR", wire_out: WireType::Data, control: None },
            ],
        },
        ContainerSpec {
            index: 13, name: "Region Oracle", symbol: "RGO",
            wire_out: WireType::Data, layer: AtlasLayer::Intelligence,
            sub_modules: [
                SubModuleSpec { name: "Deep Lore Repository",         symbol: "DLR", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Political Border Controller",   symbol: "PBC", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Demographic Population Density", symbol: "DPD", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Historic Climate Archive",      symbol: "HCA", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Faction Control Matrix",        symbol: "FCM", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Regional Danger Level",        symbol: "RDL", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Naming Convention Lexicon",    symbol: "NCL", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Cultural Monument Index",       symbol: "CMI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Economic Trade Routes",        symbol: "ETR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Myth & Legend Rumormill",      symbol: "MLR", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Ancient Ruin Locator",         symbol: "ARL", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Language & Dialect Zoner",     symbol: "LDZ", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Taxation & Toll Register",     symbol: "TTR", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Local Law Enforcer",           symbol: "LLE", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Era/Timeline State Switcher",  symbol: "ESS", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Celestial Event Forecaster",   symbol: "CEF", wire_out: WireType::Energy, control: None },
            ],
        },
        ContainerSpec {
            index: 14, name: "Terrain Hazard Evaluator", symbol: "THE",
            wire_out: WireType::Energy, layer: AtlasLayer::Intelligence,
            sub_modules: [
                SubModuleSpec { name: "Lava & Heat Radiator",         symbol: "LHR", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Avalanche Risk Assessor",      symbol: "ARA", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Toxic Gas Volumetrics",        symbol: "TGV", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Quicksand Sink Physics",       symbol: "QSP", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Radiation Decay Field",        symbol: "RDF", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Extreme Cold Frostbite",       symbol: "ECF", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Lightning Strike Predictor",   symbol: "LSP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Rockfall Trajectory Modeler",  symbol: "RTM", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Wildfire Spread Algorithm",    symbol: "WSA", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Biocontagion Spore Cloud",     symbol: "BSC", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "High Gravity Crush Zone",      symbol: "HGZ", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Sub-zero Ice Slicker",         symbol: "SIS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Unstable Bridge Fracturer",    symbol: "UBF", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Thorn & Bramble Scratcher",    symbol: "TBS", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Abyssal Depths Pressure",      symbol: "ADP", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Hazard Mitigation Checker",    symbol: "HMC", wire_out: WireType::Control, control: None },
            ],
        },
        ContainerSpec {
            index: 15, name: "Resource Node Scanner", symbol: "RNS",
            wire_out: WireType::Data, layer: AtlasLayer::Intelligence,
            sub_modules: [
                SubModuleSpec { name: "Ore Vein Populator",           symbol: "OVP", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Rare Flora Spawner",           symbol: "RFS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Respawn Timer Logic",          symbol: "RTL", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Depletion State Tracker",      symbol: "DST", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Yield Rarity RNG",             symbol: "YRR", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Underground Oil/Water Diviner", symbol: "UOD", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Fishing Node Allocator",       symbol: "FNA", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Hunting Ground Tracker",       symbol: "HGT", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Node Contested Status",        symbol: "NCS", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Tool Requirement Verifier",    symbol: "TRV", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Dynamic Economy Scarcity",     symbol: "DES", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Prospecting Ping Visualizer",  symbol: "PPV", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Hidden Cache Excavator",       symbol: "HCE", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Magic Ley-Line Syphon",        symbol: "MLS", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Harvest Particle Emitter",     symbol: "HPE", wire_out: WireType::Visual, control: None },
                SubModuleSpec { name: "Inventory Payload Packer",     symbol: "IPP", wire_out: WireType::Data, control: None },
            ],
        },
        ContainerSpec {
            index: 16, name: "Geographic Memory Cache", symbol: "GMC",
            wire_out: WireType::Data, layer: AtlasLayer::Intelligence,
            sub_modules: [
                SubModuleSpec { name: "Fog of War Revealer",          symbol: "FWR", wire_out: WireType::Spatial, control: None },
                SubModuleSpec { name: "Visited Chunk Ledger",         symbol: "VCL", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Player Custom Waypoints",      symbol: "PCW", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Cartography Skill Leveler",    symbol: "CSL", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Line-of-Sight Memory Buffer",  symbol: "LMB", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Map Annotation Syncer",        symbol: "MAS", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Treasure Map Decoder",         symbol: "TMD", wire_out: WireType::Energy, control: None },
                SubModuleSpec { name: "Shared Guild Map-Link",        symbol: "SGM", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Local Change Persister",       symbol: "LCP", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Offline State Reconciler",     symbol: "OSR", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Voxel Destruction Memory",     symbol: "VDM", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "GPS Track Log Exporter",       symbol: "TLE", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Spatial Bookmark Indexer",     symbol: "SBI", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Death Marker Registrar",       symbol: "DMR", wire_out: WireType::Data, control: None },
                SubModuleSpec { name: "Cloud Save Compressor",        symbol: "CSC", wire_out: WireType::Control, control: None },
                SubModuleSpec { name: "Temporal Replay Buffer",       symbol: "TRB", wire_out: WireType::Data, control: None },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spec_has_16_containers() {
        assert_eq!(atlas_module_spec().len(), 16);
    }
    #[test]
    fn each_container_has_16_sub_modules() {
        for c in atlas_module_spec() {
            assert_eq!(c.sub_modules.len(), 16, "Container {} has wrong sub-module count", c.name);
        }
    }
    #[test]
    fn total_256_sub_modules() {
        let total: usize = atlas_module_spec().iter().map(|c| c.sub_modules.len()).sum();
        assert_eq!(total, 256);
    }
    #[test]
    fn layers_correctly_assigned() {
        let spec = atlas_module_spec();
        assert_eq!(spec[0].layer, AtlasLayer::Coordinate);
        assert_eq!(spec[4].layer, AtlasLayer::Cartographic);
        assert_eq!(spec[8].layer, AtlasLayer::Navigation);
        assert_eq!(spec[12].layer, AtlasLayer::Intelligence);
    }
}
