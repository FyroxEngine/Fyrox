use std::path::*;

pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Texture {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) gpu_tex: u32,
    pub(crate) need_upload: bool,
    pub(crate) pixels: Vec<Rgba8>,
}

impl Texture {
    pub fn load(path: &Path) -> Result<Texture, image::ImageError> {
        let image = match image::open(path)? {
            image::DynamicImage::ImageRgba8(img) => img,
            other => other.to_rgba()
        };
        let width = image.width();
        let height = image.height();
        let raw_pixels = image.into_raw();
        let mut pixels: Vec<Rgba8> = Vec::with_capacity((width * height) as usize);

        for y in 0..height {
            for x in 0..width {
                let index = 4 * (y * width + x) as usize;

                let r = raw_pixels[index];
                let g = raw_pixels[index + 1];
                let b = raw_pixels[index + 2];
                let a = raw_pixels[index + 3];

                pixels.push(Rgba8 { r, g, b, a });
            }
        }

        Ok(Texture {
            pixels,
            need_upload: true,
            width,
            height,
            gpu_tex: 0,
        })
    }
}