// CORE-ATOM-15: Emergency Failsafe — microsecond isolation of crashed modules
use mythos::identity::MythId;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use tracing::error;

#[derive(Default)]
pub struct EmergencyFailsafe {
    isolated: Arc<RwLock<HashSet<String>>>,
}

impl EmergencyFailsafe {
    pub fn isolate(&self, id: &MythId) {
        error!(node = %id, "FAILSAFE: isolating node");
        self.isolated.write().unwrap().insert(id.as_str());
    }

    pub fn release(&self, id: &MythId) {
        self.isolated.write().unwrap().remove(&id.as_str());
    }

    pub fn is_isolated(&self, id: &MythId) -> bool {
        self.isolated.read().unwrap().contains(&id.as_str())
    }

    pub fn isolated_count(&self) -> usize {
        self.isolated.read().unwrap().len()
    }
}
