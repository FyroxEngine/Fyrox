use std::path::*;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};

pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Texture {
    pub(in crate) path: PathBuf,
    pub(in crate) width: u32,
    pub(in crate) height: u32,
    pub(in crate) gpu_tex: u32,
    pub(in crate) need_upload: bool,
    pub(in crate) pixels: Vec<Rgba8>,
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            width: 0,
            height: 0,
            gpu_tex: 0,
            need_upload: false,
            pixels: Vec::new()
        }
    }
}

impl Visit for Texture {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

impl Texture {
    pub(in crate) fn load(path: &Path) -> Result<Texture, image::ImageError> {
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
            path: PathBuf::from(path),
            pixels,
            need_upload: true,
            width,
            height,
            gpu_tex: 0,
        })
    }
}