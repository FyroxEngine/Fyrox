use serde::{Deserialize, Serialize};
use crate::{Capsule, Container, MythosModule, QgcpError, SealBlock, MAX_MYTHOS};
use myth_wire::WireType;

/// The 16 canonical layout regions — one per Mythos slot.
/// Follows the Law of 16. Every UI in the ecosystem maps to these regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutRegion {
    HeaderLeft,       // 01 — icon slots, left side of header
    HeaderCenter,     // 02 — title, breadcrumb, world name
    HeaderRight,      // 03 — icon slots, right side of header
    CanvasToolbar,    // 04 — top toolbar strip above main canvas
    CanvasLeft,       // 05 — left panel (collapsible)
    CanvasMain,       // 06 — primary work area
    CanvasRight,      // 07 — right panel (collapsible)
    CanvasStatusBar,  // 08 — status strip below main canvas
    FooterLeft,       // 09 — footer slot group, left
    FooterCenter,     // 10 — footer status / info display
    FooterRight,      // 11 — footer slot group, right
    OverlayModal,     // 12 — full-screen dialog layer
    OverlayDrawer,    // 13 — slide-in drawer panel
    OverlayTooltip,   // 14 — tooltip / popover layer
    Notification,     // 15 — toast / alert stack
    ContextMenu,      // 16 — right-click / long-press menus
}

impl LayoutRegion {
    pub const ALL: [LayoutRegion; 16] = [
        Self::HeaderLeft, Self::HeaderCenter, Self::HeaderRight,
        Self::CanvasToolbar, Self::CanvasLeft, Self::CanvasMain,
        Self::CanvasRight, Self::CanvasStatusBar,
        Self::FooterLeft, Self::FooterCenter, Self::FooterRight,
        Self::OverlayModal, Self::OverlayDrawer, Self::OverlayTooltip,
        Self::Notification, Self::ContextMenu,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::HeaderLeft     => "Header Left",
            Self::HeaderCenter   => "Header Center",
            Self::HeaderRight    => "Header Right",
            Self::CanvasToolbar  => "Canvas Toolbar",
            Self::CanvasLeft     => "Canvas Left",
            Self::CanvasMain     => "Canvas Main",
            Self::CanvasRight    => "Canvas Right",
            Self::CanvasStatusBar => "Canvas Status Bar",
            Self::FooterLeft     => "Footer Left",
            Self::FooterCenter   => "Footer Center",
            Self::FooterRight    => "Footer Right",
            Self::OverlayModal   => "Overlay Modal",
            Self::OverlayDrawer  => "Overlay Drawer",
            Self::OverlayTooltip => "Overlay Tooltip",
            Self::Notification   => "Notification",
            Self::ContextMenu    => "Context Menu",
        }
    }

    /// Default slot capacity for this region.
    pub fn default_capacity(&self) -> u8 {
        match self {
            Self::HeaderLeft | Self::HeaderRight => 4,
            Self::FooterLeft | Self::FooterRight => 4,
            Self::CanvasMain                     => 1,
            Self::OverlayModal                   => 1,
            Self::Notification                   => 8,
            _                                    => 2,
        }
    }

    /// Whether this region is collapsible by default.
    pub fn default_collapsible(&self) -> bool {
        matches!(self,
            Self::CanvasLeft | Self::CanvasRight |
            Self::OverlayDrawer | Self::CanvasToolbar |
            Self::CanvasStatusBar
        )
    }
}

/// A single slot definition — the payload of a UIGenesis Capsule.
///
/// Stored as JSON in the capsule so myth-qgcp stays dep-free.
/// myth-plugin parses these strings into typed SlotType enums
/// during layout negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDefinition {
    /// Stable slot identifier — used by plugins to request this slot.
    pub slot_id: String,

    /// Which layout region this slot lives in.
    pub region: LayoutRegion,

    /// How many plugins can share this slot simultaneously.
    pub capacity: u8,

    /// Whether this slot can be collapsed/hidden by the user.
    pub collapsible: bool,

    /// Default visibility: "always" | "on_demand" | "hidden"
    pub visibility: String,

    /// Heraldry symbol of current occupant. None = vacant.
    pub occupant_heraldry: Option<String>,

    /// Preferred width hint in logical pixels. None = fill available.
    pub preferred_width: Option<f32>,

    /// Preferred height hint in logical pixels. None = fill available.
    pub preferred_height: Option<f32>,
}

impl SlotDefinition {
    pub fn new(slot_id: impl Into<String>, region: LayoutRegion) -> Self {
        let cap = region.default_capacity();
        let collapsible = region.default_collapsible();
        Self {
            slot_id: slot_id.into(),
            region,
            capacity: cap,
            collapsible,
            visibility: "always".into(),
            occupant_heraldry: None,
            preferred_width: None,
            preferred_height: None,
        }
    }

    pub fn on_demand(mut self) -> Self {
        self.visibility = "on_demand".into();
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visibility = "hidden".into();
        self
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.preferred_width = Some(width);
        self.preferred_height = Some(height);
        self
    }

    pub fn is_vacant(&self) -> bool {
        self.occupant_heraldry.is_none()
    }

    pub fn occupy(&mut self, heraldry: impl Into<String>) {
        self.occupant_heraldry = Some(heraldry.into());
    }

    pub fn vacate(&mut self) {
        self.occupant_heraldry = None;
    }
}

/// The UI layout Genesis Container.
///
/// UIGenesis describes the slot topology of a UI layout. It uses the
/// standard 16-Mythos hierarchy: each Mythos = a layout region, each
/// Container = a slot group within that region, each Capsule = one slot.
///
/// Layouts can be world-specific (target_world_id set) or universal
/// (target_world_id = None), usable by any instrument in any world.
///
/// File extension: `.uigenesis`
/// Short alias:    `UIGen`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIGenesis {
    pub genesis_id: String,

    pub name: String,

    pub description: Option<String>,

    /// The world this layout is tuned for. None = universal.
    pub target_world_id: Option<String>,

    /// Visual theme hint for adapters:
    /// "futuristic-archivist" | "dark" | "light" | "minimal"
    pub theme_hint: String,

    /// The 16 layout region modules — one per LayoutRegion.
    pub mythos: Vec<MythosModule>,

    /// `draft` | `active` | `sealed`
    pub lifecycle: String,

    pub sealed: bool,

    pub seal: Option<SealBlock>,

    pub created_at: i64,

    pub schema_version: String,
}

impl UIGenesis {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            genesis_id: format!("ui_{}", uuid::Uuid::new_v4().simple()),
            name: name.into(),
            description: None,
            target_world_id: None,
            theme_hint: "futuristic-archivist".into(),
            mythos: Vec::new(),
            lifecycle: "draft".into(),
            sealed: false,
            seal: None,
            created_at: chrono::Utc::now().timestamp(),
            schema_version: "qgcp-v1.0".into(),
        }
    }

    pub fn with_theme(mut self, theme: impl Into<String>) -> Self {
        self.theme_hint = theme.into();
        self
    }

    pub fn for_world(mut self, world_id: impl Into<String>) -> Self {
        self.target_world_id = Some(world_id.into());
        self
    }

    pub fn add_mythos(&mut self, module: MythosModule) -> Result<(), QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        if self.mythos.len() >= MAX_MYTHOS {
            return Err(QgcpError::MythosOverflow(self.mythos.len() + 1));
        }
        self.mythos.push(module);
        Ok(())
    }

    /// Flatten all slot-definition capsules across all regions.
    /// Returns (region_label, slot_definition) pairs in layout order.
    pub fn all_slots(&self) -> Vec<SlotDefinition> {
        let mut slots = Vec::new();
        for mythos in &self.mythos {
            for container in &mythos.containers {
                for capsule in &container.capsules {
                    if let Ok(slot) = serde_json::from_value::<SlotDefinition>(
                        capsule.payload.clone()
                    ) {
                        slots.push(slot);
                    }
                }
            }
        }
        slots
    }

    /// All vacant slots — what plugins can request.
    pub fn vacant_slots(&self) -> Vec<SlotDefinition> {
        self.all_slots().into_iter().filter(|s| s.is_vacant()).collect()
    }

    /// All slots in a specific region.
    pub fn slots_in_region(&self, region: &LayoutRegion) -> Vec<SlotDefinition> {
        self.all_slots().into_iter().filter(|s| &s.region == region).collect()
    }

    pub fn seal(&mut self, sealed_by: impl Into<String>) -> Result<&SealBlock, QgcpError> {
        if self.sealed {
            return Err(QgcpError::Sealed);
        }
        let content = serde_json::to_string(&self.mythos)?;
        let hash = blake3::hash(content.as_bytes());
        self.lifecycle = "sealed".into();
        self.sealed = true;
        self.seal = Some(SealBlock::new(hex::encode(hash.as_bytes()), sealed_by));
        Ok(self.seal.as_ref().unwrap())
    }

    pub fn to_json(&self) -> Result<String, QgcpError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> Result<Self, QgcpError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Build the default "futuristic-archivist" layout with all 16 regions
    /// pre-populated as MythosModules with sensible default slot capsules.
    pub fn default_layout() -> Self {
        let mut ui = UIGenesis::new("Default Layout");

        for region in &LayoutRegion::ALL {
            let mut m = MythosModule::new(
                format!("UI-{:02}", region.label().replace(' ', "-").to_uppercase()),
                region.label(),
                WireType::Meta,
            );

            let slot = SlotDefinition::new(
                format!("slot_{}", region.label().to_lowercase().replace(' ', "_")),
                region.clone(),
            );

            let mut c = Container::new(
                format!("SLOT-{}", region.label().replace(' ', "-")),
                region.label(),
                WireType::Meta,
            );

            let cap = Capsule::new(
                region.label(),
                WireType::Meta,
                serde_json::to_value(&slot).unwrap_or_default(),
                vec!["layout".into(), "slot".into()],
                None,
            );

            let _ = c.add_capsule(cap);
            let _ = m.add_container(c);
            let _ = ui.add_mythos(m);
        }

        ui
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn law_of_16_regions() {
        assert_eq!(LayoutRegion::ALL.len(), 16);
    }

    #[test]
    fn ui_genesis_id_prefixed() {
        let ui = UIGenesis::new("Test Layout");
        assert!(ui.genesis_id.starts_with("ui_"));
    }

    #[test]
    fn default_layout_has_16_regions() {
        let ui = UIGenesis::default_layout();
        assert_eq!(ui.mythos.len(), 16);
    }

    #[test]
    fn all_slots_readable() {
        let ui = UIGenesis::default_layout();
        let slots = ui.all_slots();
        assert_eq!(slots.len(), 16);
    }

    #[test]
    fn all_slots_start_vacant() {
        let ui = UIGenesis::default_layout();
        assert!(ui.vacant_slots().len() == 16);
    }

    #[test]
    fn slots_in_region_filter() {
        let ui = UIGenesis::default_layout();
        let main = ui.slots_in_region(&LayoutRegion::CanvasMain);
        assert_eq!(main.len(), 1);
    }

    #[test]
    fn seal_prevents_mutation() {
        let mut ui = UIGenesis::new("Test");
        ui.seal("test").unwrap();
        let m = MythosModule::new("LATE", "Late", WireType::Meta);
        assert!(matches!(ui.add_mythos(m), Err(QgcpError::Sealed)));
    }
}
