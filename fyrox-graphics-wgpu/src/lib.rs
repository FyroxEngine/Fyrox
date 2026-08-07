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

//! Wgpu-based graphics server implementation for the Fyrox game engine.
//!
//! This crate provides a concrete [`GraphicsServer`](fyrox_graphics::server::GraphicsServer)
//! implementation backed by the [wgpu](https://wgpu.rs/) graphics API. It is the primary
//! rendering backend for Fyrox and supports desktop (Vulkan/Metal/DX12) and web (WebGL2/WebGPU)
//! targets.
//!
//! # Architecture
//!
//! The entry point is [`server::WgpuGraphicsServer`], which creates and owns the wgpu
//! device, queue, and surface. All GPU resources (textures, buffers, shaders, etc.) are
//! created through the server and implement the corresponding trait from [`fyrox_graphics`].
//!
//! ```text
//! WgpuGraphicsServer
//!   ├── WgpuTexture        (texture.rs)       — 1D/2D/3D/Cube textures
//!   ├── WgpuBuffer         (buffer.rs)        — uniform/vertex/index buffers
//!   ├── WgpuSampler        (sampler.rs)       — texture sampling state
//!   ├── WgpuGeometryBuffer (geometry_buffer.rs) — mesh vertex + index data
//!   ├── WgpuFrameBuffer    (framebuffer.rs)   — render targets + draw calls
//!   ├── WgpuProgram        (program.rs)       — compiled shader programs
//!   ├── WgpuQuery          (query.rs)         — GPU queries (stub)
//!   └── WgpuAsyncReadBuffer(read_buffer.rs)   — async pixel readback
//! ```
//!
//! # Binding Layout
//!
//! All shader resources use a fixed binding scheme within bind group 0:
//!
//! | Resource kind       | Binding slot           |
//! |---------------------|------------------------|
//! | Texture view        | `N`                    |
//! | Sampler             | `N + 100`              |
//! | Uniform buffer      | `N + 200`              |
//!
//! Where `N` is the resource's declared binding index from [`ShaderResourceDefinition`].
//!
//! # Pipeline Caching
//!
//! Render pipelines are immutable and expensive to create. The backend caches them
//! by a hash of the full render state (program, formats, blend, depth, stencil, cull)
//! in a per-server [`HashMap`](std::collections::HashMap).
//!
//! # Shared Shader Library
//!
//! Every compiled shader includes `shaders/shared.wgsl`, which provides PBR lighting,
//! shadow mapping, color space conversion, and other common utilities.

#![warn(missing_docs)]

/// Generic GPU buffer implementation (uniform, vertex, index, pixel read/write).
pub mod buffer;
/// Texture format helpers and binding offset constants.
pub mod format_helpers;
/// Render target (framebuffer) implementation with draw call logic and pipeline caching.
pub mod framebuffer;
/// Geometry buffer implementation (vertex + index buffers for mesh rendering).
pub mod geometry_buffer;
/// Shader compilation and program management.
pub mod program;
/// GPU query stub implementation.
pub mod query;
/// Async pixel readback buffer implementation.
pub mod read_buffer;
/// Texture sampler implementation.
pub mod sampler;
/// The main graphics server — entry point for all GPU resource creation.
pub mod server;
/// Texture creation, upload, and format mapping.
pub mod texture;
