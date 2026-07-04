use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// One atom definition loaded from `assets/atoms/seeds.toml`.
///
/// These are the raw definitions the ATOMS chemistry engine plants in the
/// quantum soup.  The engine reads tags for resonance matching and
/// workspace_status for filtering what can bond in the current build tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedAtom {
    pub id:               String,
    pub dept:             String,
    pub layer:            String,
    pub name:             String,
    pub function:         String,
    pub produces:         String,
    pub failure_mode:     String,
    pub tags:             Vec<String>,
    pub workspace_status: WorkspaceStatus,
    /// Which workspace crate already covers this atom (empty if conceptual).
    #[serde(default)]
    pub workspace_ref:    String,
}

impl SeedAtom {
    /// True when this atom can participate in the chemistry engine right now
    /// (either live or partial implementation exists to bond against).
    pub fn is_bondable(&self) -> bool {
        matches!(self.workspace_status, WorkspaceStatus::Live | WorkspaceStatus::Partial)
    }

    /// Resonance score between two atoms: how many tags they share.
    /// Higher = stronger bond candidate.
    pub fn resonance_with(&self, other: &SeedAtom) -> usize {
        self.tags.iter()
            .filter(|t| other.tags.contains(t))
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatus {
    /// Implemented in a workspace crate right now.
    Live,
    /// Scaffolding exists but not fully wired.
    Partial,
    /// On the Tier build queue.
    Planned,
    /// Idea captured; no Rust yet.
    Conceptual,
}

impl WorkspaceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Live       => "live",
            Self::Partial    => "partial",
            Self::Planned    => "planned",
            Self::Conceptual => "conceptual",
        }
    }
}

/// The full seed pool loaded from `seeds.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SeedFile {
    atoms: Vec<SeedAtom>,
}

/// The runtime atom pool.
///
/// Load once at boot via `AtomPool::load()`, then pass around as a shared ref.
#[derive(Debug, Clone)]
pub struct AtomPool {
    atoms: Vec<SeedAtom>,
    by_id: HashMap<String, usize>,
}

impl AtomPool {
    /// Load the pool from `assets/atoms/seeds.toml`.
    ///
    /// `workspace_root` is typically the directory returned by `env!("CARGO_MANIFEST_DIR")`
    /// walked up to the workspace root, or you can pass the absolute path directly.
    pub fn load(seeds_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(seeds_path)?;
        let file: SeedFile = toml::from_str(&content)?;
        let mut by_id = HashMap::new();
        for (i, atom) in file.atoms.iter().enumerate() {
            by_id.insert(atom.id.clone(), i);
        }
        Ok(Self { atoms: file.atoms, by_id })
    }

    /// All atoms in the pool.
    pub fn all(&self) -> &[SeedAtom] {
        &self.atoms
    }

    /// Atoms whose workspace_status is Live or Partial — can bond now.
    pub fn bondable(&self) -> impl Iterator<Item = &SeedAtom> {
        self.atoms.iter().filter(|a| a.is_bondable())
    }

    /// Atoms that need building (Planned or Conceptual).
    pub fn unbonded(&self) -> impl Iterator<Item = &SeedAtom> {
        self.atoms.iter().filter(|a| !a.is_bondable())
    }

    /// Look up an atom by its canonical ID.
    pub fn get(&self, id: &str) -> Option<&SeedAtom> {
        self.by_id.get(id).map(|&i| &self.atoms[i])
    }

    /// Find all atoms in a department.
    pub fn dept(&self, dept: &str) -> impl Iterator<Item = &SeedAtom> {
        let dept = dept.to_uppercase();
        self.atoms.iter().filter(move |a| a.dept == dept)
    }

    /// Candidate bond pairs sorted by resonance score (highest first).
    /// Only considers bondable atoms.
    pub fn bond_candidates(&self) -> Vec<(&SeedAtom, &SeedAtom, usize)> {
        let pool: Vec<_> = self.bondable().collect();
        let mut pairs = Vec::new();
        for (i, a) in pool.iter().enumerate() {
            for b in &pool[i + 1..] {
                let score = a.resonance_with(b);
                if score > 0 {
                    pairs.push((*a, *b, score));
                }
            }
        }
        pairs.sort_by(|x, y| y.2.cmp(&x.2));
        pairs
    }

    /// Summary stats.
    pub fn stats(&self) -> PoolStats {
        let mut stats = PoolStats::default();
        for a in &self.atoms {
            match a.workspace_status {
                WorkspaceStatus::Live       => stats.live       += 1,
                WorkspaceStatus::Partial    => stats.partial    += 1,
                WorkspaceStatus::Planned    => stats.planned    += 1,
                WorkspaceStatus::Conceptual => stats.conceptual += 1,
            }
        }
        stats.total = self.atoms.len();
        stats
    }
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub total:       usize,
    pub live:        usize,
    pub partial:     usize,
    pub planned:     usize,
    pub conceptual:  usize,
}

impl std::fmt::Display for PoolStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AtomPool [{} total] live:{} partial:{} planned:{} conceptual:{}",
            self.total, self.live, self.partial, self.planned, self.conceptual
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_pool() -> AtomPool {
        // mythos/ is one level below the workspace root, so parent() = J:\myth-os
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let seeds = manifest.parent().unwrap()
            .join("assets/atoms/seeds.toml");
        AtomPool::load(&seeds).expect("seeds.toml must be loadable")
    }

    #[test]
    fn pool_loads_without_error() {
        let pool = load_pool();
        assert!(pool.all().len() > 0, "pool must not be empty");
    }

    #[test]
    fn all_atoms_have_unique_ids() {
        let pool = load_pool();
        let mut seen = std::collections::HashSet::new();
        for atom in pool.all() {
            assert!(seen.insert(atom.id.clone()), "duplicate atom id: {}", atom.id);
        }
    }

    #[test]
    fn all_atoms_have_at_least_one_tag() {
        let pool = load_pool();
        for atom in pool.all() {
            assert!(!atom.tags.is_empty(), "atom {} has no tags", atom.id);
        }
    }

    #[test]
    fn sociomind_is_live() {
        let pool = load_pool();
        let atom = pool.get("SOCIOMIND_CORE").expect("SOCIOMIND_CORE must exist");
        assert_eq!(atom.workspace_status, WorkspaceStatus::Live);
    }

    #[test]
    fn memory_matrix_is_conceptual() {
        let pool = load_pool();
        let atom = pool.get("MEMORY_MATRIX").expect("MEMORY_MATRIX must exist");
        assert_eq!(atom.workspace_status, WorkspaceStatus::Conceptual);
    }

    #[test]
    fn bond_candidates_are_sorted_by_score() {
        let pool = load_pool();
        let pairs = pool.bond_candidates();
        let scores: Vec<usize> = pairs.iter().map(|(_, _, s)| *s).collect();
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(scores, sorted, "bond_candidates must be sorted highest-first");
    }

    #[test]
    fn stats_total_matches_all_count() {
        let pool = load_pool();
        let stats = pool.stats();
        assert_eq!(stats.total, pool.all().len());
        assert_eq!(
            stats.live + stats.partial + stats.planned + stats.conceptual,
            stats.total
        );
    }
}
