// VAULT-ATOM-05: Relationship Weaver — graph linkages between capsules
use mythos::identity::MythId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct RelationshipWeaver {
    edges: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl RelationshipWeaver {
    pub fn link(&self, from: &MythId, to: &MythId) {
        self.edges
            .write()
            .unwrap()
            .entry(from.as_str())
            .or_default()
            .insert(to.as_str());
    }

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
