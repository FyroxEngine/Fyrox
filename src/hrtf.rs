use rustfft::{
    num_complex::Complex,
    num_traits::Zero,
    FFTplanner,
};
use crate::context::Context;
use std::{
    fs::File,
    path::Path,
    io::{BufReader, Read, Error}
};
use rg3d_core::math::{
    get_barycentric_coords,
    vec3::Vec3,
    ray::Ray
};
use byteorder::{
    ReadBytesExt,
    LittleEndian
};

pub struct HrtfPoint {
    pub(in crate) pos: Vec3,
    pub(in crate) left_hrir_spectrum: Vec<Complex<f32>>,
    pub(in crate) right_hrir_spectrum: Vec<Complex<f32>>,
}

struct Face {
    a: usize,
    b: usize,
    c: usize,
}

pub struct Hrtf {
    pub(in crate) length: usize,
    pub(in crate) points: Vec<HrtfPoint>,
    faces: Vec<Face>,
}

#[derive(Debug)]
pub enum HrtfError {
    IoError(std::io::Error),
    InvalidFileFormat,
}

impl From<std::io::Error> for HrtfError {
    fn from(io_err: Error) -> Self {
        HrtfError::IoError(io_err)
    }
}

fn make_hrtf(mut raw_hrir: Vec<Complex<f32>>, pad_length: usize, planner: &mut FFTplanner<f32>) -> Vec<Complex<f32>> {
    for _ in raw_hrir.len()..pad_length {
        // Pad with zeros to length of context's output buffer.
        raw_hrir.push(Complex::zero());
    }
    let mut hrir_spectrum = vec![Complex::zero(); pad_length];
    planner.plan_fft(pad_length)
        .process(raw_hrir.as_mut(), hrir_spectrum.as_mut());
    hrir_spectrum
}

impl Hrtf {
    pub fn new(path: &Path) -> Result<Hrtf, HrtfError> {
        let mut reader = BufReader::new(File::open(path)?);

        let mut magic = [0; 4];
        reader.read_exact(&mut magic)?;
        if magic[0] != b'H' && magic[1] != b'R' && magic[2] != b'I' && magic[3] != b'R' {
            return Err(HrtfError::InvalidFileFormat);
        }

        let sample_rate = reader.read_u32::<LittleEndian>()?;
        let length = reader.read_u32::<LittleEndian>()? as usize;
        let vertex_count = reader.read_u32::<LittleEndian>()? as usize;
        let index_count = reader.read_u32::<LittleEndian>()? as usize;

        let mut indices = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            indices.push(reader.read_u32::<LittleEndian>()?);
        }
        let faces = indices.chunks(3)
            .map(|f| Face { a: f[0] as usize, b: f[1] as usize, c: f[2] as usize })
            .collect();

        let mut planner = FFTplanner::new(false);
        let pad_length = Context::SAMPLE_PER_CHANNEL + length - 1;

        let mut points = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let x = reader.read_f32::<LittleEndian>()?;
            let y = reader.read_f32::<LittleEndian>()?;
            let z = reader.read_f32::<LittleEndian>()?;

            let mut left_hrir = Vec::with_capacity(pad_length);
            for _ in 0..length {
                left_hrir.push(Complex::new(reader.read_f32::<LittleEndian>()?, 0.0));
            }
            let left_hrir_spectrum = make_hrtf(left_hrir, pad_length, &mut planner);

            let mut right_hrir = Vec::with_capacity(pad_length);
            for _ in 0..length {
                right_hrir.push(Complex::new(reader.read_f32::<LittleEndian>()?, 0.0));
            }
            let right_hrir_spectrum = make_hrtf(right_hrir, pad_length, &mut planner);

            points.push(HrtfPoint {
                pos: Vec3::new(x, y, z),
                left_hrir_spectrum,
                right_hrir_spectrum,
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
    pub fn sample_bilinear(&self, result: &mut HrtfPoint, dir: Vec3) {
        let ray = Ray::from_two_points(&Vec3::ZERO, &dir.scale(10.0)).unwrap();
        for face in self.faces.iter() {
            let a = self.points.get(face.a).unwrap();
            let b = self.points.get(face.b).unwrap();
            let c = self.points.get(face.c).unwrap();

            if let Some(p) = ray.triangle_intersection(&[a.pos, b.pos, c.pos]) {
                let (ka, kb, kc) = get_barycentric_coords(&p, &a.pos, &b.pos, &c.pos);

                let len = a.left_hrir_spectrum.len();

                result.left_hrir_spectrum.clear();
                for i in 0..len {
                    result.left_hrir_spectrum.push(a.left_hrir_spectrum[i] * ka + b.left_hrir_spectrum[i] * kb + c.left_hrir_spectrum[i] * kc)
                }

                result.right_hrir_spectrum.clear();
                for i in 0..len {
                    result.right_hrir_spectrum.push(a.right_hrir_spectrum[i] * ka + b.right_hrir_spectrum[i] * kb + c.right_hrir_spectrum[i] * kc)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_get_hrir_spectrum() {}
}
