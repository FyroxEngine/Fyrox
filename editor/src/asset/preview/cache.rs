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

use crate::{
    asset::{
        item::AssetItemMessage, preview::AssetPreviewGeneratorsCollection,
        preview::AssetPreviewTexture,
    },
    fyrox::{
        asset::untyped::UntypedResource,
        core::pool::Handle,
        engine::Engine,
        fxhash::FxHashMap,
        gui::{message::MessageDirection, UiNode},
    },
};
use fyrox::core::Uuid;
use std::sync::mpsc::Receiver;

pub struct IconRequest {
    pub asset_item: Handle<UiNode>,
    pub resource: UntypedResource,
}

pub struct AssetPreviewCache {
    receiver: Receiver<IconRequest>,
    container: FxHashMap<Uuid, AssetPreviewTexture>,
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

            let preview = if let Some(resource_uuid) = resource.resource_uuid() {
                if let Some(cached_preview) = self.container.get(&resource_uuid) {
                    Some(cached_preview.clone())
                } else if let Some(generator) = resource
                    .type_uuid()
                    .and_then(|type_uuid| generators.map.get_mut(&type_uuid))
                {
                    if let Some(preview) = generator.generate_preview(&resource, engine) {
                        self.container.insert(resource_uuid, preview.clone());
                        Some(preview)
                    } else if let Some(icon) =
                        generator.simple_icon(&resource, &engine.resource_manager)
                    {
                        let preview = AssetPreviewTexture {
                            texture: icon,
                            flip_y: false,
                        };
                        self.container.insert(resource_uuid, preview.clone());
                        Some(preview)
                    } else {
                        None
                    }
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
                    Some(preview.texture),
                    preview.flip_y,
                ));
            }
        }
    }
}
