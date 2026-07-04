// THEATRE-LAYOUT: LayoutBlueprint — panel partition presets for egui.
//
// When a vault is created or a layer is added, the user picks one of these
// blueprints. The Theatre compositor uses it to partition the egui canvas
// into named zones (header, footer, sidebar, main, etc.) that channels
// are assigned to.
//
// Translated from the React flexbox layout system in biospark-theater-with-flex-vaults.

use serde::{Deserialize, Serialize};

/// Named panel-partition presets for the Theatre canvas.
///
/// Each variant maps to an egui panel arrangement used when the Theatre
/// compositor renders the vault's layer stack to the viewport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LayoutBlueprint {
    /// A single full-viewport canvas — no splits. Default for Stage vaults.
    #[default]
    Fullscreen,

    /// Single column: header bar / main content / footer bar.
    /// Classic presentation stack.
    PancakeStack,

    /// Sidebar (left) + main content area, side by side.
    SidebarLeft,

    /// Holy Grail: header / (sidebar | main | sidebar) / footer.
    /// Maximum structural layout.
    HolyGrail,

    /// 12-column responsive grid overlay.
    /// Channels snap to column spans (1–12).
    TwelveColumn,

    /// Two equal vertical panes separated by a divider.
    SplitScreen,

    /// Pinterest-style variable-height card grid (3 columns).
    Masonry,

    /// Single child centered both horizontally and vertically.
    PerfectCenter,

    /// Content area that grows + fixed footer pinned to the bottom.
    StickyFooter,
}

impl LayoutBlueprint {
    /// Short identifier used in serialised manifests and UI labels.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Fullscreen   => "fullscreen",
            Self::PancakeStack => "pancake-stack",
            Self::SidebarLeft  => "sidebar-left",
            Self::HolyGrail    => "holy-grail",
            Self::TwelveColumn => "twelve-column",
            Self::SplitScreen  => "split-screen",
            Self::Masonry      => "masonry",
            Self::PerfectCenter => "perfect-center",
            Self::StickyFooter => "sticky-footer",
        }
    }

    /// Human-readable display title.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Fullscreen    => "Fullscreen",
            Self::PancakeStack  => "Pancake Stack",
            Self::SidebarLeft   => "Sidebar Left",
            Self::HolyGrail     => "Holy Grail",
            Self::TwelveColumn  => "12-Column Grid",
            Self::SplitScreen   => "Split Screen",
            Self::Masonry       => "Masonry",
            Self::PerfectCenter => "Perfect Center",
            Self::StickyFooter  => "Sticky Footer",
        }
    }

    /// Short description for tooltip / picker card.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Fullscreen    => "Single full-viewport canvas",
            Self::PancakeStack  => "Header / Content / Footer stack",
            Self::SidebarLeft   => "Left sidebar + main area",
            Self::HolyGrail     => "Header + two sidebars + footer",
            Self::TwelveColumn  => "12-column responsive grid",
            Self::SplitScreen   => "Two equal vertical panes",
            Self::Masonry       => "Variable-height card grid",
            Self::PerfectCenter => "Single child, centered",
            Self::StickyFooter  => "Expanding content + pinned footer",
        }
    }

    /// All variants in display order, for picker UIs.
    pub const ALL: &'static [LayoutBlueprint] = &[
        LayoutBlueprint::Fullscreen,
        LayoutBlueprint::PancakeStack,
        LayoutBlueprint::SidebarLeft,
        LayoutBlueprint::HolyGrail,
        LayoutBlueprint::TwelveColumn,
        LayoutBlueprint::SplitScreen,
        LayoutBlueprint::Masonry,
        LayoutBlueprint::PerfectCenter,
        LayoutBlueprint::StickyFooter,
    ];
}

impl std::fmt::Display for LayoutBlueprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.title())
    }
}
