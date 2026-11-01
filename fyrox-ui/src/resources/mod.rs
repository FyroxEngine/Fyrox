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

use fyrox_resource::{builtin::BuiltInResource, embedded_data_source, untyped::ResourceKind};
use fyrox_texture::{
    CompressionOptions, Texture, TextureImportOptions, TextureMinificationFilter, TextureResource,
    TextureResourceExtension,
};
use std::sync::LazyLock;
use uuid::{Uuid, uuid};

pub fn load_image(id: Uuid, data: &[u8]) -> TextureResource {
    TextureResource::load_from_memory(
        id,
        ResourceKind::External,
        data,
        TextureImportOptions::default()
            .with_compression(CompressionOptions::NoCompression)
            .with_minification_filter(TextureMinificationFilter::LinearMipMapLinear)
            .with_lod_bias(-1.0),
    )
    .expect("built-in texture expected to be valid!")
}

#[macro_export]
macro_rules! load_image {
    ($path:expr) => {
        $crate::resources::load_image($crate::core::uuid::Uuid::new_v4(), include_bytes!($path))
    };
}

macro_rules! built_in_image {
    ($id:expr, $human_name:expr, $var_name:ident = $path:expr) => {
        pub static $var_name: LazyLock<BuiltInResource<Texture>> = LazyLock::new(|| {
            BuiltInResource::new($human_name, embedded_data_source!($path), |data| {
                load_image(uuid!($id), data)
            })
        });
    };
}

built_in_image!(
    "7dc2bead-ef1b-4ec7-a3ed-cccf0b1e1441",
    "Bits",
    BITS_ICON = "bits.png"
);
built_in_image!(
    "0baff2f9-0851-4339-a5ff-54ebdd0d9cd6",
    "Revert",
    REVERT_ICON = "revert.png"
);
built_in_image!(
    "164cdecf-713c-4d8a-acf9-29b32cf3ca01",
    "Folder",
    FOLDER_ICON = "folder.png"
);
built_in_image!(
    "a57660ec-ecf8-4f27-a987-aa0f18e420c6",
    "Bind",
    BIND_ICON = "chain.png"
);
built_in_image!(
    "a5e7847e-ac13-4c8e-b93c-7fffbba9b0b4",
    "Check",
    CHECK = "check.png"
);
built_in_image!(
    "697bb12f-758e-44bf-af93-2443c75d5435",
    "Add",
    ADD = "add.png"
);
built_in_image!(
    "b6eb9459-98a5-461d-a874-7f71ff25fada",
    "Remove",
    REMOVE = "cross.png"
);
built_in_image!(
    "7904ec28-9d75-4261-b130-106121214e78",
    "Shuffle",
    SHUFFLE = "shuffle.png"
);
built_in_image!(
    "c1291b65-a91a-4eb7-a38f-f55a3a1239d2",
    "Error",
    ERROR = "error.png"
);
built_in_image!(
    "6d192201-9113-4f29-8008-008919b5b863",
    "Warning",
    WARNING = "warning.png"
);
built_in_image!(
    "5b9b522d-1be9-429e-ba7a-850cd8fbd39c",
    "Info",
    INFO = "info.png"
);
built_in_image!(
    "73c47a0d-ceb8-411d-83f8-81979f31dcc9",
    "Brush",
    BRUSH = "brush.png"
);
built_in_image!(
    "9a1ac648-59ea-4996-bd25-6db02f13360c",
    "Thickness",
    THICKNESS = "thickness.png"
);
built_in_image!(
    "116f2b4a-b772-4a0c-b19f-f46a968185f7",
    "Number",
    NUMBER = "numbers.png"
);
built_in_image!(
    "87e6647b-a13f-4a8e-81d1-8192f642a69e",
    "Texture",
    TEXTURE = "texture.png"
);
built_in_image!(
    "b3ebf6e1-f6f4-43aa-a5cc-245984683708",
    "Color",
    COLOR = "colors.png"
);

pub fn all_resources() -> [BuiltInResource<Texture>; 16] {
    [
        BITS_ICON.clone(),
        REVERT_ICON.clone(),
        FOLDER_ICON.clone(),
        BIND_ICON.clone(),
        CHECK.clone(),
        ADD.clone(),
        REMOVE.clone(),
        SHUFFLE.clone(),
        ERROR.clone(),
        WARNING.clone(),
        INFO.clone(),
        BRUSH.clone(),
        THICKNESS.clone(),
        NUMBER.clone(),
        TEXTURE.clone(),
        COLOR.clone(),
    ]
}
