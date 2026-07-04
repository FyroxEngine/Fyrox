use serde::{Deserialize, Serialize};

/// The 16-Layer Z-Indexed Rendering System
///
/// Every vault has its own independent layer stack.
/// Layer 00 = Surface (header, footer, critical UI)
/// Layers -01 to -14 = Intermediate (content, graphs, timelines)
/// Layer -15 = Base (deep background / environmental anchor)
///
/// OCCLUSION RULE: Any layer with a solid (100% opaque) fill
/// acts as a total occluder — rendering stops for everything below it.

pub const NUM_LAYERS: usize = 16;
pub const SURFACE_LAYER: usize = 0;
pub const BASE_LAYER: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStack {
    pub layers: [Layer; NUM_LAYERS],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub index: usize,
    pub name: String,
    pub visible: bool,
    pub fill: LayerFill,
    pub content: LayerContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerFill {
    /// No fill — transparent, layers below show through
    None,
    /// Solid color — acts as TOTAL OCCLUDER, nothing below renders
    Solid([u8; 4]),
    /// Image background (filename in assets)
    Image(String),
    /// Gradient (top color, bottom color)
    Gradient([u8; 4], [u8; 4]),
    /// Live GPU shader (shader name/id)
    Shader(String),
}

impl LayerFill {
    /// Returns true if this fill completely occludes everything below
    pub fn is_occluder(&self) -> bool {
        match self {
            LayerFill::Solid(c) => c[3] == 255,
            LayerFill::Image(_) => true,
            LayerFill::Gradient(a, b) => a[3] == 255 && b[3] == 255,
            LayerFill::Shader(_) => true, // shaders fill the entire rect
            LayerFill::None => false,
        }
    }

    pub fn to_color32(&self) -> Option<egui::Color32> {
        match self {
            LayerFill::Solid(c) => Some(egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])),
            LayerFill::Gradient(a, _) => Some(egui::Color32::from_rgba_unmultiplied(a[0], a[1], a[2], a[3])),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerContent {
    /// Nothing on this layer yet
    Empty,
    /// The vault header
    Header,
    /// The vault footer
    Footer,
    /// The node graph workspace
    Graph,
    /// A timeline view
    Timeline,
    /// Custom content (plugin-defined)
    Custom(String),
}

impl Default for Layer {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            visible: true,
            fill: LayerFill::None,
            content: LayerContent::Empty,
        }
    }
}

impl LayerStack {
    /// Create a default vault layer stack
    pub fn new_vault_default() -> Self {
        let mut layers: [Layer; NUM_LAYERS] = std::array::from_fn(|i| Layer {
            index: i,
            name: format!("Layer -{:02}", i),
            visible: true,
            fill: LayerFill::None,
            content: LayerContent::Empty,
        });

        // Layer 00: Surface — header + footer
        layers[0].name = "Surface".into();
        layers[0].content = LayerContent::Header;

        // Layer 01: Primary workspace
        layers[1].name = "Workspace".into();
        layers[1].content = LayerContent::Graph;

        // Layer 14: Footer
        layers[14].name = "Footer".into();
        layers[14].content = LayerContent::Footer;

        // Layer 15: Base — deep background
        layers[15].name = "Base".into();
        layers[15].fill = LayerFill::Solid([10, 10, 16, 255]);

        Self { layers }
    }

    /// Determine which layers are actually visible given the occlusion rule.
    /// Returns indices from top (0) down, stopping at the first occluder.
    pub fn visible_layers(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for i in 0..NUM_LAYERS {
            let layer = &self.layers[i];
            if !layer.visible {
                continue;
            }
            result.push(i);
            if layer.fill.is_occluder() {
                break; // total occluder — stop rendering below
            }
        }
        result
    }

    /// Get mutable reference to a layer by index
    pub fn layer_mut(&mut self, index: usize) -> Option<&mut Layer> {
        self.layers.get_mut(index)
    }
}

/// Vault rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderMode {
    TwoD,
    ThreeD,
    Hybrid,
}

impl RenderMode {
    pub fn label(&self) -> &str {
        match self {
            RenderMode::TwoD => "2D",
            RenderMode::ThreeD => "3D",
            RenderMode::Hybrid => "Hybrid",
        }
    }

    pub const ALL: [RenderMode; 3] = [RenderMode::TwoD, RenderMode::ThreeD, RenderMode::Hybrid];
}
