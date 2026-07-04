use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub plugin_type: PluginType,
    /// Which wire types this plugin can send/receive
    pub wire_types: Vec<WireType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginType {
    /// A full vault (expansion card in the master vault)
    Vault,
    /// Provides node types for the graph
    NodeProvider,
    /// A data source / ingestion pipeline
    DataSource,
    /// An output / sink / renderer
    Output,
    /// A skill / LLM persona
    Skill,
    /// A hardware bridge (MIDI, sensors, etc.)
    Hardware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireType {
    AGT, // Agent
    AST, // Asset
    AUD, // Audio
    BHV, // Behavior
    CTL, // Control
    DAT, // Data
    ENR, // Energy
    EVT, // Event
    IDN, // Identity
    LGC, // Logic
    NAR, // Narrative
    SOC, // Social
    SPA, // Spatial
    TMP, // Temporal
    VIS, // Visual
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStatus {
    Registered,
    Active,
    Suspended,
    Error,
}

/// Describes a node type that a plugin provides
#[derive(Debug, Clone)]
pub struct NodeRegistration {
    pub type_id: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    pub num_inputs: usize,
    pub num_outputs: usize,
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
}

/// The trait every plugin implements
pub trait Plugin: Send {
    fn manifest(&self) -> &PluginManifest;
    fn status(&self) -> PluginStatus;
    fn initialize(&mut self) -> Result<(), String>;
    fn tick(&mut self, clock: u64);
    fn shutdown(&mut self);

    /// Return all node types this plugin provides
    fn registered_nodes(&self) -> &[NodeRegistration];
}

/// A plugin that provides node types to the graph
pub struct NodePlugin {
    pub manifest: PluginManifest,
    pub status: PluginStatus,
    pub nodes: Vec<NodeRegistration>,
}

impl NodePlugin {
    pub fn new(manifest: PluginManifest, nodes: Vec<NodeRegistration>) -> Self {
        Self {
            manifest,
            status: PluginStatus::Active,
            nodes,
        }
    }
}

impl Plugin for NodePlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn status(&self) -> PluginStatus {
        self.status
    }

    fn initialize(&mut self) -> Result<(), String> {
        self.status = PluginStatus::Active;
        Ok(())
    }

    fn tick(&mut self, _clock: u64) {}

    fn shutdown(&mut self) {
        self.status = PluginStatus::Suspended;
    }

    fn registered_nodes(&self) -> &[NodeRegistration] {
        &self.nodes
    }
}

// ── Built-in plugin factories ──────────────────────────────────────────

pub fn create_text_plugin() -> NodePlugin {
    NodePlugin::new(
        PluginManifest {
            name: "Text Atoms".into(),
            version: "1.0.0".into(),
            description: "Text sources and string transforms".into(),
            author: "PRISM Core".into(),
            plugin_type: PluginType::NodeProvider,
            wire_types: vec![WireType::DAT, WireType::NAR],
        },
        vec![
            NodeRegistration {
                type_id: "text_source".into(),
                display_name: "Text Source".into(),
                category: "Sources".into(),
                description: "Emits a text value".into(),
                num_inputs: 0,
                num_outputs: 1,
                input_labels: vec![],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "uppercase".into(),
                display_name: "Uppercase".into(),
                category: "Transforms".into(),
                description: "Converts text to uppercase".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "lowercase".into(),
                display_name: "Lowercase".into(),
                category: "Transforms".into(),
                description: "Converts text to lowercase".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "reverse".into(),
                display_name: "Reverse".into(),
                category: "Transforms".into(),
                description: "Reverses text".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "length".into(),
                display_name: "Length".into(),
                category: "Transforms".into(),
                description: "Returns character count".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "merge".into(),
                display_name: "Merge".into(),
                category: "Combinators".into(),
                description: "Joins two inputs".into(),
                num_inputs: 2,
                num_outputs: 1,
                input_labels: vec!["A".into(), "B".into()],
                output_labels: vec!["out".into()],
            },
        ],
    )
}

pub fn create_math_plugin() -> NodePlugin {
    NodePlugin::new(
        PluginManifest {
            name: "Math Atoms".into(),
            version: "1.0.0".into(),
            description: "Numeric sources and math operations".into(),
            author: "PRISM Core".into(),
            plugin_type: PluginType::NodeProvider,
            wire_types: vec![WireType::DAT],
        },
        vec![
            NodeRegistration {
                type_id: "number_source".into(),
                display_name: "Number".into(),
                category: "Sources".into(),
                description: "Emits a numeric value".into(),
                num_inputs: 0,
                num_outputs: 1,
                input_labels: vec![],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "double".into(),
                display_name: "Double".into(),
                category: "Math".into(),
                description: "Multiplies by 2".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "negate".into(),
                display_name: "Negate".into(),
                category: "Math".into(),
                description: "Flips sign".into(),
                num_inputs: 1,
                num_outputs: 1,
                input_labels: vec!["in".into()],
                output_labels: vec!["out".into()],
            },
            NodeRegistration {
                type_id: "add".into(),
                display_name: "Add".into(),
                category: "Math".into(),
                description: "Adds two numbers".into(),
                num_inputs: 2,
                num_outputs: 1,
                input_labels: vec!["A".into(), "B".into()],
                output_labels: vec!["out".into()],
            },
        ],
    )
}

pub fn create_output_plugin() -> NodePlugin {
    NodePlugin::new(
        PluginManifest {
            name: "Output Atoms".into(),
            version: "1.0.0".into(),
            description: "Sinks and display nodes".into(),
            author: "PRISM Core".into(),
            plugin_type: PluginType::Output,
            wire_types: vec![WireType::DAT, WireType::VIS],
        },
        vec![NodeRegistration {
            type_id: "sink".into(),
            display_name: "Output".into(),
            category: "Sinks".into(),
            description: "Captures and displays the final value".into(),
            num_inputs: 1,
            num_outputs: 0,
            input_labels: vec!["in".into()],
            output_labels: vec![],
        }],
    )
}
