use serde::{Deserialize, Serialize};

use crate::midi::MidiBinding;

// ── Split direction ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SplitDir {
    None,
    Vertical,
    Horizontal,
}

// ── Panel node ────────────────────────────────────────────────────────────────

/// A node in the recursive panel-split tree.
///
/// Leaf nodes (no children) represent actual UI zones that get assigned to
/// an instrument control or Theatre channel. Interior nodes are pure geometry
/// — they split their allocated rect among their children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelNode {
    pub id:       u64,
    pub name:     String,

    /// Current flex weight (relative to siblings). Drives split ratio.
    pub flex:        f32,
    /// Lerp target set by MIDI or animation. Renderer moves `flex` toward this.
    pub flex_target: f32,

    pub split:    SplitDir,
    pub children: Vec<PanelNode>,

    /// Background fill color [r, g, b, a].
    pub bg:       [u8; 4],
    pub show_bg:  bool,

    /// Optional MIDI CC binding that controls this node's flex ratio live.
    pub midi: Option<MidiBinding>,

    /// Arbitrary slot ID — links this leaf to an instrument control or Theatre channel.
    /// e.g. "filter_cutoff", "ch_12", "scope_a".
    pub slot_id: Option<String>,
}

impl PanelNode {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            name:        format!("Panel {id}"),
            flex:        1.0,
            flex_target: 1.0,
            split:       SplitDir::None,
            children:    vec![],
            bg:          [14, 14, 32, 200],
            show_bg:     true,
            midi:        None,
            slot_id:     None,
        }
    }

    pub fn is_leaf(&self) -> bool { self.children.is_empty() }

    // ── Tree mutation ─────────────────────────────────────────────────────────

    /// Split this leaf into `count` equal children along `dir`.
    /// No-op if this node already has children.
    pub fn split_into(&mut self, dir: SplitDir, count: usize, next_id: &mut u64) {
        if self.split != SplitDir::None { return; }
        self.split = dir;
        self.children = (0..count).map(|_| {
            let node = PanelNode::new(*next_id);
            *next_id += 1;
            node
        }).collect();
    }

    /// Remove all children and become a leaf again.
    pub fn collapse(&mut self) {
        self.split = SplitDir::None;
        self.children.clear();
    }

    /// Find a node by id anywhere in the subtree.
    pub fn find_mut(&mut self, id: u64) -> Option<&mut PanelNode> {
        if self.id == id { return Some(self); }
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) { return Some(found); }
        }
        None
    }

    pub fn find(&self, id: u64) -> Option<&PanelNode> {
        if self.id == id { return Some(self); }
        for child in &self.children {
            if let Some(found) = child.find(id) { return Some(found); }
        }
        None
    }

    /// Remove a child by id from anywhere in the subtree.
    pub fn remove_child(&mut self, id: u64) -> bool {
        let before = self.children.len();
        self.children.retain(|c| c.id != id);
        if self.children.len() != before {
            if self.children.is_empty() { self.split = SplitDir::None; }
            return true;
        }
        for child in &mut self.children {
            if child.remove_child(id) { return true; }
        }
        false
    }

    // ── MIDI ──────────────────────────────────────────────────────────────────

    /// Walk the tree looking for any node bound to `channel` + `cc`.
    /// If found, set its `flex_target` from the CC value.
    /// Call this from your MIDI input handler each time a CC message arrives.
    pub fn apply_midi(&mut self, channel: u8, cc: u8, value: u8) {
        if let Some(binding) = &self.midi {
            if binding.channel == channel && binding.cc == cc {
                self.flex_target = binding.flex_from_cc(value);
            }
        }
        for child in &mut self.children {
            child.apply_midi(channel, cc, value);
        }
    }

    /// Lerp `flex` toward `flex_target` each frame.
    /// Call from your render/update loop with delta time in seconds.
    /// `speed` controls how fast it catches up — 8.0 feels snappy, 3.0 is smooth.
    pub fn tick(&mut self, dt: f32, speed: f32) {
        let diff = self.flex_target - self.flex;
        if diff.abs() > 0.001 {
            self.flex += diff * (speed * dt).min(1.0);
        } else {
            self.flex = self.flex_target;
        }
        for child in &mut self.children {
            child.tick(dt, speed);
        }
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    /// All leaf (id, rect) pairs given the root rect. Uses current `flex` values.
    #[cfg(feature = "egui-layout")]
    pub fn layout_leaves(&self, rect: egui::Rect) -> Vec<(u64, egui::Rect)> {
        layout_leaves(self, rect)
    }

    /// All nodes (id, rect, depth) — useful for drawing the editor canvas.
    #[cfg(feature = "egui-layout")]
    pub fn layout_all(&self, rect: egui::Rect) -> Vec<(u64, egui::Rect, usize)> {
        layout_all(self, rect, 0)
    }

    /// Flat list of all leaf slot IDs with their computed normalized rects.
    /// Normalized = 0.0–1.0 coords relative to root — resolution independent.
    pub fn layout_normalized(&self) -> Vec<(u64, NormalizedRect)> {
        let root = NormalizedRect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
        layout_normalized(self, root)
    }
}

// ── Normalized rect (no egui dep) ────────────────────────────────────────────

/// A rect in 0.0–1.0 space relative to the root container.
/// Multiply by actual window size to get pixels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

fn layout_normalized(node: &PanelNode, rect: NormalizedRect) -> Vec<(u64, NormalizedRect)> {
    if node.is_leaf() {
        return vec![(node.id, rect)];
    }
    let total: f32 = node.children.iter().map(|c| c.flex).sum::<f32>().max(0.001);
    let mut out = vec![];
    match node.split {
        SplitDir::Vertical => {
            let mut x = rect.x;
            for child in &node.children {
                let w = rect.w * child.flex / total;
                out.extend(layout_normalized(child, NormalizedRect { x, y: rect.y, w, h: rect.h }));
                x += w;
            }
        }
        SplitDir::Horizontal => {
            let mut y = rect.y;
            for child in &node.children {
                let h = rect.h * child.flex / total;
                out.extend(layout_normalized(child, NormalizedRect { x: rect.x, y, w: rect.w, h }));
                y += h;
            }
        }
        SplitDir::None => out.push((node.id, rect)),
    }
    out
}

// ── egui layout helpers (feature-gated) ──────────────────────────────────────

#[cfg(feature = "egui-layout")]
fn layout_leaves(node: &PanelNode, rect: egui::Rect) -> Vec<(u64, egui::Rect)> {
    if node.is_leaf() { return vec![(node.id, rect)]; }
    let total: f32 = node.children.iter().map(|c| c.flex).sum::<f32>().max(0.001);
    let mut out = vec![];
    match node.split {
        SplitDir::Vertical => {
            let mut x = rect.left();
            for child in &node.children {
                let w = rect.width() * child.flex / total;
                let cr = egui::Rect::from_min_size(
                    egui::Pos2::new(x, rect.top()), egui::Vec2::new(w, rect.height()));
                out.extend(layout_leaves(child, cr));
                x += w;
            }
        }
        SplitDir::Horizontal => {
            let mut y = rect.top();
            for child in &node.children {
                let h = rect.height() * child.flex / total;
                let cr = egui::Rect::from_min_size(
                    egui::Pos2::new(rect.left(), y), egui::Vec2::new(rect.width(), h));
                out.extend(layout_leaves(child, cr));
                y += h;
            }
        }
        SplitDir::None => out.push((node.id, rect)),
    }
    out
}

#[cfg(feature = "egui-layout")]
fn layout_all(node: &PanelNode, rect: egui::Rect, depth: usize)
    -> Vec<(u64, egui::Rect, usize)>
{
    let mut out = vec![(node.id, rect, depth)];
    if node.is_leaf() { return out; }
    let total: f32 = node.children.iter().map(|c| c.flex).sum::<f32>().max(0.001);
    match node.split {
        SplitDir::Vertical => {
            let mut x = rect.left();
            for child in &node.children {
                let w = rect.width() * child.flex / total;
                let cr = egui::Rect::from_min_size(
                    egui::Pos2::new(x, rect.top()), egui::Vec2::new(w, rect.height()));
                out.extend(layout_all(child, cr, depth + 1));
                x += w;
            }
        }
        SplitDir::Horizontal => {
            let mut y = rect.top();
            for child in &node.children {
                let h = rect.height() * child.flex / total;
                let cr = egui::Rect::from_min_size(
                    egui::Pos2::new(rect.left(), y), egui::Vec2::new(rect.width(), h));
                out.extend(layout_all(child, cr, depth + 1));
                y += h;
            }
        }
        SplitDir::None => {}
    }
    out
}
