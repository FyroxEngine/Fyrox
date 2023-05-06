//! Resource dependency graph. See [`ResourceDependencyGraph`] docs for more info.

use crate::{collect_used_resources, state::ResourceState, untyped::UntypedResource};
use fxhash::FxHashSet;

/// A node of [`ResourceDependencyGraph`].
pub struct ResourceGraphNode {
    /// A resource associated with the graph node.
    pub resource: UntypedResource,
    /// A list of children nodes of the graph.
    pub children: Vec<ResourceGraphNode>,
}

impl ResourceGraphNode {
    /// Creates a new resource graph node for a given untyped resource. This method is recursive -
    /// it will initialize the entire sub-graph of dependencies automatically.
    pub fn new(resource: &UntypedResource) -> Self {
        let mut children = Vec::new();

        // Look for dependent resources.
        let mut dependent_resources = FxHashSet::default();

        let resource_state = resource.0.lock();
        if let ResourceState::Ok(resource_data) = &*resource_state {
            collect_used_resources(resource_data, &mut dependent_resources);
        }

        children.extend(
            dependent_resources
                .into_iter()
                .map(|r| ResourceGraphNode::new(&r)),
        );

        Self {
            resource: resource.clone(),
            children,
        }
    }
}

/// Resource dependency graph allows you to collect all dependencies of a resource in structured form.
/// Internally, it uses reflection to look into resources content and find dependent resources. An example
/// of dependent resource is very simple: if you have a 3D model, then it most likely has a bunch of
/// textures - these textures are dependent resources. A more complex example - a game level could depend
/// on lots of prefabs, which in their turn may depend on other prefabs, textures, sounds, etc.  
pub struct ResourceDependencyGraph {
    /// Root node of the graph.
    pub root: ResourceGraphNode,
}

impl ResourceDependencyGraph {
    /// Creates a new resource dependency graph starting from a given untyped resource.
    pub fn new(resource: &UntypedResource) -> Self {
        Self {
            root: ResourceGraphNode::new(resource),
        }
    }
}
