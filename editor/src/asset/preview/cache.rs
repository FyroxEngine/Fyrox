use crate::{
    asset::preview::AssetPreviewGeneratorsCollection,
    fyrox::{
        asset::untyped::ResourceKind, asset::untyped::UntypedResource, engine::Engine,
        fxhash::FxHashMap,
    },
};
use std::collections::VecDeque;

pub struct AssetPreviewCache {
    queue: VecDeque<UntypedResource>,
    container: FxHashMap<ResourceKind, UntypedResource>,
    throughput: usize,
}

impl Default for AssetPreviewCache {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            container: Default::default(),
            throughput: 4,
        }
    }
}

impl AssetPreviewCache {
    pub fn enqueue(&mut self, resource: UntypedResource) {
        self.queue.push_back(resource)
    }

    pub fn update(
        &mut self,
        generators: &mut AssetPreviewGeneratorsCollection,
        engine: &mut Engine,
    ) {
        for resource in self.queue.drain(0..self.throughput) {
            if let Some(generator) = generators.map.get_mut(&resource.type_uuid()) {
                match generator.generate_preview(&resource, engine) {
                    Some(preview) => {
                        self.container.insert(resource.kind(), preview.into());
                    }
                    None => {
                        if let Some(icon) =
                            generator.simple_icon(&resource, &engine.resource_manager)
                        {
                            self.container.insert(resource.kind(), icon);
                        }
                    }
                }
            }
        }
    }
}
