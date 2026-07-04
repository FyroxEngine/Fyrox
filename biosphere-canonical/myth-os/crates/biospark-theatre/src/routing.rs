// THEATRE-ROUTING: ChannelRouter — directed signal graph with cycle detection.
//
// Channels can feed output from one layer into another as input.
// Any route that would create a cycle is rejected — cycles cause infinite feedback.
//
// DFS-based cycle detection: O(V + E), runs before every new edge is committed.

use crate::TheatreError;
use myth_wire::ChannelId;
use std::collections::{HashMap, HashSet};

/// Directed signal routing graph between Theatre channels.
///
/// A connection from channel A → channel B means A's output feeds B's input.
/// All connections are validated for cycles before being stored.
#[derive(Debug, Default)]
pub struct ChannelRouter {
    /// Adjacency list: channel_id → destination channel_ids.
    edges: HashMap<u32, Vec<u32>>,
}

impl ChannelRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Route `from` channel's output into `to` channel's input.
    ///
    /// Returns `RoutingCycle` if the connection would close a loop.
    /// The graph is not modified on failure.
    pub fn connect(&mut self, from: ChannelId, to: ChannelId) -> Result<(), TheatreError> {
        let from = from.get();
        let to = to.get();

        // Add tentatively.
        self.edges.entry(from).or_default().push(to);

        if self.has_cycle() {
            // Roll back.
            if let Some(v) = self.edges.get_mut(&from) {
                v.retain(|&x| x != to);
            }
            return Err(TheatreError::RoutingCycle(from));
        }

        Ok(())
    }

    /// Remove the route from `from` to `to`. No-op if the route doesn't exist.
    pub fn disconnect(&mut self, from: ChannelId, to: ChannelId) {
        if let Some(v) = self.edges.get_mut(&from.get()) {
            v.retain(|&x| x != to.get());
        }
    }

    /// Get all channel IDs that `from` feeds into.
    pub fn destinations(&self, from: ChannelId) -> &[u32] {
        self.edges
            .get(&from.get())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Whether any channel routes to `to` (i.e., `to` has an incoming edge).
    pub fn has_incoming(&self, to: ChannelId) -> bool {
        let target = to.get();
        self.edges.values().any(|dests| dests.contains(&target))
    }

    /// Whether the routing graph contains any cycles.
    ///
    /// Uses DFS coloring: unvisited → in-stack → done.
    pub fn has_cycle(&self) -> bool {
        let mut visited: HashSet<u32> = HashSet::new();
        let mut stack: HashSet<u32> = HashSet::new();

        for &node in self.edges.keys() {
            if !visited.contains(&node)
                && self.dfs_has_cycle(node, &mut visited, &mut stack)
            {
                return true;
            }
        }
        false
    }

    fn dfs_has_cycle(
        &self,
        node: u32,
        visited: &mut HashSet<u32>,
        stack: &mut HashSet<u32>,
    ) -> bool {
        visited.insert(node);
        stack.insert(node);

        if let Some(neighbors) = self.edges.get(&node) {
            for &next in neighbors {
                if !visited.contains(&next) {
                    if self.dfs_has_cycle(next, visited, stack) {
                        return true;
                    }
                } else if stack.contains(&next) {
                    return true; // back edge → cycle
                }
            }
        }

        stack.remove(&node);
        false
    }

    /// Remove all routes.
    pub fn clear(&mut self) {
        self.edges.clear();
    }

    /// Number of unique connections in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(n: u32) -> ChannelId {
        ChannelId::new(n)
    }

    #[test]
    fn simple_chain_has_no_cycle() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(1)).unwrap();
        r.connect(ch(1), ch(2)).unwrap();
        assert!(!r.has_cycle());
    }

    #[test]
    fn direct_cycle_is_rejected() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(1)).unwrap();
        let err = r.connect(ch(1), ch(0));
        assert!(matches!(err, Err(TheatreError::RoutingCycle(_))));
        // Graph should be unchanged — still only 1 edge.
        assert_eq!(r.edge_count(), 1);
    }

    #[test]
    fn triangle_cycle_is_rejected() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(1)).unwrap();
        r.connect(ch(1), ch(2)).unwrap();
        let err = r.connect(ch(2), ch(0));
        assert!(matches!(err, Err(TheatreError::RoutingCycle(_))));
        assert_eq!(r.edge_count(), 2);
    }

    #[test]
    fn self_loop_is_rejected() {
        let mut r = ChannelRouter::new();
        let err = r.connect(ch(0), ch(0));
        assert!(matches!(err, Err(TheatreError::RoutingCycle(_))));
    }

    #[test]
    fn disconnect_removes_edge() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(1)).unwrap();
        r.disconnect(ch(0), ch(1));
        assert_eq!(r.edge_count(), 0);
    }

    #[test]
    fn destinations_returns_correct_targets() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(1)).unwrap();
        r.connect(ch(0), ch(2)).unwrap();
        let mut dests = r.destinations(ch(0)).to_vec();
        dests.sort();
        assert_eq!(dests, vec![1, 2]);
    }

    #[test]
    fn has_incoming_detects_target() {
        let mut r = ChannelRouter::new();
        r.connect(ch(0), ch(3)).unwrap();
        assert!(r.has_incoming(ch(3)));
        assert!(!r.has_incoming(ch(0)));
    }
}
