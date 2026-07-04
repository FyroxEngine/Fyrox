// CORE-ATOM-05: State Machine Supervisor — node lifecycle state tracking.
//
// Tracks the lifecycle state of every known Vault, Genesis, and Module node
// that has registered with Core. Provides atomic state transitions.
// The SSoT for "is this node alive and what is it doing?"

use myth_wire::MythId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    Booting,
    Ready,
    Simulating,
    Paused,
    Draining,
    Dead,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NodeKind {
    Vault,
    Genesis,
    Module,
    Theater,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: MythId,
    pub kind: NodeKind,
    pub state: NodeState,
}

#[derive(Default)]
pub struct StateMachineSupervisor {
    nodes: Arc<RwLock<HashMap<String, NodeRecord>>>,
}

impl StateMachineSupervisor {
    pub fn register(&self, id: MythId, kind: NodeKind) {
        self.nodes.write().unwrap().insert(
            id.as_str(),
            NodeRecord { id, kind, state: NodeState::Booting },
        );
    }

    /// Transition `id` to `next`. Returns false if the node is not registered.
    pub fn transition(&self, id: &MythId, next: NodeState) -> bool {
        if let Some(rec) = self.nodes.write().unwrap().get_mut(&id.as_str()) {
            rec.state = next;
            true
        } else {
            false
        }
    }

    pub fn state(&self, id: &MythId) -> Option<NodeState> {
        self.nodes
            .read()
            .unwrap()
            .get(&id.as_str())
            .map(|r| r.state)
    }

    pub fn all_nodes(&self) -> Vec<NodeRecord> {
        self.nodes.read().unwrap().values().cloned().collect()
    }
}
