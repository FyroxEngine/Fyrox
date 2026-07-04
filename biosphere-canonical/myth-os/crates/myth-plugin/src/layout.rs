/// Layout slot types that plugins can request.
///
/// These mirror UIGenesis `LayoutRegion` but as typed Rust enums so plugin
/// code gets compile-time guarantees. The registry converts between the two.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SlotType {
    HeaderLeft,
    HeaderCenter,
    HeaderRight,
    CanvasToolbar,
    CanvasLeft,
    CanvasMain,
    CanvasRight,
    CanvasStatusBar,
    FooterLeft,
    FooterCenter,
    FooterRight,
    OverlayModal,
    OverlayDrawer,
    OverlayTooltip,
    Notification,
    ContextMenu,
}

impl SlotType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HeaderLeft      => "header_left",
            Self::HeaderCenter    => "header_center",
            Self::HeaderRight     => "header_right",
            Self::CanvasToolbar   => "canvas_toolbar",
            Self::CanvasLeft      => "canvas_left",
            Self::CanvasMain      => "canvas_main",
            Self::CanvasRight     => "canvas_right",
            Self::CanvasStatusBar => "canvas_status_bar",
            Self::FooterLeft      => "footer_left",
            Self::FooterCenter    => "footer_center",
            Self::FooterRight     => "footer_right",
            Self::OverlayModal    => "overlay_modal",
            Self::OverlayDrawer   => "overlay_drawer",
            Self::OverlayTooltip  => "overlay_tooltip",
            Self::Notification    => "notification",
            Self::ContextMenu     => "context_menu",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "header_left"      => Self::HeaderLeft,
            "header_center"    => Self::HeaderCenter,
            "header_right"     => Self::HeaderRight,
            "canvas_toolbar"   => Self::CanvasToolbar,
            "canvas_left"      => Self::CanvasLeft,
            "canvas_main"      => Self::CanvasMain,
            "canvas_right"     => Self::CanvasRight,
            "canvas_status_bar"=> Self::CanvasStatusBar,
            "footer_left"      => Self::FooterLeft,
            "footer_center"    => Self::FooterCenter,
            "footer_right"     => Self::FooterRight,
            "overlay_modal"    => Self::OverlayModal,
            "overlay_drawer"   => Self::OverlayDrawer,
            "overlay_tooltip"  => Self::OverlayTooltip,
            "notification"     => Self::Notification,
            "context_menu"     => Self::ContextMenu,
            _                  => return None,
        })
    }
}

/// How a plugin wants its slot to appear by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Always rendered, never collapses.
    Always,
    /// Rendered but hidden until the user or system reveals it.
    OnDemand,
    /// Not rendered unless explicitly triggered.
    Hidden,
}

/// A single slot request from a plugin.
#[derive(Debug, Clone)]
pub struct SlotRequest {
    /// The slot type / region this plugin wants.
    pub slot_type: SlotType,
    /// Human label shown in the UI (e.g. "Plugin Foundry — Main Canvas").
    pub label: String,
    /// Default visibility preference.
    pub visibility: Visibility,
    /// Optional stable slot_id to request a specific named slot.
    /// If None the registry picks the first available slot in that region.
    pub preferred_slot_id: Option<String>,
    /// Width hint in logical pixels. None = fill available.
    pub preferred_width: Option<f32>,
    /// Height hint in logical pixels. None = fill available.
    pub preferred_height: Option<f32>,
}

impl SlotRequest {
    pub fn new(slot_type: SlotType, label: impl Into<String>) -> Self {
        Self {
            slot_type,
            label: label.into(),
            visibility: Visibility::Always,
            preferred_slot_id: None,
            preferred_width: None,
            preferred_height: None,
        }
    }

    pub fn on_demand(mut self) -> Self {
        self.visibility = Visibility::OnDemand;
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visibility = Visibility::Hidden;
        self
    }

    pub fn with_size(mut self, w: f32, h: f32) -> Self {
        self.preferred_width = Some(w);
        self.preferred_height = Some(h);
        self
    }

    pub fn prefer_slot(mut self, slot_id: impl Into<String>) -> Self {
        self.preferred_slot_id = Some(slot_id.into());
        self
    }
}

/// A collection of slot requests — what a plugin asks for at startup.
#[derive(Debug, Clone, Default)]
pub struct LayoutRequest {
    pub requests: Vec<SlotRequest>,
}

impl LayoutRequest {
    pub fn new() -> Self { Self::default() }

    pub fn add(mut self, req: SlotRequest) -> Self {
        self.requests.push(req);
        self
    }

    pub fn is_empty(&self) -> bool { self.requests.is_empty() }
}

/// A successfully granted slot.
#[derive(Debug, Clone)]
pub struct GrantedSlot {
    /// The stable slot_id assigned (matches UIGenesis SlotDefinition).
    pub slot_id: String,
    /// Which type was granted — may differ from request if alternatives applied.
    pub slot_type: SlotType,
    /// The label the plugin requested.
    pub label: String,
}

/// A denied slot with context so the plugin can respond intelligently.
#[derive(Debug, Clone)]
pub struct DeniedSlot {
    /// Which request this is in response to.
    pub requested_type: SlotType,
    pub label: String,
    /// The heraldry of whoever currently holds that slot.
    /// Shown in UI as "Venturan is in the Left Panel."
    pub occupant_heraldry: Option<String>,
    /// Alternative slots the registry is willing to offer instead.
    pub alternatives_offered: Vec<SlotType>,
    /// Human-readable reason string.
    pub reason: String,
}

/// Full outcome of one `negotiate_layout()` call.
#[derive(Debug, Clone, Default)]
pub struct LayoutGrant {
    pub granted: Vec<GrantedSlot>,
    pub denied: Vec<DeniedSlot>,
}

impl LayoutGrant {
    pub fn all_granted(&self) -> bool { self.denied.is_empty() }
    pub fn any_granted(&self) -> bool { !self.granted.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_type_round_trips() {
        let all = [
            SlotType::HeaderLeft, SlotType::HeaderCenter, SlotType::HeaderRight,
            SlotType::CanvasToolbar, SlotType::CanvasLeft, SlotType::CanvasMain,
            SlotType::CanvasRight, SlotType::CanvasStatusBar,
            SlotType::FooterLeft, SlotType::FooterCenter, SlotType::FooterRight,
            SlotType::OverlayModal, SlotType::OverlayDrawer, SlotType::OverlayTooltip,
            SlotType::Notification, SlotType::ContextMenu,
        ];
        assert_eq!(all.len(), 16);
        for st in &all {
            let s = st.as_str();
            let back = SlotType::from_str(s).unwrap();
            assert_eq!(&back, st);
        }
    }

    #[test]
    fn layout_request_builder() {
        let req = LayoutRequest::new()
            .add(SlotRequest::new(SlotType::CanvasLeft, "My Panel").on_demand())
            .add(SlotRequest::new(SlotType::HeaderRight, "Icon").with_size(32.0, 32.0));
        assert_eq!(req.requests.len(), 2);
        assert_eq!(req.requests[0].visibility, Visibility::OnDemand);
        assert_eq!(req.requests[1].preferred_width, Some(32.0));
    }

    #[test]
    fn layout_grant_all_granted_when_no_denials() {
        let grant = LayoutGrant {
            granted: vec![GrantedSlot {
                slot_id: "slot_canvas_left".into(),
                slot_type: SlotType::CanvasLeft,
                label: "My Panel".into(),
            }],
            denied: vec![],
        };
        assert!(grant.all_granted());
        assert!(grant.any_granted());
    }
}
