/// Head-related transfer function (HRTF) loader, interpolator and renderer.

use rustfft::{
    num_complex::Complex,
    num_traits::Zero,
    FFTplanner,
};
use crate::{
    context::Context,
    device,
    source::{
        Source,
        Status
    }
};
use std::{
    fs::File,
    path::Path,
    io::{BufReader, Read, Error},
};
use rg3d_core::math::{
    get_barycentric_coords,
    vec3::Vec3,
    ray::Ray,
};
use byteorder::{
    ReadBytesExt,
    LittleEndian,
};

struct HrtfPoint {
    pos: Vec3,
    left_hrtf: Vec<Complex<f32>>,
    right_hrtf: Vec<Complex<f32>>,
}

struct Face {
    a: usize,
    b: usize,
    c: usize,
}

pub struct HrtfSphere {
    length: usize,
    points: Vec<HrtfPoint>,
    faces: Vec<Face>,
}

#[derive(Debug)]
pub enum HrtfError {
    /// Io error has occurred (file does not exists, etc.)
    IoError(std::io::Error),

    /// Hrtf has sample rate that differs from device sample rate.
    /// (current_sample_rate, device_sample_rate)
    /// You should resample hrtf first.
    InvalidSampleRate(u32, u32),

    /// It is not valid hrtf base file.
    InvalidFileFormat,
}

impl From<std::io::Error> for HrtfError {
    fn from(io_err: Error) -> Self {
        HrtfError::IoError(io_err)
    }
}

fn make_hrtf(mut hrir: Vec<Complex<f32>>, pad_length: usize, planner: &mut FFTplanner<f32>) -> Vec<Complex<f32>> {
    for _ in hrir.len()..pad_length {
        // Pad with zeros to length of context's output buffer.
        hrir.push(Complex::zero());
    }
    let mut hrtf = vec![Complex::zero(); pad_length];
    planner.plan_fft(pad_length).process(hrir.as_mut(), hrtf.as_mut());
    hrtf
}

fn read_hrir(reader: &mut dyn Read, len: usize) -> Result<Vec<Complex<f32>>, HrtfError> {
    let mut hrir = Vec::with_capacity(len);
    for _ in 0..len {
        hrir.push(Complex::new(reader.read_f32::<LittleEndian>()?, 0.0));
    }
    Ok(hrir)
}

fn read_faces(reader: &mut dyn Read, index_count: usize) -> Result<Vec<Face>, HrtfError> {
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        indices.push(reader.read_u32::<LittleEndian>()?);
    }
    let faces = indices.chunks(3)
        .map(|f| Face { a: f[0] as usize, b: f[1] as usize, c: f[2] as usize })
        .collect();
    Ok(faces)
}

impl HrtfSphere {
    /// Loads HRIR sphere and creates HRTF sphere from it.
    pub fn new(path: &Path) -> Result<HrtfSphere, HrtfError> {
        let mut reader = BufReader::new(File::open(path)?);

        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;
        if magic[0] != b'H' && magic[1] != b'R' && magic[2] != b'I' && magic[3] != b'R' {
            return Err(HrtfError::InvalidFileFormat);
        }

        let sample_rate = reader.read_u32::<LittleEndian>()?;
        if sample_rate != device::SAMPLE_RATE {
            return Err(HrtfError::InvalidSampleRate(sample_rate, device::SAMPLE_RATE));
        }
        let length = reader.read_u32::<LittleEndian>()? as usize;
        let vertex_count = reader.read_u32::<LittleEndian>()? as usize;
        let index_count = reader.read_u32::<LittleEndian>()? as usize;

        let faces = read_faces(&mut reader, index_count)?;

        let mut planner = FFTplanner::new(false);
        let pad_length = Context::SAMPLE_PER_CHANNEL + length - 1;

        let mut points = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let z = reader.read_f32::<LittleEndian>()?;

            let left_hrtf = make_hrtf(read_hrir(&mut reader, length)?, pad_length, &mut planner);
            let right_hrtf = make_hrtf(read_hrir(&mut reader, length)?, pad_length, &mut planner);

            points.push(HrtfPoint {
                pos: Vec3::new(x, y, z),
                left_hrtf,
                right_hrtf,
            });
        }

        Ok(Self {
            points,
            length,
            faces,
        })
    }

    /// Sampling with bilinear interpolation
    /// http://www02.smt.ufrj.br/~diniz/conf/confi117.pdf
    pub fn sample_bilinear(&self, left_hrtf: &mut Vec<Complex<f32>>, right_hrtf: &mut Vec<Complex<f32>>, dir: Vec3) {
        if let Some(ray) = Ray::from_two_points(&Vec3::ZERO, &dir.scale(10.0)) {
            for face in self.faces.iter() {
                let a = self.points.get(face.a).unwrap();
                let b = self.points.get(face.b).unwrap();
                let c = self.points.get(face.c).unwrap();

                if let Some(p) = ray.triangle_intersection(&[a.pos, b.pos, c.pos]) {
                    let (ka, kb, kc) = get_barycentric_coords(&p, &a.pos, &b.pos, &c.pos);

                    let len = a.left_hrtf.len();

                    left_hrtf.clear();
                    for i in 0..len {
                        left_hrtf.push(
                            a.left_hrtf[i] * ka +
                                b.left_hrtf[i] * kb +
                                c.left_hrtf[i] * kc);
                    }

                    right_hrtf.clear();
                    for i in 0..len {
                        right_hrtf.push(
                            a.right_hrtf[i] * ka +
                                b.right_hrtf[i] * kb +
                                c.right_hrtf[i] * kc);
                    }
                }
            }
        } else {
            // In case if we have degenerated dir vector use first available point as HRTF.
            let pt = self.points.first().unwrap();

            let len = pt.left_hrtf.len();

            left_hrtf.clear();
            for i in 0..len {
                left_hrtf.push(pt.left_hrtf[i])
            }

            right_hrtf.clear();
            for i in 0..len {
                right_hrtf.push(pt.right_hrtf[i])
            }
        }
    }
}

/// Overlap-add convolution in frequency domain.
///
/// https://en.wikipedia.org/wiki/Overlap%E2%80%93add_method
///
fn convolve(in_buffer: &mut [Complex<f32>],
            out_buffer: &mut [Complex<f32>],
            hrtf: &[Complex<f32>],
            prev_samples: &mut Vec<Complex<f32>>,
            fft: &mut FFTplanner<f32>,
            ifft: &mut FFTplanner<f32>) {
    assert_eq!(hrtf.len(), in_buffer.len());

    fft.plan_fft(in_buffer.len()).process(in_buffer, out_buffer);

    // Multiply HRIR and input signal in frequency domain
    for (s, h) in out_buffer.iter_mut().zip(hrtf.iter()) {
        *s *= *h;
    }

    ifft.plan_fft(in_buffer.len()).process(out_buffer, in_buffer);

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

fn get_pad_len(hrtf_len: usize) -> usize {
    // Total length for each temporary buffer.
    // The value defined by overlap-add convolution method:
    //
    // pad_length = M + N - 1,
    //
    // where M - signal length, N - hrtf length
    Context::SAMPLE_PER_CHANNEL + hrtf_len - 1
}

pub struct HrtfRenderer {
    hrtf_sphere: HrtfSphere,
    left_in_buffer: Vec<Complex<f32>>,
    right_in_buffer: Vec<Complex<f32>>,
    left_out_buffer: Vec<Complex<f32>>,
    right_out_buffer: Vec<Complex<f32>>,
    fft: FFTplanner<f32>,
    ifft: FFTplanner<f32>,
    left_hrtf: Vec<Complex<f32>>,
    right_hrtf: Vec<Complex<f32>>,
}

fn mute(buffer: &mut [Complex<f32>]) {
    for s in buffer {
        *s = Complex::zero();
    }
}

pub(in crate) fn get_raw_samples(source: &mut Source, left: &mut [Complex<f32>], right: &mut [Complex<f32>]) {
    assert_eq!(left.len(), right.len());

    if source.get_status() != Status::Playing {
        mute(left);
        mute(right);
        return;
    }


    let mut anything_sampled = false;

    if let Some(buffer) = source.get_buffer().clone() {
        if let Ok(mut buffer) = buffer.lock() {
            if buffer.is_empty() {
                return;
            }

            for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                if source.get_status() == Status::Playing {
                    // Ignore all channels except left. Only mono sounds can be processed by HRTF.
                    let (raw_left, _) = source.next_sample_pair(&mut buffer);
                    let sample = Complex::new(raw_left, 0.0);
                    *left = sample;
                    *right = sample;
                } else {
                    // Fill rest with zeros
                    *left = Complex::zero();
                    *right = Complex::zero();
                }

                anything_sampled = true;
            }
        }
    }

    if !anything_sampled {
        mute(left);
        mute(right);
    }
}

impl HrtfRenderer {
    pub fn new(hrtf_sphere: HrtfSphere) -> Self {
        let pad_length = get_pad_len(hrtf_sphere.length);

        Self {
            hrtf_sphere,
            left_in_buffer: vec![Complex::zero(); pad_length],
            right_in_buffer: vec![Complex::zero(); pad_length],
            left_out_buffer: vec![Complex::zero(); pad_length],
            right_out_buffer: vec![Complex::zero(); pad_length],
            fft: FFTplanner::new(false),
            ifft: FFTplanner::new(true),
            left_hrtf: Default::default(),
            right_hrtf: Default::default(),
        }
    }

    pub(in crate) fn render_source(&mut self, source: &mut Source, out_buf: &mut [(f32, f32)]) {
        // Still very unoptimal and heavy. TODO: Optimize.
        let pad_length = get_pad_len(self.hrtf_sphere.length);

        if source.last_frame_left_samples.len() != self.hrtf_sphere.length - 1 {
            source.last_frame_left_samples = vec![Complex::zero(); pad_length];
        }
        if source.last_frame_right_samples.len() != self.hrtf_sphere.length - 1 {
            source.last_frame_right_samples = vec![Complex::zero(); pad_length];
        }

        self.hrtf_sphere.sample_bilinear(&mut self.left_hrtf, &mut self.right_hrtf, source.hrtf_sampling_vector);

        // Gather samples for processing.
        get_raw_samples(source, &mut self.left_in_buffer[0..Context::SAMPLE_PER_CHANNEL],
                               &mut self.right_in_buffer[0..Context::SAMPLE_PER_CHANNEL]);

        mute(&mut self.left_in_buffer[Context::SAMPLE_PER_CHANNEL..pad_length]);
        mute(&mut self.right_in_buffer[Context::SAMPLE_PER_CHANNEL..pad_length]);

        convolve(&mut self.left_in_buffer, &mut self.left_out_buffer,
                 &self.left_hrtf, &mut source.last_frame_left_samples,
                 &mut self.fft, &mut self.ifft);

        convolve(&mut self.right_in_buffer, &mut self.right_out_buffer,
                 &self.right_hrtf, &mut source.last_frame_right_samples,
                 &mut self.fft, &mut self.ifft);

        // Mix samples into output buffer with rescaling.
        let k = source.distance_gain / (pad_length as f32);

        // Take only N samples as output data, rest will be used for next frame.
        for (i, (out_left, out_right)) in out_buf.iter_mut().enumerate() {
            let left = self.left_in_buffer.get(i).unwrap();
            *out_left += left.re * k;

            let right = self.right_in_buffer.get(i).unwrap();
            *out_right += right.re * k;
        }
    }
}