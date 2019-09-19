#[allow(clippy::all)]
mod gl;
pub mod render;
pub mod surface;
pub mod gpu_program;
pub mod error;

mod geometry_buffer;
mod ui_renderer;
mod particle_system_renderer;
mod gbuffer;
mod deferred_light_renderer;
mod shadow_map_renderer;

#[repr(C)]
pub struct TriangleDefinition {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}