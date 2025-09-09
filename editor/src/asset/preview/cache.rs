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
        core::{futures::executor::block_on, parking_lot::Mutex, pool::Handle, SafeLock, Uuid},
        engine::Engine,
        fxhash::FxHashMap,
        gui::{message::MessageDirection, UiNode},
    },
    load_image,
};
use std::{
    collections::VecDeque,
    sync::{mpsc::Receiver, Arc},
};

pub struct IconRequest {
    pub widget_handle: Handle<UiNode>,
    pub resource: UntypedResource,
    pub force_update: bool,
}

pub struct AssetPreviewCache {
    container: FxHashMap<Uuid, AssetPreviewTexture>,
    throughput: usize,
    queue: Arc<Mutex<VecDeque<IconRequest>>>,
}

impl AssetPreviewCache {
    pub fn new(receiver: Receiver<IconRequest>, throughput: usize) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue2 = queue.clone();
        std::thread::spawn(move || {
            for request in receiver.iter() {
                let resource = request.resource.clone();
                if block_on(resource).is_ok() {
                    queue.safe_lock().push_back(request);
                }
            }
        });

        Self {
            container: Default::default(),
            throughput,
            queue: queue2,
        }
    }

    fn preview_for(
        &mut self,
        resource: &UntypedResource,
        generators: &mut AssetPreviewGeneratorsCollection,
        force_update: bool,
        generated_counter: &mut usize,
        engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        let resource_uuid = resource.resource_uuid();

        if let (false, Some(cached_preview)) = (force_update, self.container.get(&resource_uuid)) {
            return Some(cached_preview.clone());
        } else if let Some(generator) = resource
            .type_uuid()
            .and_then(|type_uuid| generators.map.get_mut(&type_uuid))
        {
            if let Some(preview) = generator.generate_preview(resource, engine) {
                *generated_counter += 1;
                self.container.insert(resource_uuid, preview.clone());
                return Some(preview);
            } else if let Some(icon) = generator.simple_icon(resource, &engine.resource_manager) {
                let preview = AssetPreviewTexture::from_texture_with_gray_tint(icon);
                self.container.insert(resource_uuid, preview.clone());
                return Some(preview);
            }
        }

        load_image!("../../../resources/asset.png").map(|placeholder_image| {
            AssetPreviewTexture::from_texture_with_gray_tint(placeholder_image)
        })
    }

    pub fn update(
        &mut self,
        generators: &mut AssetPreviewGeneratorsCollection,
        engine: &mut Engine,
    ) {
        let mut generated = 0;
        let queue = self.queue.clone();
        let mut queue = queue.safe_lock();
        while let Some(request) = queue.pop_back() {
            let IconRequest {
                widget_handle,
                resource,
                force_update,
            } = request;

            if let Some(preview) =
                self.preview_for(&resource, generators, force_update, &mut generated, engine)
            {
                let ui = engine.user_interfaces.first();

                ui.send_message(AssetItemMessage::icon(
                    widget_handle,
                    MessageDirection::ToWidget,
                    Some(preview.texture),
                    preview.flip_y,
                    preview.color,
                ));
            }

            if generated >= self.throughput {
                break;
            }
        }
    }
}
