// VAULT-ATOM-05: Relationship Weaver — directed graph linkages between capsules.
//
// Maintains a directed edge graph: "capsule A is related to capsule B".
// Used by the higher narrative layers to represent character relationships,
// event chains, object ownership, and faction ties.
// Edges are keyed by string IDs for stability across renames.

use myth_wire::MythId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct RelationshipWeaver {
    edges: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl RelationshipWeaver {
    /// Create a directed edge from → to.
    pub fn link(&self, from: &MythId, to: &MythId) {
        self.edges
            .write()
            .unwrap()
            .entry(from.as_str())
            .or_default()
            .insert(to.as_str());
    }

    /// Remove a directed edge.
    pub fn unlink(&self, from: &MythId, to: &MythId) {
        if let Some(neighbors) = self.edges.write().unwrap().get_mut(&from.as_str()) {
            neighbors.remove(&to.as_str());
        }
    }

    /// All capsules directly related to `id` (outgoing edges only).
    pub fn neighbors(&self, id: &MythId) -> Vec<String> {
        self.edges
            .read()
            .unwrap()
            .get(&id.as_str())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }
}
