// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{NodeWrapper, PrefabData, SceneGraph};
use fyrox_core::{futures::executor::block_on, info};
use fyrox_resource::{Resource, manager::ResourceManager};
use std::collections::HashSet;

pub struct ResourceGraphVertex<P: PrefabData> {
    resource: Resource<P>,
    children: Vec<ResourceGraphVertex<P>>,
}

impl<P: PrefabData> ResourceGraphVertex<P> {
    pub fn new(model: Resource<P>, resource_manager: ResourceManager) -> Self {
        let mut children = Vec::new();

        // false positive, resource hash and equality check performed by comparing pointers.
        #[allow(clippy::mutable_key_type)]
        let mut dependent_resources = HashSet::new();

        // Look for dependent resources.
        for resource in resource_manager.state().iter() {
            if let Some(other_model) = resource.try_cast::<P>() {
                let mut state = other_model.state();
                if let Some(model_data) = state.data()
                    && model_data.graph().linear_iter().any(|n| {
                        n.resource().as_ref().map(|r| r.clone().into_untyped())
                            == Some(model.clone().into_untyped())
                    })
                {
                    dependent_resources.insert(other_model.clone());
                }
            }
        }

        children.extend(
            dependent_resources
                .into_iter()
                .map(|r| ResourceGraphVertex::new(r, resource_manager.clone())),
        );

        Self {
            resource: model,
            children,
        }
    }

    pub fn resolve(&self) {
        info!(
            "Resolving {} resource from dependency graph...",
            self.resource.resource_uuid()
        );

        // Wait until resource is fully loaded, then resolve.
        if block_on(self.resource.clone()).is_ok() {
            for child in self.children.iter() {
                child.resolve();
            }
        }
    }
}

pub struct ResourceDependencyGraph<P: PrefabData> {
    root: ResourceGraphVertex<P>,
}

impl<P: PrefabData> ResourceDependencyGraph<P> {
    pub fn new(model: Resource<P>, resource_manager: ResourceManager) -> Self {
        Self {
            root: ResourceGraphVertex::new(model, resource_manager),
        }
    }

    pub fn resolve(&self) {
        self.root.resolve()
    }
}
