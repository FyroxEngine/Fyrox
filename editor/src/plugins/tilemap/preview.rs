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
    asset::preview::{AssetPreviewGenerator, AssetPreviewTexture},
    fyrox::{
        asset::{manager::ResourceManager, untyped::UntypedResource},
        core::pool::Handle,
        engine::Engine,
        scene::{node::Node, tilemap::tileset::TileSet, Scene},
    },
    load_image,
};
use fyrox::resource::texture::TextureResource;

pub struct TileSetPreview;

impl AssetPreviewGenerator for TileSetPreview {
    fn generate_scene(
        &mut self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
        _scene: &mut Scene,
    ) -> Handle<Node> {
        Handle::NONE
    }

    fn generate_preview(
        &mut self,
        resource: &UntypedResource,
        _engine: &mut Engine,
    ) -> Option<AssetPreviewTexture> {
        let tile_set_resource = resource.try_cast::<TileSet>()?;
        let texture = tile_set_resource.state().data()?.preview_texture()?;
        Some(AssetPreviewTexture {
            texture,
            flip_y: false,
        })
    }

    fn simple_icon(
        &self,
        _resource: &UntypedResource,
        _resource_manager: &ResourceManager,
    ) -> Option<TextureResource> {
        load_image!("../../../resources/tile_set.png")
    }
}
