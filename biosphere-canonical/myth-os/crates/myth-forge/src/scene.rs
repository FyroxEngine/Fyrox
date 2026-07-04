use egui::{Color32, Pos2, Vec2};
use serde::{Deserialize, Serialize};

// Re-export the panel tree from myth-stencil so the rest of myth-forge
// can keep using the same names without changes.
pub use myth_stencil::node::{PanelNode, SplitDir};

// ── Atom types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AtomKind {
    Knob, Fader, Pad, Jack, Meter, Scope, Label, Divider,
}

impl AtomKind {
    pub fn label(&self) -> &'static str {
        match self {
            AtomKind::Knob    => "Knob",
            AtomKind::Fader   => "Fader",
            AtomKind::Pad     => "Pad",
            AtomKind::Jack    => "Jack",
            AtomKind::Meter   => "Meter",
            AtomKind::Scope   => "Scope",
            AtomKind::Label   => "Label",
            AtomKind::Divider => "Divider",
        }
    }

    pub fn default_size(&self) -> Vec2 {
        match self {
            AtomKind::Knob    => Vec2::new(72.0, 88.0),
            AtomKind::Fader   => Vec2::new(32.0, 140.0),
            AtomKind::Pad     => Vec2::new(60.0, 60.0),
            AtomKind::Jack    => Vec2::new(32.0, 40.0),
            AtomKind::Meter   => Vec2::new(20.0, 120.0),
            AtomKind::Scope   => Vec2::new(200.0, 120.0),
            AtomKind::Label   => Vec2::new(100.0, 20.0),
            AtomKind::Divider => Vec2::new(4.0, 120.0),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            AtomKind::Knob    => "◎",
            AtomKind::Fader   => "▮",
            AtomKind::Pad     => "▣",
            AtomKind::Jack    => "◉",
            AtomKind::Meter   => "▐",
            AtomKind::Scope   => "⌇",
            AtomKind::Label   => "T",
            AtomKind::Divider => "┃",
        }
    }
}

// ── Atom params ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomParams {
    pub label:     String,
    pub value:     f32,
    pub color:     [u8; 4],
    pub secondary: [u8; 4],
    pub size_px:   f32,
    pub text:      String,
    pub is_output: bool,
    pub lit:       bool,
}

impl Default for AtomParams {
    fn default() -> Self {
        Self {
            label:     String::new(),
            value:     0.5,
            color:     [0, 200, 255, 255],
            secondary: [57, 255, 20, 255],
            size_px:   56.0,
            text:      "LABEL".into(),
            is_output: false,
            lit:       false,
        }
    }
}

impl AtomParams {
    pub fn color32(&self) -> Color32 {
        Color32::from_rgba_unmultiplied(
            self.color[0], self.color[1], self.color[2], self.color[3])
    }
}

// ── Placed atom ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedAtom {
    pub id:     u64,
    pub kind:   AtomKind,
    pub pos:    [f32; 2],
    pub size:   [f32; 2],
    pub params: AtomParams,
    pub layer:  u8,
    pub locked: bool,
}

impl PlacedAtom {
    pub fn new(id: u64, kind: AtomKind, pos: Pos2) -> Self {
        let size = kind.default_size();
        Self { id, pos: [pos.x, pos.y], size: [size.x, size.y],
               params: AtomParams::default(), layer: 0, locked: false, kind }
    }

    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(
            Pos2::new(self.pos[0], self.pos[1]),
            Vec2::new(self.size[0], self.size[1]))
    }
}

// ── Panel tree helpers (egui-specific color conversion) ───────────────────────

pub fn panel_bg_color(node: &PanelNode) -> Color32 {
    Color32::from_rgba_unmultiplied(node.bg[0], node.bg[1], node.bg[2], node.bg[3])
}

// ── Saved component (reusable atom group) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name:        String,
    pub description: String,
    pub atoms:       Vec<PlacedAtom>,  // positions are relative to bounding box origin
}

impl Component {
    pub fn from_atoms(name: &str, description: &str, atoms: &[&PlacedAtom]) -> Self {
        if atoms.is_empty() {
            return Self { name: name.into(), description: description.into(), atoms: vec![] };
        }
        // Normalise to bounding-box origin
        let min_x = atoms.iter().map(|a| a.pos[0]).fold(f32::MAX, f32::min);
        let min_y = atoms.iter().map(|a| a.pos[1]).fold(f32::MAX, f32::min);
        let normalised = atoms.iter().map(|a| {
            let mut copy = (*a).clone();
            copy.pos[0] -= min_x;
            copy.pos[1] -= min_y;
            copy
        }).collect();
        Self { name: name.into(), description: description.into(), atoms: normalised }
    }

    /// Instantiate at `origin`, assigning fresh ids starting from `next_id`.
    pub fn instantiate(&self, origin: Pos2, next_id: &mut u64) -> Vec<PlacedAtom> {
        self.atoms.iter().map(|a| {
            let mut copy = a.clone();
            copy.id    = *next_id;
            *next_id  += 1;
            copy.pos[0] += origin.x;
            copy.pos[1] += origin.y;
            copy
        }).collect()
    }
}

// ── Canvas scene ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasScene {
    pub name:       String,
    pub module_tag: String,
    pub atoms:      Vec<PlacedAtom>,
    pub root:       PanelNode,
    pub bg_color:   [u8; 4],
    next_id:        u64,
}

impl Default for CanvasScene {
    fn default() -> Self {
        Self {
            name: "New Instrument".into(),
            module_tag: "INST".into(),
            atoms: vec![],
            root: PanelNode::new(0),
            bg_color: [8, 8, 18, 255],
            next_id: 1,
        }
    }
}

impl CanvasScene {
    pub fn new(name: &str, module_tag: &str) -> Self {
        Self { name: name.into(), module_tag: module_tag.into(), ..Default::default() }
    }

    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1; id
    }

    pub fn add_atom(&mut self, kind: AtomKind, pos: Pos2) -> u64 {
        let id = self.next_id();
        self.atoms.push(PlacedAtom::new(id, kind, pos));
        id
    }

    pub fn remove_atom(&mut self, id: u64) { self.atoms.retain(|a| a.id != id); }

    pub fn get_atom_mut(&mut self, id: u64) -> Option<&mut PlacedAtom> {
        self.atoms.iter_mut().find(|a| a.id == id)
    }

    pub fn move_to_front(&mut self, id: u64) {
        if let Some(i) = self.atoms.iter().position(|a| a.id == id) {
            let atom = self.atoms.remove(i);
            self.atoms.push(atom);
        }
    }

    pub fn split_panel(&mut self, panel_id: u64, dir: SplitDir, count: usize) {
        if let Some(node) = self.root.find_mut(panel_id) {
            node.split_into(dir, count, &mut self.next_id);
        }
    }

    pub fn collapse_panel(&mut self, panel_id: u64) {
        if let Some(node) = self.root.find_mut(panel_id) {
            node.collapse();
        }
    }

    pub fn remove_panel(&mut self, id: u64) {
        if id == self.root.id { return; }
        self.root.remove_child(id);
    }

    pub fn add_atoms_from_component(&mut self, comp: &Component, origin: Pos2) -> Vec<u64> {
        let instances = comp.instantiate(origin, &mut self.next_id);
        let ids: Vec<u64> = instances.iter().map(|a| a.id).collect();
        self.atoms.extend(instances);
        ids
    }
}
