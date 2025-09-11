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

#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::{Vector2, Vector3},
        array_as_u8_slice,
    },
    graphics::{
        error::FrameworkError,
        gpu_texture::{CubeMapFace, GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
        server::GraphicsServer,
    },
};
use bytemuck::{Pod, Zeroable};
use half::f16;
use std::{fs::File, io::Write, path::Path};

pub struct CubeMapFaceDescriptor {
    pub face: CubeMapFace,
    pub look: Vector3<f32>,
    pub up: Vector3<f32>,
}

impl CubeMapFaceDescriptor {
    pub fn cube_faces() -> [Self; 6] {
        [
            CubeMapFaceDescriptor {
                face: CubeMapFace::PositiveX,
                look: Vector3::new(1.0, 0.0, 0.0),
                up: Vector3::new(0.0, -1.0, 0.0),
            },
            CubeMapFaceDescriptor {
                face: CubeMapFace::NegativeX,
                look: Vector3::new(-1.0, 0.0, 0.0),
                up: Vector3::new(0.0, -1.0, 0.0),
            },
            CubeMapFaceDescriptor {
                face: CubeMapFace::PositiveY,
                look: Vector3::new(0.0, 1.0, 0.0),
                up: Vector3::new(0.0, 0.0, 1.0),
            },
            CubeMapFaceDescriptor {
                face: CubeMapFace::NegativeY,
                look: Vector3::new(0.0, -1.0, 0.0),
                up: Vector3::new(0.0, 0.0, -1.0),
            },
            CubeMapFaceDescriptor {
                face: CubeMapFace::PositiveZ,
                look: Vector3::new(0.0, 0.0, 1.0),
                up: Vector3::new(0.0, -1.0, 0.0),
            },
            CubeMapFaceDescriptor {
                face: CubeMapFace::NegativeZ,
                look: Vector3::new(0.0, 0.0, -1.0),
                up: Vector3::new(0.0, -1.0, 0.0),
            },
        ]
    }
}

fn radical_inverse_vd_c(mut bits: u32) -> f32 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x55555555) << 1) | ((bits & 0xAAAAAAAA) >> 1);
    bits = ((bits & 0x33333333) << 2) | ((bits & 0xCCCCCCCC) >> 2);
    bits = ((bits & 0x0F0F0F0F) << 4) | ((bits & 0xF0F0F0F0) >> 4);
    bits = ((bits & 0x00FF00FF) << 8) | ((bits & 0xFF00FF00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

fn hammersley(i: usize, n: usize) -> Vector2<f32> {
    Vector2::new(i as f32 / n as f32, radical_inverse_vd_c(i as u32))
}

fn importance_sample_ggx(x_i: Vector2<f32>, roughness: f32, n: Vector3<f32>) -> Vector3<f32> {
    let a = roughness * roughness;

    let phi = 2.0 * std::f32::consts::PI * x_i.x;
    let cos_theta = ((1.0 - x_i.y) / (1.0 + (a * a - 1.0) * x_i.y)).sqrt();
    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

    // from spherical coordinates to cartesian coordinates
    let h = Vector3::new(phi.cos() * sin_theta, phi.sin() * sin_theta, cos_theta);

    // from tangent-space vector to world-space sample vector
    let up = if n.z.abs() < 0.999 {
        Vector3::new(0.0, 0.0, 1.0)
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    };
    let tangent = up.cross(&n).normalize();
    let bitangent = n.cross(&tangent);

    (tangent * h.x + bitangent * h.y + n * h.z).normalize()
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let a = roughness;
    let k = (a * a) / 2.0;

    let nom = n_dot_v;
    let denom = n_dot_v * (1.0 - k) + k;

    nom / denom
}

fn geometry_smith(roughness: f32, n_dot_v: f32, n_dot_l: f32) -> f32 {
    let ggx2 = geometry_schlick_ggx(n_dot_v, roughness);
    let ggx1 = geometry_schlick_ggx(n_dot_l, roughness);

    ggx1 * ggx2
}

fn integrate_brdf(n_dot_v: f32, roughness: f32, samples: usize) -> Vector2<f32> {
    let v = Vector3::new((1.0 - n_dot_v * n_dot_v).sqrt(), 0.0, n_dot_v);

    let mut a = 0.0;
    let mut b = 0.0;

    let n = Vector3::new(0.0, 0.0, 1.0);

    for i in 0..samples {
        let x_i = hammersley(i, samples);
        let h = importance_sample_ggx(x_i, roughness, n);
        let l = (2.0 * v.dot(&h) * h - v).normalize();

        let n_dot_l = l.z.max(0.0);
        let n_dot_h = h.z.max(0.0);
        let v_dot_h = v.dot(&h).max(0.0);
        let n_dot_v = n.dot(&v).max(0.0);

        if n_dot_l > 0.0 {
            let g = geometry_smith(roughness, n_dot_v, n_dot_l);

            let g_vis = (g * v_dot_h) / (n_dot_h * n_dot_v);
            let fc = (1.0 - v_dot_h).powf(5.0);

            a += (1.0 - fc) * g_vis;
            b += fc * g_vis;
        }
    }

    Vector2::new(a / samples as f32, b / samples as f32)
}

#[derive(Default, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Pixel {
    pub x: f16,
    pub y: f16,
}

pub fn make_brdf_lut_image(size: usize, sample_count: usize) -> Vec<Pixel> {
    let mut pixels = vec![Pixel::default(); size * size];

    for y in 0..size {
        for x in 0..size {
            let n_dot_v = (y as f32 + 0.5) * (1.0 / size as f32);
            let roughness = (x as f32 + 0.5) * (1.0 / size as f32);
            let pair = integrate_brdf(n_dot_v, roughness, sample_count);
            let pixel = &mut pixels[y * size + x];
            pixel.x = f16::from_f32(pair.x);
            pixel.y = f16::from_f32(pair.y);
        }
    }

    pixels
}

pub fn generate_brdf_lut_texture(
    server: &dyn GraphicsServer,
    size: usize,
    sample_count: usize,
) -> Result<GpuTexture, FrameworkError> {
    let pixels = make_brdf_lut_image(size, sample_count);
    make_brdf_lut(server, size, array_as_u8_slice(&pixels))
}

pub fn make_brdf_lut(
    server: &dyn GraphicsServer,
    size: usize,
    pixels: &[u8],
) -> Result<GpuTexture, FrameworkError> {
    server.create_texture(GpuTextureDescriptor {
        name: "BrdfLut",
        kind: GpuTextureKind::Rectangle {
            width: size,
            height: size,
        },
        pixel_kind: PixelKind::RG16F,
        mip_count: 1,
        data: Some(pixels),
        ..Default::default()
    })
}

pub fn write_brdf_lut(path: &Path, size: usize, sample_count: usize) -> std::io::Result<()> {
    let pixels = make_brdf_lut_image(size, sample_count);
    let mut file = File::create(path)?;
    file.write_all(array_as_u8_slice(&pixels))?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::renderer::utils::write_brdf_lut;
    use std::path::Path;

    // Use this test to write BRDF use by the lighting module.
    #[test]
    fn test_write_brdf_lut() {
        write_brdf_lut(
            Path::new("src/renderer/brdf_256x256_256samples.bin"),
            256,
            256,
        )
        .unwrap();
    }
}
