use std::{
    sync::{
        Arc,
        Mutex,
    },
    time,
};
use crate::{
    error::SoundError,
    source::Source,
    device::run_device,
    listener::Listener,
    source::Status,
    hrtf::Hrtf,
};
use rg3d_core::{
    pool::{Pool, Handle},
    visitor::{Visit, VisitResult, Visitor},
};
use rustfft::{
    num_complex::Complex,
    num_traits::Zero,
    FFTplanner,
};
use crate::hrtf::HrtfPoint;
use crate::source::SourceKind;

pub struct Context {
    sources: Pool<Source>,
    listener: Listener,
    master_gain: f32,
    hrtf: Option<Hrtf>,
    left_in_buffer: Vec<Complex<f32>>,
    right_in_buffer: Vec<Complex<f32>>,
    left_out_buffer: Vec<Complex<f32>>,
    right_out_buffer: Vec<Complex<f32>>,
    fft: FFTplanner<f32>,
    ifft: FFTplanner<f32>,
    hrtf_buf: HrtfPoint,
}

/// Overlap-add convolution in frequency domain.
///
/// https://en.wikipedia.org/wiki/Overlap%E2%80%93add_method
///
fn convolve(in_buffer: &mut [Complex<f32>],
            out_buffer: &mut [Complex<f32>],
            hrir_spectrum: &[Complex<f32>],
            prev_samples: &mut Vec<Complex<f32>>,
            fft: &mut FFTplanner<f32>,
            ifft: &mut FFTplanner<f32>,
) {
    assert_eq!(hrir_spectrum.len(), in_buffer.len());

    fft.plan_fft(in_buffer.len())
        .process(in_buffer, out_buffer);
    // Multiply HRIR and input signal in frequency domain
    for (s, h) in out_buffer.iter_mut().zip(hrir_spectrum.iter()) {
        *s *= *h;
    }
    ifft.plan_fft(in_buffer.len())
        .process(out_buffer, in_buffer);

    // Add part from previous frame.
    for (l, c) in prev_samples.iter().zip(in_buffer.iter_mut()) {
        *c += *l;
    }

    // Remember samples from current frame as remainder for next frame.
    prev_samples.clear();
    for c in in_buffer.iter().skip(Context::SAMPLE_PER_CHANNEL) {
        prev_samples.push(*c);
    }
}

fn pad_zeros(buf: &mut [Complex<f32>]) {
    for v in buf.iter_mut() {
        *v = Complex::zero();
    }
}

impl Context {
    pub const SAMPLE_PER_CHANNEL: usize = 3584;

    pub fn new() -> Result<Arc<Mutex<Self>>, SoundError> {
        let context = Self {
            sources: Pool::new(),
            listener: Listener::new(),
            master_gain: 1.0,
            hrtf: None,
            left_in_buffer: Default::default(),
            right_in_buffer: Default::default(),
            left_out_buffer: Default::default(),
            right_out_buffer: Default::default(),
            fft: FFTplanner::new(false),
            ifft: FFTplanner::new(true),
            hrtf_buf: HrtfPoint {
                pos: Default::default(),
                left_hrtf: vec![],
                right_hrtf: vec![],
            },
        };

        let context = Arc::new(Mutex::new(context));

        // Run device with a mixer callback. Mixer callback will mix samples
        // from source with a fixed rate.
        run_device(4 * Self::SAMPLE_PER_CHANNEL as u32, {
            let context = context.clone();
            Box::new(move |buf| {
                if let Ok(mut context) = context.lock() {
                    context.render(buf);
                }
            })
        })?;

        Ok(context)
    }

    fn render(&mut self, buf: &mut [(f32, f32)]) {
        let current_time = time::Instant::now();
        for source in self.sources.iter_mut() {
            let use_hrtf = match source.get_kind() {
                SourceKind::Flat => false,
                SourceKind::Spatial(_) => self.hrtf.is_some(),
            };

            if use_hrtf {
                let hrtf = self.hrtf.as_ref().unwrap();

                let pad_length = Self::SAMPLE_PER_CHANNEL + hrtf.length - 1;

                hrtf.sample_bilinear(&mut self.hrtf_buf, source.hrtf_sampling_vector);

                let point = &self.hrtf_buf;

                // Prepare buffers
                if self.left_in_buffer.len() != pad_length {
                    self.left_in_buffer = vec![Complex::zero(); pad_length];
                }
                if self.left_out_buffer.len() != pad_length {
                    self.left_out_buffer = vec![Complex::zero(); pad_length];
                }
                if self.right_in_buffer.len() != pad_length {
                    self.right_in_buffer = vec![Complex::zero(); pad_length];
                }
                if self.right_out_buffer.len() != pad_length {
                    self.right_out_buffer = vec![Complex::zero(); pad_length];
                }
                if source.last_frame_left_samples.len() != hrtf.length - 1 {
                    source.last_frame_left_samples = vec![Complex::zero(); pad_length];
                }
                if source.last_frame_right_samples.len() != hrtf.length - 1 {
                    source.last_frame_right_samples = vec![Complex::zero(); pad_length];
                }

                // Gather samples for processing.
                source.sample_for_hrtf(&mut self.left_in_buffer[0..Context::SAMPLE_PER_CHANNEL],
                                       &mut self.right_in_buffer[0..Context::SAMPLE_PER_CHANNEL]);

                pad_zeros(&mut self.left_in_buffer[Context::SAMPLE_PER_CHANNEL..pad_length]);
                pad_zeros(&mut self.right_in_buffer[Context::SAMPLE_PER_CHANNEL..pad_length]);

                // Do magic.
                assert_eq!(point.left_hrtf.len(), point.right_hrtf.len());

                convolve(&mut self.left_in_buffer,
                         &mut self.left_out_buffer,
                         &point.left_hrtf,
                         &mut source.last_frame_left_samples,
                         &mut self.fft,
                         &mut self.ifft);

                convolve(&mut self.right_in_buffer,
                         &mut self.right_out_buffer,
                         &point.right_hrtf,
                         &mut source.last_frame_right_samples,
                         &mut self.fft,
                         &mut self.ifft);

                // Mix samples into output buffer with rescaling.
                let k = source.distance_gain / (pad_length as f32);

                // Take only N samples as output data, rest will be used for next frame.
                for (i, (out_left, out_right)) in buf.iter_mut().enumerate() {
                     let left = self.left_in_buffer.get(i).unwrap();
                    *out_left += left.re * k;

                    let right = self.right_in_buffer.get(i).unwrap();
                    *out_right += right.re * k;
                }
            } else {
                source.sample_into(buf);
            }
        }

        // Apply master gain to be able to control total sound volume.
        for (left, right) in buf {
            *left *= self.master_gain;
            *right *= self.master_gain;
        }

        println!("sound render time = {:?}", time::Instant::now() - current_time);
    }

    pub fn set_hrtf(&mut self, hrtf: Hrtf) {
        self.hrtf = Some(hrtf);
    }

    pub fn set_master_gain(&mut self, gain: f32) {
        self.master_gain = gain;
    }

    pub fn get_master_gain(&self) -> f32 {
        self.master_gain
    }

    pub fn add_source(&mut self, source: Source) -> Handle<Source> {
        self.sources.spawn(source)
    }

    pub fn get_sources(&self) -> &Pool<Source> {
        &self.sources
    }

    pub fn get_sources_mut(&mut self) -> &mut Pool<Source> {
        &mut self.sources
    }

    pub fn get_source(&self, handle: Handle<Source>) -> &Source {
        self.sources.borrow(handle)
    }

    pub fn get_source_mut(&mut self, handle: Handle<Source>) -> &mut Source {
        self.sources.borrow_mut(handle)
    }

    pub fn get_listener(&self) -> &Listener {
        &self.listener
    }

    pub fn get_listener_mut(&mut self) -> &mut Listener {
        &mut self.listener
    }

    pub fn update(&mut self) -> Result<(), SoundError> {
        self.listener.update();
        for source in self.sources.iter_mut() {
            source.update(&self.listener)?;
        }
        for i in 0..self.sources.get_capacity() {
            if let Some(source) = self.sources.at(i) {
                if source.is_play_once() && source.get_status() == Status::Stopped {
                    self.sources.free(self.sources.handle_from_index(i));
                }
            }
        }
        Ok(())
    }
}

impl Visit for Context {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        if visitor.is_reading() {
            self.sources.clear();
        }

        self.master_gain.visit("MasterGain", visitor)?;
        self.listener.visit("Listener", visitor)?;
        self.sources.visit("Sources", visitor)?;

        visitor.leave_region()
    }
}