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

use crate::utils::load_image;
use fyrox_texture::TextureResource;
use std::sync::LazyLock;

macro_rules! built_in_image {
    ($name:ident = $path:expr) => {
        pub static $name: LazyLock<Option<TextureResource>> =
            LazyLock::new(|| load_image(include_bytes!($path)));
    };
}

built_in_image!(BITS_ICON = "bits.png");
built_in_image!(REVERT_ICON = "revert.png");
built_in_image!(FOLDER_ICON = "folder.png");
built_in_image!(BIND_ICON = "chain.png");
built_in_image!(CHECK = "check.png");
built_in_image!(ADD = "add.png");
built_in_image!(REMOVE = "cross.png");
