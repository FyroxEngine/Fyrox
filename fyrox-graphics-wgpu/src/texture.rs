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

use crate::server::WgpuGraphicsServer;
use fyrox_graphics::{
    error::FrameworkError,
    gpu_texture::{
        image_1d_size_bytes, image_2d_size_bytes, image_3d_size_bytes, GpuTextureDescriptor,
        GpuTextureKind, GpuTextureTrait, PixelKind,
    },
};
use std::cell::Cell;
use std::rc::Weak;

/// Maps a Fyrox [`PixelKind`] to the corresponding wgpu [`TextureFormat`].
///
/// 3-component formats (RGB8, BGR8, SRGB8, RGB16F, RGB32F) are mapped to their
/// 4-component equivalents because wgpu has no 3-component texture formats.
/// The actual data expansion happens in [`expand_to_rgba`].
fn pixel_kind_to_wgpu_format(kind: PixelKind) -> wgpu::TextureFormat {
    match kind {
        PixelKind::R32F => wgpu::TextureFormat::R32Float,
        PixelKind::R32UI => wgpu::TextureFormat::R32Uint,
        PixelKind::R16F => wgpu::TextureFormat::R16Float,
        PixelKind::RG16F => wgpu::TextureFormat::Rg16Float,
        PixelKind::D32F => wgpu::TextureFormat::Depth32Float,
        PixelKind::D16 => wgpu::TextureFormat::Depth16Unorm,
        PixelKind::D24S8 => wgpu::TextureFormat::Depth24PlusStencil8,
        PixelKind::RGBA8 | PixelKind::RGB8 => wgpu::TextureFormat::Rgba8Unorm,
        PixelKind::SRGBA8 | PixelKind::SRGB8 => wgpu::TextureFormat::Rgba8UnormSrgb,
        PixelKind::BGRA8 | PixelKind::BGR8 => wgpu::TextureFormat::Bgra8Unorm,
        PixelKind::RG8 | PixelKind::LA8 => wgpu::TextureFormat::Rg8Unorm,
        PixelKind::R8 | PixelKind::L8 => wgpu::TextureFormat::R8Unorm,
        PixelKind::R8UI => wgpu::TextureFormat::R8Uint,
        PixelKind::RGBA16F => wgpu::TextureFormat::Rgba16Float,
        PixelKind::RGB16F => wgpu::TextureFormat::Rgba16Float,
        PixelKind::RGBA32F | PixelKind::RGB32F => wgpu::TextureFormat::Rgba32Float,
        PixelKind::R11G11B10F => wgpu::TextureFormat::Rg11b10Ufloat,
        PixelKind::RGB10A2 => wgpu::TextureFormat::Rgb10a2Unorm,
        _ => wgpu::TextureFormat::Rgba8Unorm,
    }
}

/// Returns true if the pixel kind is 3-component and needs expansion to 4-component
/// for wgpu compatibility (wgpu has no 3-component texture formats).
fn needs_rgba_expansion(pk: PixelKind) -> bool {
    matches!(
        pk,
        PixelKind::RGB8
            | PixelKind::BGR8
            | PixelKind::SRGB8
            | PixelKind::RGB16F
            | PixelKind::RGB32F
    )
}

/// Expands 3-component pixel data to 4-component by adding an opaque alpha channel.
fn expand_to_rgba(pk: PixelKind, data: &[u8]) -> Vec<u8> {
    match pk {
        PixelKind::RGB8 | PixelKind::SRGB8 | PixelKind::BGR8 => {
            // 3 bytes → 4 bytes per pixel (add 0xFF alpha)
            let pixel_count = data.len() / 3;
            let mut out = Vec::with_capacity(pixel_count * 4);
            for chunk in data.chunks(3) {
                out.extend_from_slice(chunk);
                out.push(0xFF);
            }
            out
        }
        PixelKind::RGB16F => {
            // 6 bytes → 8 bytes per pixel (add 1.0f16 = 0x3C00 as alpha)
            let pixel_count = data.len() / 6;
            let mut out = Vec::with_capacity(pixel_count * 8);
            for chunk in data.chunks(6) {
                out.extend_from_slice(chunk);
                out.extend_from_slice(&[0x00, 0x3C]);
            }
            out
        }
        PixelKind::RGB32F => {
            // 12 bytes → 16 bytes per pixel (add 1.0f32 = 0x3F800000 as alpha)
            let pixel_count = data.len() / 12;
            let mut out = Vec::with_capacity(pixel_count * 16);
            for chunk in data.chunks(12) {
                out.extend_from_slice(chunk);
                out.extend_from_slice(&[0x00, 0x00, 0x80, 0x3F]);
            }
            out
        }
        _ => data.to_vec(),
    }
}

fn texture_dimension(kind: GpuTextureKind) -> wgpu::TextureDimension {
    match kind {
        GpuTextureKind::Line { .. } => wgpu::TextureDimension::D1,
        GpuTextureKind::Rectangle { .. } | GpuTextureKind::Cube { .. } => {
            wgpu::TextureDimension::D2
        }
        GpuTextureKind::Volume { .. } => wgpu::TextureDimension::D3,
    }
}

pub(crate) fn texture_size(kind: GpuTextureKind) -> (u32, u32, u32) {
    match kind {
        GpuTextureKind::Line { length } => (length as u32, 1, 1),
        GpuTextureKind::Rectangle { width, height } => (width as u32, height as u32, 1),
        GpuTextureKind::Cube { size } => (size as u32, size as u32, 6),
        GpuTextureKind::Volume {
            width,
            height,
            depth,
        } => (width as u32, height as u32, depth as u32),
    }
}

fn texture_view_dimension(kind: GpuTextureKind) -> wgpu::TextureViewDimension {
    match kind {
        GpuTextureKind::Line { .. } => wgpu::TextureViewDimension::D1,
        GpuTextureKind::Rectangle { .. } => wgpu::TextureViewDimension::D2,
        GpuTextureKind::Cube { .. } => wgpu::TextureViewDimension::Cube,
        GpuTextureKind::Volume { .. } => wgpu::TextureViewDimension::D3,
    }
}

/// Writes a single mip level (or cubemap face) to a texture, expanding 3-component
/// pixel data to 4-component if needed for wgpu compatibility.
#[allow(clippy::too_many_arguments)]
fn write_mip_data(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip: u32,
    origin: wgpu::Origin3d,
    extent: wgpu::Extent3d,
    pk: PixelKind,
    data: &[u8],
    fmt: wgpu::TextureFormat,
) {
    let expanded;
    let upload_data = if needs_rgba_expansion(pk) {
        expanded = expand_to_rgba(pk, data);
        expanded.as_slice()
    } else {
        data
    };
    let bps = fmt.block_copy_size(None).unwrap_or(4);
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: mip,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        upload_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some((extent.width * bps).max(1)),
            rows_per_image: Some(extent.height.max(1)),
        },
        extent,
    );
}

/// Wgpu implementation of [`GpuTextureTrait`].
///
/// Wraps a [`wgpu::Texture`] with two views: a full-aspect view for rendering and
/// a separate `DepthOnly` view for shader bindings (depth-stencil textures require
/// this because wgpu rejects `Depth+Stencil` aspect on texture bindings).
///
/// Memory usage is tracked on the server and decremented on drop.
pub struct WgpuTexture {
    server: Weak<WgpuGraphicsServer>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Separate view with DepthOnly aspect for shader bindings of depth-stencil textures.
    /// For non-depth-stencil textures, this is the same as `view`.
    binding_view: wgpu::TextureView,
    kind: Cell<GpuTextureKind>,
    pixel_kind: Cell<PixelKind>,
    size_bytes: Cell<usize>,
}

impl WgpuTexture {
    /// Creates a new GPU texture from the given descriptor.
    ///
    /// Creates the wgpu texture, views (including a separate `DepthOnly` binding
    /// view for depth-stencil formats), uploads initial data if provided, and
    /// tracks memory usage on the server.
    pub fn new(
        server: &WgpuGraphicsServer,
        desc: GpuTextureDescriptor,
    ) -> Result<Self, FrameworkError> {
        let format = pixel_kind_to_wgpu_format(desc.pixel_kind);
        let dimension = texture_dimension(desc.kind);
        let (raw_w, raw_h, depth_or_layers) = texture_size(desc.kind);
        let width = raw_w.max(1);
        let height = raw_h.max(1);
        let mip_count = desc.mip_count.max(1) as u32;

        let texture = server
            .state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: if server.named_objects {
                    Some(desc.name)
                } else {
                    None
                },
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: depth_or_layers,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: None,
            dimension: Some(texture_view_dimension(desc.kind)),
            aspect: wgpu::TextureAspect::All,
            ..Default::default()
        });

        // For depth-stencil textures, create a separate view with DepthOnly aspect
        // for shader bindings. wgpu rejects Depth+Stencil aspect on texture bindings.
        let binding_view = match format {
            wgpu::TextureFormat::Depth24PlusStencil8
            | wgpu::TextureFormat::Depth32FloatStencil8 => {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: None,
                    dimension: Some(texture_view_dimension(desc.kind)),
                    aspect: wgpu::TextureAspect::DepthOnly,
                    ..Default::default()
                })
            }
            _ => view.clone(),
        };

        let size_bytes = calc_size_bytes(desc.kind, desc.pixel_kind, desc.mip_count);
        if let Some(data) = desc.data {
            Self::upload(
                &server.state.queue,
                &texture,
                desc.kind,
                desc.pixel_kind,
                data,
                desc.mip_count,
            )?;
        }
        server.memory_usage.borrow_mut().textures += size_bytes;

        Ok(Self {
            server: server.weak_ref(),
            texture,
            view,
            binding_view,
            kind: Cell::new(desc.kind),
            pixel_kind: Cell::new(desc.pixel_kind),
            size_bytes: Cell::new(size_bytes),
        })
    }

    fn upload(
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
        kind: GpuTextureKind,
        pk: PixelKind,
        data: &[u8],
        mip_count: usize,
    ) -> Result<(), FrameworkError> {
        let mip_count = mip_count.max(1);
        let fmt = pixel_kind_to_wgpu_format(pk);
        let mut offset = 0;
        for mip in 0..mip_count {
            match kind {
                GpuTextureKind::Line { length } => {
                    if let Some(l) = length.checked_shr(mip as u32) {
                        let sz = image_1d_size_bytes(pk, l);
                        if offset + sz > data.len() {
                            break;
                        }
                        write_mip_data(
                            queue,
                            texture,
                            mip as u32,
                            wgpu::Origin3d::ZERO,
                            wgpu::Extent3d {
                                width: l as u32,
                                height: 1,
                                depth_or_array_layers: 1,
                            },
                            pk,
                            &data[offset..offset + sz],
                            fmt,
                        );
                        offset += sz;
                    }
                }
                GpuTextureKind::Rectangle { width, height } => {
                    if let (Some(w), Some(h)) = (
                        width.checked_shr(mip as u32),
                        height.checked_shr(mip as u32),
                    ) {
                        let sz = image_2d_size_bytes(pk, w, h);
                        if offset + sz > data.len() {
                            break;
                        }
                        write_mip_data(
                            queue,
                            texture,
                            mip as u32,
                            wgpu::Origin3d::ZERO,
                            wgpu::Extent3d {
                                width: w as u32,
                                height: h as u32,
                                depth_or_array_layers: 1,
                            },
                            pk,
                            &data[offset..offset + sz],
                            fmt,
                        );
                        offset += sz;
                    }
                }
                GpuTextureKind::Cube { size } => {
                    if let Some(s) = size.checked_shr(mip as u32) {
                        let bpf = image_2d_size_bytes(pk, s, s);
                        for face in 0..6u32 {
                            let fo = offset + (face as usize) * bpf;
                            if fo + bpf > data.len() {
                                break;
                            }
                            write_mip_data(
                                queue,
                                texture,
                                mip as u32,
                                wgpu::Origin3d {
                                    x: 0,
                                    y: 0,
                                    z: face,
                                },
                                wgpu::Extent3d {
                                    width: s as u32,
                                    height: s as u32,
                                    depth_or_array_layers: 1,
                                },
                                pk,
                                &data[fo..fo + bpf],
                                fmt,
                            );
                        }
                        offset += 6 * bpf;
                    }
                }
                GpuTextureKind::Volume {
                    width,
                    height,
                    depth,
                } => {
                    if let (Some(w), Some(h), Some(d)) = (
                        width.checked_shr(mip as u32),
                        height.checked_shr(mip as u32),
                        depth.checked_shr(mip as u32),
                    ) {
                        let sz = image_3d_size_bytes(pk, w, h, d);
                        if offset + sz > data.len() {
                            break;
                        }
                        write_mip_data(
                            queue,
                            texture,
                            mip as u32,
                            wgpu::Origin3d::ZERO,
                            wgpu::Extent3d {
                                width: w as u32,
                                height: h as u32,
                                depth_or_array_layers: d as u32,
                            },
                            pk,
                            &data[offset..offset + sz],
                            fmt,
                        );
                        offset += sz;
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns a reference to the underlying [`wgpu::Texture`].
    pub fn wgpu_texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    /// Returns a reference to the full-aspect [`wgpu::TextureView`].
    pub fn wgpu_view(&self) -> &wgpu::TextureView {
        &self.view
    }
    /// Returns a view suitable for shader bindings.
    ///
    /// For depth-stencil textures, this is a `DepthOnly`-aspect view (wgpu rejects
    /// `Depth+Stencil` aspect on texture bindings). For all other textures, this
    /// is the same as [`wgpu_view`](Self::wgpu_view).
    pub fn wgpu_binding_view(&self) -> &wgpu::TextureView {
        &self.binding_view
    }
    /// Returns the wgpu texture format of this texture.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture.format()
    }
}

fn calc_size_bytes(kind: GpuTextureKind, pk: PixelKind, mip_count: usize) -> usize {
    let mip_count = mip_count.max(1);
    let mut total = 0;
    for mip in 0..mip_count {
        match kind {
            GpuTextureKind::Line { length } => {
                if let Some(l) = length.checked_shr(mip as u32) {
                    total += image_1d_size_bytes(pk, l);
                }
            }
            GpuTextureKind::Rectangle { width, height } => {
                if let (Some(w), Some(h)) = (
                    width.checked_shr(mip as u32),
                    height.checked_shr(mip as u32),
                ) {
                    total += image_2d_size_bytes(pk, w, h);
                }
            }
            GpuTextureKind::Cube { size } => {
                if let Some(s) = size.checked_shr(mip as u32) {
                    total += 6 * image_2d_size_bytes(pk, s, s);
                }
            }
            GpuTextureKind::Volume {
                width,
                height,
                depth,
            } => {
                if let (Some(w), Some(h), Some(d)) = (
                    width.checked_shr(mip as u32),
                    height.checked_shr(mip as u32),
                    depth.checked_shr(mip as u32),
                ) {
                    total += image_3d_size_bytes(pk, w, h, d);
                }
            }
        }
    }
    total
}

impl Drop for WgpuTexture {
    fn drop(&mut self) {
        if let Some(server) = self.server.upgrade() {
            server.memory_usage.borrow_mut().textures -= self.size_bytes.get();
        }
    }
}

impl GpuTextureTrait for WgpuTexture {
    fn set_data(
        &self,
        kind: GpuTextureKind,
        pk: PixelKind,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<usize, FrameworkError> {
        let Some(server) = self.server.upgrade() else {
            return Err(FrameworkError::GraphicsServerUnavailable);
        };
        let new_size = calc_size_bytes(kind, pk, mip_count);
        let mut mu = server.memory_usage.borrow_mut();
        mu.textures -= self.size_bytes.get();
        mu.textures += new_size;
        drop(mu);
        self.size_bytes.set(new_size);
        self.kind.set(kind);
        self.pixel_kind.set(pk);
        if let Some(data) = data {
            Self::upload(
                &server.state.queue,
                &self.texture,
                kind,
                pk,
                data,
                mip_count,
            )?;
        }
        Ok(new_size)
    }
    fn kind(&self) -> GpuTextureKind {
        self.kind.get()
    }
    fn pixel_kind(&self) -> PixelKind {
        self.pixel_kind.get()
    }
}
