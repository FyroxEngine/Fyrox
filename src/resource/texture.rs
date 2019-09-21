use std::path::*;
use rg3d_core::visitor::{Visit, VisitResult, Visitor};
use crate::renderer::gpu_texture::GpuTexture;

pub struct Texture {
    pub(in crate) path: PathBuf,
    pub(in crate) width: u32,
    pub(in crate) height: u32,
    pub(in crate) gpu_tex: Option<GpuTexture>,
    pub(in crate) bytes: Vec<u8>,
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            width: 0,
            height: 0,
            gpu_tex: None,
            bytes: Vec::new()
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

        Ok(Texture {
            path: PathBuf::from(path),
            width: image.width(),
            height: image.height(),
            bytes: image.into_raw(),
            gpu_tex: None,
        })
    }

    pub(in crate) fn bind(&self, sampler_index: usize) {
        if let Some(texture) = &self.gpu_tex {
            texture.bind(sampler_index)
        }
    }
}