use crate::{
    asset::{
        item::AssetItemMessage, preview::AssetPreviewGeneratorsCollection,
        preview::AssetPreviewTexture,
    },
    fyrox::{
        asset::untyped::{ResourceKind, UntypedResource},
        core::pool::Handle,
        engine::Engine,
        fxhash::FxHashMap,
        gui::{message::MessageDirection, UiNode},
        resource::texture::Texture,
    },
};
use std::sync::mpsc::Receiver;

pub struct IconRequest {
    pub asset_item: Handle<UiNode>,
    pub resource: UntypedResource,
}

pub struct AssetPreviewCache {
    receiver: Receiver<IconRequest>,
    container: FxHashMap<ResourceKind, AssetPreviewTexture>,
    throughput: usize,
}

impl AssetPreviewCache {
    pub fn new(receiver: Receiver<IconRequest>, throughput: usize) -> Self {
        Self {
            receiver,
            container: Default::default(),
            throughput,
        }
    }

    pub fn update(
        &mut self,
        generators: &mut AssetPreviewGeneratorsCollection,
        engine: &mut Engine,
    ) {
        for request in self.receiver.try_iter().take(self.throughput) {
            let IconRequest {
                asset_item,
                resource,
            } = request;

            let resource_kind = resource.kind();
            let preview = if let Some(cached_preview) = self.container.get(&resource_kind) {
                Some(cached_preview.clone())
            } else if let Some(generator) = generators.map.get_mut(&resource.type_uuid()) {
                if let Some(preview) = generator.generate_preview(&resource, engine) {
                    self.container.insert(resource_kind, preview.clone());
                    Some(preview)
                } else if let Some(icon) =
                    generator.simple_icon(&resource, &engine.resource_manager)
                {
                    let preview = AssetPreviewTexture {
                        texture: icon.try_cast::<Texture>().unwrap(),
                        flip_y: false,
                    };
                    self.container.insert(resource_kind, preview.clone());
                    Some(preview)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(preview) = preview {
                let ui = engine.user_interfaces.first();

                ui.send_message(AssetItemMessage::icon(
                    asset_item,
                    MessageDirection::ToWidget,
                    Some(preview.texture.into()),
                    preview.flip_y,
                ));
            }
        }
    }
}
