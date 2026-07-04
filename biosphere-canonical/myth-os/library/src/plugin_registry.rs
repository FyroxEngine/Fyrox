// Plugin Registry — the catalogue of available plugins.
//
// Every capability in the Library is a plugin. The 16 Quantum modules are the
// foundational set; everything else builds on top of them.
//
// PluginDef is static metadata (what a plugin IS).
// PluginRegistry is the Bevy resource that holds all known plugins.
// ActivePlugin tracks which plugin is open in the current vault view.

use bevy::prelude::*;
use egui::Color32;

// ── Quantum module identity ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantumModule {
    Genesis, Quill,    Core,      Order,
    Vault,   Loom,     Forge,     Mythos,
    Codex,   Atlas,    Composer,  Prism,
    Architect, Chronicle, Animus, Nexus,
    Cipher,  Agora,
}

impl QuantumModule {
    pub fn color(&self) -> Color32 {
        match self {
            Self::Genesis   => Color32::from_rgb(255, 208,  96),
            Self::Quill     => Color32::from_rgb(140,  80, 255),
            Self::Core      => Color32::from_rgb(  0, 200, 180),
            Self::Order     => Color32::from_rgb(200, 168,  96),
            Self::Vault     => Color32::from_rgb(212, 160,  48),
            Self::Loom      => Color32::from_rgb(220,  60, 120),
            Self::Forge     => Color32::from_rgb(255, 100,   0),
            Self::Mythos    => Color32::from_rgb(128,  80, 224),
            Self::Codex     => Color32::from_rgb(  0, 192,  96),
            Self::Atlas     => Color32::from_rgb( 30, 140, 255),
            Self::Composer  => Color32::from_rgb(220, 140,  30),
            Self::Prism     => Color32::from_rgb(224, 216, 255),
            Self::Architect => Color32::from_rgb(100, 180, 255),
            Self::Chronicle => Color32::from_rgb(176, 128,  48),
            Self::Animus    => Color32::from_rgb(244, 192,  37),
            Self::Nexus     => Color32::from_rgb(255, 255, 255),
            Self::Cipher    => Color32::from_rgb(204,  16,  32),
            Self::Agora     => Color32::from_rgb( 74, 158, 143),
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Genesis   => "⬡",
            Self::Quill     => "✦",
            Self::Core      => "⊗",
            Self::Order     => "⊞",
            Self::Vault     => "◈",
            Self::Loom      => "⊜",
            Self::Forge     => "⚒",
            Self::Mythos    => "✧",
            Self::Codex     => "⊟",
            Self::Atlas     => "⊕",
            Self::Composer  => "♪",
            Self::Prism     => "◇",
            Self::Architect => "⊠",
            Self::Chronicle => "⊙",
            Self::Animus    => "⊛",
            Self::Nexus     => "◉",
            Self::Cipher    => "⊘",
            Self::Agora     => "⇌",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Genesis   => "Genesis",
            Self::Quill     => "Quill",
            Self::Core      => "Core",
            Self::Order     => "Order",
            Self::Vault     => "Vault",
            Self::Loom      => "Loom",
            Self::Forge     => "Forge",
            Self::Mythos    => "Mythos",
            Self::Codex     => "Codex",
            Self::Atlas     => "Atlas",
            Self::Composer  => "Composer",
            Self::Prism     => "Prism",
            Self::Architect => "Architect",
            Self::Chronicle => "Chronicle",
            Self::Animus    => "Animus",
            Self::Nexus     => "Nexus",
            Self::Cipher    => "Cipher",
            Self::Agora     => "Agora",
        }
    }
}

// ── Plugin definition ─────────────────────────────────────────────────────────

pub struct PluginDef {
    pub id:          &'static str,
    pub name:        &'static str,
    pub description: &'static str,
    pub module:      QuantumModule,
}

impl PluginDef {
    pub fn color(&self) -> Color32 { self.module.color() }
    pub fn glyph(&self) -> &'static str { self.module.glyph() }
}

// ── Core plugin catalogue ─────────────────────────────────────────────────────

pub const CORE_PLUGINS: &[PluginDef] = &[
    PluginDef { id: "genesis.seed",        name: "World Seeder",      module: QuantumModule::Genesis,   description: "Seed and manage Genesis Containers." },
    PluginDef { id: "quill.scrolls",       name: "Scrolls",           module: QuantumModule::Quill,     description: "Read and write narrative documents." },
    PluginDef { id: "quill.tomes",         name: "Tomes",             module: QuantumModule::Quill,     description: "Long-form structured knowledge." },
    PluginDef { id: "core.signal",         name: "Signal Monitor",    module: QuantumModule::Core,      description: "Coherence and resonance readouts." },
    PluginDef { id: "order.rules",         name: "Rule Editor",       module: QuantumModule::Order,     description: "Define and enforce vault laws." },
    PluginDef { id: "vault.archive",       name: "Archive Browser",   module: QuantumModule::Vault,     description: "Browse sealed and archived records." },
    PluginDef { id: "loom.board",          name: "Connection Board",  module: QuantumModule::Loom,      description: "Map relationships and social graphs." },
    PluginDef { id: "forge.blueprints",    name: "Blueprints",        module: QuantumModule::Forge,     description: "Design and build 3D structures." },
    PluginDef { id: "mythos.lore",         name: "Lore Browser",      module: QuantumModule::Mythos,    description: "Explore world lore and story arcs." },
    PluginDef { id: "codex.index",         name: "Codex Index",       module: QuantumModule::Codex,     description: "Searchable index of all vault knowledge." },
    PluginDef { id: "atlas.space",         name: "Space Map",         module: QuantumModule::Atlas,     description: "Navigate spatial and dimensional maps." },
    PluginDef { id: "composer.player",     name: "Audio Player",      module: QuantumModule::Composer,  description: "Music playback and audio synthesis." },
    PluginDef { id: "composer.studio",     name: "Composer Studio",   module: QuantumModule::Composer,  description: "Multi-track audio composition." },
    PluginDef { id: "prism.canvas",        name: "Canvas",            module: QuantumModule::Prism,     description: "2D drawing and illustration." },
    PluginDef { id: "architect.structure", name: "Structure Editor",  module: QuantumModule::Architect, description: "Plan and edit architectural forms." },
    PluginDef { id: "chronicle.log",       name: "Event Log",         module: QuantumModule::Chronicle, description: "Temporal record of vault events." },
    PluginDef { id: "animus.rhythm",       name: "Rhythm Engine",     module: QuantumModule::Animus,    description: "Motion, animation and rhythm tools." },
    PluginDef { id: "nexus.hub",           name: "Nexus Hub",         module: QuantumModule::Nexus,     description: "Cross-vault connection and routing." },
    PluginDef { id: "cipher.secure",       name: "Secure Vault",      module: QuantumModule::Cipher,    description: "Encrypted storage and glyph seals." },
    PluginDef { id: "agora.exchange",      name: "Exchange",          module: QuantumModule::Agora,     description: "Trade, barter and value transfer." },
    // ── Theatre ───────────────────────────────────────────────────────────────
    PluginDef { id: "theatre.stage",       name: "Master Stage",      module: QuantumModule::Prism,     description: "BioSpark Theatre compositor — composite renderer." },
    PluginDef { id: "theatre.mixer",       name: "Channel Mixer",     module: QuantumModule::Composer,  description: "16–64 channel instrument mixer. Faders, glyphs, layout." },
    // ── Genesis tools ─────────────────────────────────────────────────────────
    PluginDef { id: "genesis.forge",       name: "Plugin Forge",      module: QuantumModule::Genesis,   description: "Build, upload, and remix .qgcp / .qgenesis files." },
];

// ── Registry resource ─────────────────────────────────────────────────────────

#[derive(Resource)]
pub struct PluginRegistry {
    pub plugins: &'static [PluginDef],
}

impl PluginRegistry {
    pub fn by_id(&self, id: &str) -> Option<&PluginDef> {
        self.plugins.iter().find(|p| p.id == id)
    }
}

// ── Active plugin (which plugin is open in the current vault view) ────────────

#[derive(Resource, Default)]
pub struct ActivePlugin(pub Option<&'static str>);

// ── Plugin store open flag ────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct PluginStoreOpen(pub bool);

// ── Bevy plugin ───────────────────────────────────────────────────────────────

pub struct PluginRegistryPlugin;

impl Plugin for PluginRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PluginRegistry { plugins: CORE_PLUGINS })
           .init_resource::<ActivePlugin>()
           .init_resource::<PluginStoreOpen>();
    }
}
