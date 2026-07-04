// VAULT-ATOM-06: Hierarchy Mapper — parent-child relationship tree.
//
// Tracks the parent of every capsule and can walk the full ancestry path
// from a leaf to the root. Used by the Registry and higher-level layers
// to understand container nesting (Genesis → Mythos → Container → Capsule).

use myth_wire::MythId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct HierarchyMapper {
    parent_of: Arc<RwLock<HashMap<String, String>>>,
}

impl HierarchyMapper {
    pub fn set_parent(&self, child: &MythId, parent: &MythId) {
        self.parent_of
            .write()
            .unwrap()
            .insert(child.as_str(), parent.as_str());
    }

    /// Walk up the parent chain from `id`, returning the full path root-first.
    pub fn path(&self, id: &MythId) -> Vec<String> {
        let map = self.parent_of.read().unwrap();
        let mut path = vec![id.as_str()];
        let mut current = id.as_str();
        loop {
            match map.get(&current) {
                Some(parent) => {
                    path.push(parent.clone());
                    current = parent.clone();
                }
                None => break,
            }
        }
        path.reverse();
        path
    }
}
