// VAULT-ATOM-11: Cache Orchestrator — LRU cache for active simulation modules
use mythos::identity::MythId;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

const DEFAULT_CAPACITY: usize = 256;

pub struct CacheOrchestrator {
    capacity: usize,
    store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    lru_order: Arc<RwLock<VecDeque<String>>>,
}

impl CacheOrchestrator {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            store: Arc::new(RwLock::new(HashMap::new())),
            lru_order: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub fn insert(&self, id: &MythId, data: Vec<u8>) {
        let key = id.as_str();
        {
            let mut order = self.lru_order.write().unwrap();
            order.retain(|k| k != &key);
            order.push_back(key.clone());
            if order.len() > self.capacity {
                if let Some(evict_key) = order.pop_front() {
                    self.store.write().unwrap().remove(&evict_key);
                }
            }
        }
        self.store.write().unwrap().insert(key, data);
    }

    pub fn get(&self, id: &MythId) -> Option<Vec<u8>> {
        let key = id.as_str();
        let data = self.store.read().unwrap().get(&key).cloned()?;
        // Promote to MRU position.
        let mut order = self.lru_order.write().unwrap();
        order.retain(|k| k != &key);
        order.push_back(key);
        Some(data)
    }

    pub fn invalidate(&self, id: &MythId) {
        let key = id.as_str();
        self.store.write().unwrap().remove(&key);
        self.lru_order.write().unwrap().retain(|k| k != &key);
    }
}

impl Default for CacheOrchestrator {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}
