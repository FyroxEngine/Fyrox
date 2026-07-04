use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use ndarray::Array3;
use std::path::Path;

use crate::ViewerState;

pub fn load(path: &Path, resolution: u32) -> Result<Array3<f32>, String> {
    let bytes    = std::fs::read(path).map_err(|e| e.to_string())?;
    let n        = (resolution as usize).pow(3);
    let expected = n * 4;

    if bytes.len() < expected {
        return Err(format!(
            "File is {} bytes but {}³ field needs {} bytes",
            bytes.len(),
            resolution,
            expected
        ));
    }

    let values: Vec<f32> = bytes[..expected]
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    let r = resolution as usize;
    Array3::from_shape_vec((r, r, r), values).map_err(|e| e.to_string())
}

/// Regenerate the egui texture from the current slice when tex_dirty is set.
pub fn refresh_texture(mut state: ResMut<ViewerState>, mut images: ResMut<Assets<Image>>) {
    if !state.tex_dirty {
        return;
    }
    state.tex_dirty = false;

    let res = state.field_res as usize;
    let idx = state.slice as usize;
    let axis = state.axis;

    let (Some(field), Some(handle)) = (&state.field, &state.tex) else {
        return;
    };

    // Extract 2D slice into a flat vec (row-major: row = second axis, col = third axis)
    let pixels: Vec<f32> = match axis {
        crate::Axis::X => {
            let mut v = vec![0.0f32; res * res];
            for row in 0..res {
                for col in 0..res {
                    v[row * res + col] = field[[idx, col, row]];
                }
            }
            v
        }
        crate::Axis::Y => {
            let mut v = vec![0.0f32; res * res];
            for row in 0..res {
                for col in 0..res {
                    v[row * res + col] = field[[col, idx, row]];
                }
            }
            v
        }
        crate::Axis::Z => {
            let mut v = vec![0.0f32; res * res];
            for row in 0..res {
                for col in 0..res {
                    v[row * res + col] = field[[col, row, idx]];
                }
            }
            v
        }
    };

    // Normalize to [0, 1]
    let min_v = pixels.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_v = pixels.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = (max_v - min_v).max(1e-7);

    // Inferno-inspired colormap: dark blue/purple → orange → yellow
    let rgba: Vec<u8> = pixels
        .iter()
        .flat_map(|&v| {
            let t = ((v - min_v) / range).clamp(0.0, 1.0);
            let r = (t * 1.8 * 255.0).clamp(0.0, 255.0) as u8;
            let g = ((t - 0.25).max(0.0) * 1.5 * 255.0).clamp(0.0, 255.0) as u8;
            let b = ((1.0 - t * 1.5).max(0.0) * 255.0).clamp(0.0, 255.0) as u8;
            [r, g, b, 255u8]
        })
        .collect();

    let new_image = Image::new(
        Extent3d {
            width:                 res as u32,
            height:                res as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    if let Some(img) = images.get_mut(handle) {
        *img = new_image;
    }
}
