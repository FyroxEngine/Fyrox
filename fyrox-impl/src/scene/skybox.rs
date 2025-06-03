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

//! Skybox is a huge box around a camera. See [`SkyBox`] docs for more info.

use crate::{
    asset::{builtin::BuiltInResource, embedded_data_source, untyped::ResourceKind},
    core::{log::Log, reflect::prelude::*, uuid_provider, visitor::prelude::*},
};
use fyrox_texture::{
    CompressionOptions, Texture, TextureImportOptions, TextureKind, TextureMinificationFilter,
    TexturePixelKind, TextureResource, TextureResourceExtension, TextureWrapMode,
};
use lazy_static::lazy_static;
use uuid::{uuid, Uuid};

/// Skybox is a huge box around a camera. Each face has its own texture, when textures are
/// properly made, there is no seams and you get good decoration which contains static
/// skies and/or some other objects (mountains, buildings, etc.). Usually skyboxes used
/// in outdoor scenes, however real use of it limited only by your imagination. Skybox
/// will be drawn first, none of objects could be drawn before skybox.
#[derive(Debug, Clone, Default, PartialEq, Reflect, Visit, Eq)]
pub struct SkyBox {
    /// Texture for front face.
    #[reflect(setter = "set_front")]
    pub(crate) front: Option<TextureResource>,

    /// Texture for back face.
    #[reflect(setter = "set_back")]
    pub(crate) back: Option<TextureResource>,

    /// Texture for left face.
    #[reflect(setter = "set_left")]
    pub(crate) left: Option<TextureResource>,

    /// Texture for right face.
    #[reflect(setter = "set_right")]
    pub(crate) right: Option<TextureResource>,

    /// Texture for top face.
    #[reflect(setter = "set_top")]
    pub(crate) top: Option<TextureResource>,

    /// Texture for bottom face.
    #[reflect(setter = "set_bottom")]
    pub(crate) bottom: Option<TextureResource>,

    /// Cubemap texture
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) cubemap: Option<TextureResource>,
}

uuid_provider!(SkyBox = "45f359f1-e26f-4ace-81df-097f63474c72");

impl SkyBox {
    /// Returns cubemap texture
    pub fn cubemap(&self) -> Option<TextureResource> {
        self.cubemap.clone()
    }

    /// Returns cubemap texture
    pub fn cubemap_ref(&self) -> Option<&TextureResource> {
        self.cubemap.as_ref()
    }

    /// Validates input set of texture and checks if it possible to create a cube map from them.
    /// There are two main conditions for successful cube map creation:
    /// - All textures must have same width and height, and width must be equal to height.
    /// - All textures must have same pixel kind.
    pub fn validate(&self) -> Result<(), SkyBoxError> {
        struct TextureInfo {
            pixel_kind: TexturePixelKind,
            width: u32,
            height: u32,
        }

        let mut first_info: Option<TextureInfo> = None;

        for (index, texture) in self.textures().iter().enumerate() {
            if let Some(texture) = texture {
                if let Some(texture) = texture.state().data() {
                    if let TextureKind::Rectangle { width, height } = texture.kind() {
                        if width != height {
                            return Err(SkyBoxError::NonSquareTexture {
                                index,
                                width,
                                height,
                            });
                        }

                        if let Some(first_info) = first_info.as_mut() {
                            if first_info.width != width
                                || first_info.height != height
                                || first_info.pixel_kind != texture.pixel_kind()
                            {
                                return Err(SkyBoxError::DifferentTexture {
                                    expected_width: first_info.width,
                                    expected_height: first_info.height,
                                    expected_pixel_kind: first_info.pixel_kind,
                                    index,
                                    actual_width: width,
                                    actual_height: height,
                                    actual_pixel_kind: texture.pixel_kind(),
                                });
                            }
                        } else {
                            first_info = Some(TextureInfo {
                                pixel_kind: texture.pixel_kind(),
                                width,
                                height,
                            });
                        }
                    }
                } else {
                    return Err(SkyBoxError::TextureIsNotReady { index });
                }
            }
        }

        Ok(())
    }

    /// Creates a cubemap using provided faces. If some face has not been provided corresponding side will be black.
    ///
    /// # Important notes.
    ///
    /// It will fail if provided face's kind is not TextureKind::Rectangle.
    pub fn create_cubemap(&mut self) -> Result<(), SkyBoxError> {
        self.validate()?;

        let (kind, pixel_kind, bytes_per_face) =
            self.textures().iter().find(|face| face.is_some()).map_or(
                (
                    TextureKind::Rectangle {
                        width: 1,
                        height: 1,
                    },
                    TexturePixelKind::R8,
                    1,
                ),
                |face| {
                    let face = face.clone().unwrap();
                    let data = face.data_ref();

                    (data.kind(), data.pixel_kind(), data.mip_level_data(0).len())
                },
            );

        let size = match kind {
            TextureKind::Rectangle { width, height } => {
                assert_eq!(width, height);
                width
            }
            _ => return Err(SkyBoxError::UnsupportedTextureKind(kind)),
        };

        let mut data = Vec::<u8>::with_capacity(bytes_per_face * 6);
        for face in self.textures().iter() {
            if let Some(f) = face.clone() {
                data.extend(f.data_ref().mip_level_data(0));
            } else {
                let black_face_data = vec![0; bytes_per_face];
                data.extend(black_face_data);
            }
        }

        let cubemap = TextureResource::from_bytes(
            Uuid::new_v4(),
            TextureKind::Cube { size },
            pixel_kind,
            data,
            ResourceKind::Embedded,
        )
        .ok_or(SkyBoxError::UnableToBuildCubeMap)?;

        let mut cubemap_ref = cubemap.data_ref();
        cubemap_ref.set_s_wrap_mode(TextureWrapMode::ClampToEdge);
        cubemap_ref.set_t_wrap_mode(TextureWrapMode::ClampToEdge);
        drop(cubemap_ref);

        self.cubemap = Some(cubemap);

        Ok(())
    }

    /// Returns slice with all textures, where: 0 - Left, 1 - Right, 2 - Top, 3 - Bottom
    /// 4 - Front, 5 - Back.
    ///
    /// # Important notes.
    ///
    /// These textures are **not** used for rendering! The renderer uses cube map made of these
    /// textures. Public access for these textures is needed in case you need to read internals
    /// of the textures.
    pub fn textures(&self) -> [Option<TextureResource>; 6] {
        [
            self.left.clone(),
            self.right.clone(),
            self.top.clone(),
            self.bottom.clone(),
            self.front.clone(),
            self.back.clone(),
        ]
    }

    /// Set new texture for the left side of the skybox.
    pub fn set_left(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.left, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for left face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn left(&self) -> Option<TextureResource> {
        self.left.clone()
    }

    /// Set new texture for the right side of the skybox.
    pub fn set_right(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.right, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for right face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn right(&self) -> Option<TextureResource> {
        self.right.clone()
    }

    /// Set new texture for the top side of the skybox.
    pub fn set_top(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.top, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for top face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn top(&self) -> Option<TextureResource> {
        self.top.clone()
    }

    /// Set new texture for the bottom side of the skybox.
    pub fn set_bottom(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.bottom, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for bottom face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn bottom(&self) -> Option<TextureResource> {
        self.bottom.clone()
    }

    /// Set new texture for the front side of the skybox.
    pub fn set_front(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.front, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for front face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn front(&self) -> Option<TextureResource> {
        self.front.clone()
    }

    /// Set new texture for the back side of the skybox.
    pub fn set_back(&mut self, texture: Option<TextureResource>) -> Option<TextureResource> {
        let prev = std::mem::replace(&mut self.back, texture);
        Log::verify(self.create_cubemap());
        prev
    }

    /// Returns a texture that is used for back face of the cube map.
    ///
    /// # Important notes.
    ///
    /// This textures is not used for rendering! The renderer uses cube map made of face textures.
    pub fn back(&self) -> Option<TextureResource> {
        self.back.clone()
    }
}

/// An error that may occur during skybox creation.
#[derive(Debug)]
pub enum SkyBoxError {
    /// Texture kind is not TextureKind::Rectangle
    UnsupportedTextureKind(TextureKind),
    /// Cube map was failed to build.
    UnableToBuildCubeMap,
    /// Input texture is not square.
    NonSquareTexture {
        /// Texture index.
        index: usize,
        /// Width of the faulty texture.
        width: u32,
        /// Height of the faulty texture.
        height: u32,
    },
    /// Some input texture differs in size or pixel kind.
    DifferentTexture {
        /// Actual width of the first valid texture in the input set.
        expected_width: u32,
        /// Actual height of the first valid texture in the input set.
        expected_height: u32,
        /// Actual pixel kind of the first valid texture in the input set.
        expected_pixel_kind: TexturePixelKind,
        /// Index of the faulty input texture.
        index: usize,
        /// Width of the faulty texture.
        actual_width: u32,
        /// Height of the faulty texture.
        actual_height: u32,
        /// Pixel kind of the faulty texture.
        actual_pixel_kind: TexturePixelKind,
    },
    /// Occurs when one of the input textures is either still loading or failed to load.
    TextureIsNotReady {
        /// Index of the faulty input texture.
        index: usize,
    },
}

/// SkyBox builder is used to create new skybox in declarative manner.
pub struct SkyBoxBuilder {
    /// Texture for front face.
    pub front: Option<TextureResource>,
    /// Texture for back face.
    pub back: Option<TextureResource>,
    /// Texture for left face.
    pub left: Option<TextureResource>,
    /// Texture for right face.
    pub right: Option<TextureResource>,
    /// Texture for top face.
    pub top: Option<TextureResource>,
    /// Texture for bottom face.
    pub bottom: Option<TextureResource>,
}

impl SkyBoxBuilder {
    /// Sets desired front face of cubemap.
    pub fn with_front(mut self, texture: TextureResource) -> Self {
        self.front = Some(texture);
        self
    }

    /// Sets desired back face of cubemap.
    pub fn with_back(mut self, texture: TextureResource) -> Self {
        self.back = Some(texture);
        self
    }

    /// Sets desired left face of cubemap.
    pub fn with_left(mut self, texture: TextureResource) -> Self {
        self.left = Some(texture);
        self
    }

    /// Sets desired right face of cubemap.
    pub fn with_right(mut self, texture: TextureResource) -> Self {
        self.right = Some(texture);
        self
    }

    /// Sets desired top face of cubemap.
    pub fn with_top(mut self, texture: TextureResource) -> Self {
        self.top = Some(texture);
        self
    }

    /// Sets desired front face of cubemap.
    pub fn with_bottom(mut self, texture: TextureResource) -> Self {
        self.bottom = Some(texture);
        self
    }

    /// Creates a new instance of skybox.
    pub fn build(self) -> Result<SkyBox, SkyBoxError> {
        let mut skybox = SkyBox {
            left: self.left,
            right: self.right,
            top: self.top,
            bottom: self.bottom,
            front: self.front,
            back: self.back,
            cubemap: None,
        };

        skybox.create_cubemap()?;

        Ok(skybox)
    }
}

fn load_texture(id: Uuid, data: &[u8]) -> TextureResource {
    TextureResource::load_from_memory(
        id,
        ResourceKind::External,
        data,
        TextureImportOptions::default()
            .with_compression(CompressionOptions::NoCompression)
            .with_minification_filter(TextureMinificationFilter::Linear),
    )
    .ok()
    .unwrap()
}

lazy_static! {
    static ref BUILT_IN_SKYBOX_FRONT: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_FRONT",
        embedded_data_source!("skybox/front.png"),
        |data| { load_texture(uuid!("f8d4519b-2947-4c83-9aa5-800a70ae918e"), data) }
    );
    static ref BUILT_IN_SKYBOX_BACK: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_BACK",
        embedded_data_source!("skybox/back.png"),
        |data| { load_texture(uuid!("28676705-58bd-440f-b0aa-ce42cf95be79"), data) }
    );
    static ref BUILT_IN_SKYBOX_TOP: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_TOP",
        embedded_data_source!("skybox/top.png"),
        |data| { load_texture(uuid!("03e38da7-53d1-48c0-87f8-2baf9869d61d"), data) }
    );
    static ref BUILT_IN_SKYBOX_BOTTOM: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_BOTTOM",
        embedded_data_source!("skybox/bottom.png"),
        |data| { load_texture(uuid!("01684dc1-34b2-48b3-b8c2-30a7718cb9e7"), data) }
    );
    static ref BUILT_IN_SKYBOX_LEFT: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_LEFT",
        embedded_data_source!("skybox/left.png"),
        |data| { load_texture(uuid!("1725b779-7633-477a-a7b0-995c079c3202"), data) }
    );
    static ref BUILT_IN_SKYBOX_RIGHT: BuiltInResource<Texture> = BuiltInResource::new(
        "__BUILT_IN_SKYBOX_RIGHT",
        embedded_data_source!("skybox/right.png"),
        |data| { load_texture(uuid!("5f74865a-3eae-4bff-8743-b9d1f7bb3c59"), data) }
    );
    static ref BUILT_IN_SKYBOX: SkyBox = SkyBoxKind::make_built_in_skybox();
}

impl SkyBoxKind {
    fn make_built_in_skybox() -> SkyBox {
        let front = BUILT_IN_SKYBOX_FRONT.resource();
        let back = BUILT_IN_SKYBOX_BACK.resource();
        let top = BUILT_IN_SKYBOX_TOP.resource();
        let bottom = BUILT_IN_SKYBOX_BOTTOM.resource();
        let left = BUILT_IN_SKYBOX_LEFT.resource();
        let right = BUILT_IN_SKYBOX_RIGHT.resource();

        SkyBoxBuilder {
            front: Some(front),
            back: Some(back),
            left: Some(left),
            right: Some(right),
            top: Some(top),
            bottom: Some(bottom),
        }
        .build()
        .unwrap()
    }

    /// Returns a references to built-in sky box.
    pub fn built_in_skybox() -> &'static SkyBox {
        &BUILT_IN_SKYBOX
    }

    /// Returns an array with references to the textures being used in built-in sky box. The order is:
    /// front, back, top, bottom, left, right.
    pub fn built_in_skybox_textures() -> [&'static BuiltInResource<Texture>; 6] {
        [
            &BUILT_IN_SKYBOX_FRONT,
            &BUILT_IN_SKYBOX_BACK,
            &BUILT_IN_SKYBOX_TOP,
            &BUILT_IN_SKYBOX_BOTTOM,
            &BUILT_IN_SKYBOX_LEFT,
            &BUILT_IN_SKYBOX_RIGHT,
        ]
    }
}

/// A fixed set of possible sky boxes, that can be selected when building [`Camera`] scene node.
#[derive(Default)]
pub enum SkyBoxKind {
    /// Uses built-in sky box. This is default sky box.
    #[default]
    Builtin,
    /// No sky box. Surroundings will be filled with back buffer clear color.
    None,
    /// Specific skybox. One can be built using [`SkyBoxBuilder`].
    Specific(SkyBox),
}
