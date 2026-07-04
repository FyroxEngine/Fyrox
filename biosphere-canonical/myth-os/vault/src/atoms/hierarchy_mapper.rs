// VAULT-ATOM-06: Hierarchy Mapper — structural tree of containers/chambers
use mythos::identity::MythId;
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

    pub fn path(&self, id: &MythId) -> Vec<String> {
        let map = self.parent_of.read().unwrap();
        let mut path = vec![id.as_str()];
        let mut current = id.as_str();
        while let Some(parent) = map.get(&current) {
            path.push(parent.clone());
            current = parent.clone();
        }
        path.reverse();
        path
    }
}
