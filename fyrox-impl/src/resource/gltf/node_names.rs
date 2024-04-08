use crate::core::log::Log;
use crate::core::pool::Handle;
use crate::core::NameProvider;
use crate::fxhash::{FxHashMap, FxHashSet};
use crate::graph::BaseSceneGraph;
use crate::graph::SceneGraph;
use crate::scene::graph::Graph;
use crate::scene::node::Node;
use std::cell::RefCell;
use std::path::Path;

#[derive(Debug)]
struct NodeName {
    handle: Handle<Node>,
    name: RefCell<String>,
}

/// Attempt to remove duplicate names by renaming the nodes of the graph
/// without depending on the order of sibling nodes.
///
/// If a parent node has the same name as its child node, the child node
/// is renamed to "parent_name+1". Grandchildren are renamed to "parent_name+2"
/// and so on.
///
/// If two nodes have the same name, but their parents have different names,
/// the nodes are renamed to "node_name+parentA" and "node_name+parentB".
///
/// If the nodes have parents with the same names, then grandparent names may
/// be used if the grandparents have different names. If the whole ancestry
/// of the nodes is searched without finding ancestors with distinct names
/// then the nodes are not renamed.
///
/// If after the whole procedure there is found to still be nodes with duplicate names,
/// an error is logged to alert the user to the problem.
pub fn resolve_name_conflicts(path: &Path, graph: &mut Graph) {
    let node_sets: Vec<Vec<Handle<Node>>> = build_node_sets_from_graph(graph);
    for nodes in node_sets {
        if nodes.len() > 1 {
            resolve_conflict(nodes, graph);
        }
    }
    // Check if conflicts have actually been resolved.
    let mut name_set: FxHashSet<&str> = FxHashSet::default();
    for node in graph.linear_iter() {
        if !name_set.insert(node.name()) {
            Log::err(format!(
                "A node with existing name {} was found during the load of {} resource! \
                    Do **NOT IGNORE** this message, please fix names in your model, otherwise \
                    engine won't be able to correctly restore data from your resource!",
                node.name(),
                path.display()
            ));
        }
    }
}

fn build_node_sets_from_graph(graph: &Graph) -> Vec<Vec<Handle<Node>>> {
    let mut name_map: FxHashMap<&str, Vec<Handle<Node>>> = FxHashMap::default();
    for (handle, node) in graph.pair_iter() {
        let name = node.name();
        let list = name_map
            .entry(name)
            .or_insert_with(|| Vec::with_capacity(1));
        list.push(handle);
    }
    name_map.into_values().collect()
}

fn build_node_sets_from_list(nodes: &[NodeName]) -> Vec<Vec<&NodeName>> {
    let mut name_map: FxHashMap<&str, Vec<&NodeName>> = FxHashMap::default();
    let names: Vec<_> = nodes.iter().map(|n| n.name.borrow()).collect();
    for (node, name) in nodes.iter().zip(names.iter()) {
        let list = name_map
            .entry(name.as_str())
            .or_insert_with(|| Vec::with_capacity(1));
        list.push(node);
    }
    name_map.into_values().collect()
}

fn resolve_conflict(handles: Vec<Handle<Node>>, graph: &mut Graph) {
    // Resolve cases where parents have the same names as children.
    let mut node_names = build_node_name_list(handles.as_slice(), graph);
    for NodeName { handle, name } in node_names.iter_mut() {
        let d = count_ancestor_depth(*handle, handles.as_slice(), graph);
        if d > 0 {
            name.borrow_mut().push('+');
            name.borrow_mut().push_str(d.to_string().as_str());
        }
    }
    // Build lists of nodes which still have duplicate names
    let node_sets = build_node_sets_from_list(&node_names);
    // Add ancestor names if that will eliminate duplicates.
    for set in node_sets {
        if set.len() > 1 {
            if let Some(names) = find_distinct_ancestor_names(set.as_slice(), graph) {
                for (node, name) in set.iter().zip(names.iter()) {
                    node.name.borrow_mut().push('+');
                    node.name.borrow_mut().push_str(name);
                }
            }
        }
    }
    for node in node_names {
        graph[node.handle].set_name(node.name.borrow().as_str());
    }
}

fn build_node_name_list(handles: &[Handle<Node>], graph: &Graph) -> Vec<NodeName> {
    handles
        .iter()
        .map(|h| NodeName {
            handle: *h,
            name: RefCell::new(graph[*h].name().to_owned()),
        })
        .collect()
}

fn find_distinct_ancestor_names<'a>(
    node_names: &[&NodeName],
    graph: &'a Graph,
) -> Option<Vec<&'a str>> {
    for i in 1.. {
        if let Some(names) = get_ancestor_names(node_names, i, graph) {
            if !contains_duplicate_name(names.as_slice()) {
                return Some(names);
            }
        } else {
            return None;
        }
    }
    None
}

fn get_ancestor_names<'a>(
    node_names: &[&NodeName],
    depth: usize,
    graph: &'a Graph,
) -> Option<Vec<&'a str>> {
    let mut result: Vec<&'a str> = Vec::with_capacity(node_names.len());
    for n in node_names {
        if let Some(n) = get_ancestor_name(n.handle, depth, graph) {
            result.push(n);
        } else {
            return None;
        }
    }
    Some(result)
}

fn get_ancestor_name(mut handle: Handle<Node>, depth: usize, graph: &Graph) -> Option<&str> {
    for _ in 0..depth {
        if let Some(n) = graph.try_get(handle) {
            handle = n.parent();
        } else {
            return None;
        }
    }
    graph.try_get(handle).map(Node::name)
}

fn contains_duplicate_name(names: &[&str]) -> bool {
    let mut iter = names.iter();
    while let Some(n) = iter.next() {
        let mut rest = iter.clone();
        if rest.any(|x| *n == *x) {
            return true;
        }
    }
    false
}

fn count_ancestor_depth(mut handle: Handle<Node>, list: &[Handle<Node>], graph: &Graph) -> usize {
    let mut count: usize = 0;
    while let Some(node) = graph.try_get(handle) {
        handle = node.parent();
        if list.iter().any(|h| *h == handle) {
            count += 1;
        } else {
            break;
        }
    }
    count
}
