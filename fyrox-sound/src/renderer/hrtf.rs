//! Head-Related Transfer Function (HRTF) module. Provides all needed types and methods for HRTF rendering.
//!
//! # Overview
//!
//! HRTF stands for [Head-Related Transfer Function](https://en.wikipedia.org/wiki/Head-related_transfer_function)
//! and can work only with spatial sounds. For each of such sound source after it was processed by HRTF you can
//! definitely tell from which locationsound came from. In other words HRTF improves perception of sound to
//! the level of real life.
//!
//! # HRIR Spheres
//!
//! This library uses Head-Related Impulse Response (HRIR) spheres to create HRTF spheres. HRTF sphere is a set of
//! points in 3D space which are connected into a mesh forming triangulated sphere. Each point contains spectrum
//! for left and right ears which will be used to modify samples from each spatial sound source to create binaural
//! sound. HRIR spheres can be found [here](https://github.com/mrDIMAS/hrir_sphere_builder/tree/master/hrtf_base/IRCAM)
//!
//! # Usage
//!
//! To use HRTF you need to change default renderer to HRTF renderer like so:
//!
//! ```no_run
//! use fyrox_sound::context::{self, SoundContext};
//! use fyrox_sound::renderer::hrtf::{HrirSphereResource, HrirSphereResourceExt, HrtfRenderer};
//! use fyrox_sound::renderer::Renderer;
//! use std::path::{Path, PathBuf};
//! use hrtf::HrirSphere;
//!
//! fn use_hrtf(context: &mut SoundContext) {
//!     // IRC_1002_C.bin is HRIR sphere in binary format, can be any valid HRIR sphere
//!     // from base mentioned above.
//!     let hrir_path = PathBuf::from("examples/data/IRC_1002_C.bin");
//!     let hrir_sphere = HrirSphere::from_file(&hrir_path, context::SAMPLE_RATE).unwrap();
//!
//!     context.state().set_renderer(Renderer::HrtfRenderer(HrtfRenderer::new(HrirSphereResource::from_hrir_sphere(hrir_sphere, hrir_path.into()))));
//! }
//! ```
//!
//! # Performance
//!
//! HRTF is `heavy`. Usually it 4-5 slower than default renderer, this is essential because HRTF requires some heavy
//! math (fast Fourier transform, convolution, etc.). On Ryzen 1700 it takes 400-450 μs (0.4 - 0.45 ms) per source.
//! In most cases this is ok, engine works in separate thread and it has around 100 ms to prepare new portion of
//! samples for output device.
//!
//! # Known problems
//!
//! This renderer still suffers from small audible clicks in very fast moving sounds, clicks sounds more like
//! "buzzing" - it is due the fact that hrtf is different from frame to frame which gives "bumps" in amplitude
//! of signal because of phase shift each impulse response have. This can be fixed by short cross fade between
//! small amount of samples from previous frame with same amount of frames of current as proposed in
//! [here](http://csoundjournal.com/issue9/newHRTFOpcodes.html)
//!
//! Clicks can be reproduced by using clean sine wave of 440 Hz on some source moving around listener.

use crate::{
    context::{self, DistanceModel, SoundContext},
    listener::Listener,
    renderer::render_source_2d_only,
    source::SoundSource,
};
use fyrox_core::{
    log::Log,
    reflect::prelude::*,
    uuid::{uuid, Uuid},
    visitor::{Visit, VisitResult, Visitor},
    TypeUuidProvider,
};
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    state::LoadError,
    Resource, ResourceData,
};
use hrtf::HrirSphere;
use std::error::Error;
use std::path::Path;
use std::{any::Any, fmt::Debug, fmt::Formatter, path::PathBuf, sync::Arc};

/// See module docs.
#[derive(Clone, Debug, Default, Reflect)]
pub struct HrtfRenderer {
    hrir_resource: Option<HrirSphereResource>,
    #[reflect(hidden)]
    processor: Option<hrtf::HrtfProcessor>,
}

impl Visit for HrtfRenderer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        Log::verify(self.hrir_resource.visit("HrirResource", &mut region));

        Ok(())
    }
}

impl HrtfRenderer {
    /// Creates new HRTF renderer using specified HRTF sphere. See module docs for more info.
    pub fn new(hrir_sphere_resource: HrirSphereResource) -> Self {
        Self {
            processor: Some(hrtf::HrtfProcessor::new(
                {
                    let sphere = hrir_sphere_resource.data_ref().hrir_sphere.clone().unwrap();
                    sphere
                },
                SoundContext::HRTF_INTERPOLATION_STEPS,
                SoundContext::HRTF_BLOCK_LEN,
            )),
            hrir_resource: Some(hrir_sphere_resource),
        }
    }

    /// Sets a desired HRIR sphere resource. Current state of the renderer will be reset and then it will be recreated
    /// on the next render call only if the resource is fully loaded.
    pub fn set_hrir_sphere_resource(&mut self, resource: Option<HrirSphereResource>) {
        self.hrir_resource = resource;
        self.processor = None;
    }

    /// Returns current HRIR sphere resource (if any).
    pub fn hrir_sphere_resource(&self) -> Option<HrirSphereResource> {
        self.hrir_resource.clone()
    }

    pub(crate) fn render_source(
        &mut self,
        source: &mut SoundSource,
        listener: &Listener,
        distance_model: DistanceModel,
        out_buf: &mut [(f32, f32)],
    ) {
        // Re-create HRTF processor on the fly only when a respective HRIR sphere resource is fully loaded.
        // This is a poor-man's async support for crippled OSes such as WebAssembly.
        if self.processor.is_none() {
            if let Some(resource) = self.hrir_resource.as_ref() {
                let mut header = resource.state();
                if let Some(hrir) = header.data() {
                    self.processor = Some(hrtf::HrtfProcessor::new(
                        hrir.hrir_sphere.clone().unwrap(),
                        SoundContext::HRTF_INTERPOLATION_STEPS,
                        SoundContext::HRTF_BLOCK_LEN,
                    ));
                }
            }
        }

        // Render as 2D first with k = (1.0 - spatial_blend).
        render_source_2d_only(source, out_buf);

        // Then add HRTF part with k = spatial_blend
        let new_distance_gain = source.gain()
            * source.spatial_blend()
            * source.calculate_distance_gain(listener, distance_model);
        let new_sampling_vector = source.calculate_sampling_vector(listener);

        if let Some(processor) = self.processor.as_mut() {
            processor.process_samples(hrtf::HrtfContext {
                source: &source.frame_samples,
                output: out_buf,
                new_sample_vector: hrtf::Vec3::new(
                    new_sampling_vector.x,
                    new_sampling_vector.y,
                    new_sampling_vector.z,
                ),
                prev_sample_vector: hrtf::Vec3::new(
                    source.prev_sampling_vector.x,
                    source.prev_sampling_vector.y,
                    source.prev_sampling_vector.z,
                ),
                prev_left_samples: &mut source.prev_left_samples,
                prev_right_samples: &mut source.prev_right_samples,
                prev_distance_gain: source.prev_distance_gain.unwrap_or(new_distance_gain),
                new_distance_gain,
            });
        }

        source.prev_sampling_vector = new_sampling_vector;
        source.prev_distance_gain = Some(new_distance_gain);
    }
}

/// Wrapper for [`HrirSphere`] to be able to use it in the resource manager, that will handle async resource
/// loading automatically.
#[derive(Reflect, Default, Visit)]
pub struct HrirSphereResourceData {
    #[reflect(hidden)]
    #[visit(skip)]
    hrir_sphere: Option<HrirSphere>,
}

impl Debug for HrirSphereResourceData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HrirSphereResourceData").finish()
    }
}

impl TypeUuidProvider for HrirSphereResourceData {
    fn type_uuid() -> Uuid {
        uuid!("c92a0fa3-0ed3-49a9-be44-8f06271c6be2")
    }
}

impl ResourceData for HrirSphereResourceData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
        Err("Saving is not supported!".to_string().into())
    }

    fn can_be_saved(&self) -> bool {
        false
    }
}

/// Resource loader for [`HrirSphereResource`].
pub struct HrirSphereLoader;

impl ResourceLoader for HrirSphereLoader {
    fn extensions(&self) -> &[&str] {
        &["hrir"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <HrirSphereResourceData as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let reader = io.file_reader(&path).await.map_err(LoadError::new)?;
            let hrir_sphere =
                HrirSphere::new(reader, context::SAMPLE_RATE).map_err(LoadError::new)?;
            Ok(LoaderPayload::new(HrirSphereResourceData {
                hrir_sphere: Some(hrir_sphere),
            }))
        })
    }
}

/// An alias to `Resource<HrirSphereResourceData>`.
pub type HrirSphereResource = Resource<HrirSphereResourceData>;

/// A set of extension methods for [`HrirSphereResource`]
pub trait HrirSphereResourceExt {
    /// Creates a new HRIR sphere resource directly from pre-loaded HRIR sphere. It could be used if you
    /// do not use a resource manager, but want to load HRIR spheres manually.
    fn from_hrir_sphere(hrir_sphere: HrirSphere, kind: ResourceKind) -> Self;
}

impl HrirSphereResourceExt for HrirSphereResource {
    fn from_hrir_sphere(hrir_sphere: HrirSphere, kind: ResourceKind) -> Self {
        Resource::new_ok(
            kind,
            HrirSphereResourceData {
                hrir_sphere: Some(hrir_sphere),
            },
        )
    }
}
