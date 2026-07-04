/// field_reader.rs — Read a .raw scalar field from disk into an ndarray.
///
/// The .raw format is exactly what mandelbulb::output writes:
/// a flat sequence of 32-bit little-endian floats in [z][y][x] order.
/// This matches Acropora's "Export Voxel Data" format (§6.4).

use ndarray::Array3;
use std::io::Read;
use std::path::Path;

// ---------------------------------------------------------------------------
// ScalarField
// ---------------------------------------------------------------------------

/// A loaded 3D scalar field ready for iso-surface extraction.
#[derive(Debug)]
pub struct ScalarField {
    /// Raw distance/density values. Shape: [z, y, x].
    pub data: Array3<f32>,

    /// Resolution per axis (cube assumed).
    pub resolution: u32,

    /// Surface boundary. Voxels with value ≤ iso_threshold are inside.
    pub iso_threshold: f32,

    /// Spatial extent of each axis. Always [-2.0, 2.0] for standard fields.
    pub bounds_min: f32,
    pub bounds_max: f32,
}

impl ScalarField {
    /// World-space position of a voxel center given its [iz, iy, ix] index.
    #[inline]
    pub fn voxel_center(&self, iz: usize, iy: usize, ix: usize) -> glam::Vec3 {
        let res  = self.resolution as f32;
        let step = (self.bounds_max - self.bounds_min) / res;
        glam::Vec3::new(
            self.bounds_min + (ix as f32 + 0.5) * step,
            self.bounds_min + (iy as f32 + 0.5) * step,
            self.bounds_min + (iz as f32 + 0.5) * step,
        )
    }

    /// World-space position of a voxel corner at [iz, iy, ix].
    #[inline]
    pub fn voxel_corner(&self, iz: usize, iy: usize, ix: usize) -> glam::Vec3 {
        let res  = self.resolution as f32;
        let step = (self.bounds_max - self.bounds_min) / res;
        glam::Vec3::new(
            self.bounds_min + ix as f32 * step,
            self.bounds_min + iy as f32 * step,
            self.bounds_min + iz as f32 * step,
        )
    }

    /// Step size between adjacent voxel corners.
    #[inline]
    pub fn step(&self) -> f32 {
        (self.bounds_max - self.bounds_min) / self.resolution as f32
    }

    /// Get scalar value at [iz, iy, ix], returning f32::MAX if out of bounds.
    /// f32::MAX is treated as outside the surface (safe for boundary marching).
    #[inline]
    pub fn get(&self, iz: isize, iy: isize, ix: isize) -> f32 {
        let res = self.resolution as isize;
        if iz < 0 || iy < 0 || ix < 0 || iz >= res || iy >= res || ix >= res {
            return f32::MAX;
        }
        self.data[[iz as usize, iy as usize, ix as usize]]
    }

    /// Returns true if the voxel at [iz, iy, ix] is on or inside the surface.
    #[inline]
    pub fn is_inside(&self, iz: isize, iy: isize, ix: isize) -> bool {
        self.get(iz, iy, ix) <= self.iso_threshold
    }

    /// Total voxel count.
    #[inline]
    pub fn voxel_count(&self) -> usize {
        let r = self.resolution as usize;
        r * r * r
    }

    /// Count of voxels inside the surface.
    pub fn interior_count(&self) -> usize {
        self.data.iter().filter(|&&v| v <= self.iso_threshold).count()
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// Read errors.
#[derive(Debug)]
pub enum ReadError {
    Io(String),
    SizeMismatch { expected: usize, found: usize },
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {msg}"),
            Self::SizeMismatch { expected, found } => {
                write!(f, "field size mismatch: expected {expected} floats, found {found}")
            }
        }
    }
}

impl std::error::Error for ReadError {}

/// Read a .raw scalar field from disk.
///
/// `dimensions` is [x, y, z] voxel count per axis (from the WirePacket).
/// `iso_threshold` is passed through from the mandelbulb output.
pub fn read_field(
    path:          &Path,
    dimensions:    [u32; 3],
    iso_threshold: f32,
) -> Result<ScalarField, ReadError> {
    let resolution = dimensions[0]; // cube assumed — all three should match
    let expected   = (resolution as usize).pow(3);

    // Read raw bytes
    let mut file = std::fs::File::open(path)
        .map_err(|e| ReadError::Io(e.to_string()))?;

    let mut bytes = Vec::with_capacity(expected * 4);
    file.read_to_end(&mut bytes)
        .map_err(|e| ReadError::Io(e.to_string()))?;

    // Convert bytes → f32 values (little-endian)
    let found = bytes.len() / 4;
    if found != expected {
        return Err(ReadError::SizeMismatch { expected, found });
    }

    let floats: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    // Reshape into Array3<f32> with shape [z, y, x] — same layout as writer
    let res  = resolution as usize;
    let data = Array3::from_shape_vec((res, res, res), floats)
        .map_err(|e| ReadError::Io(e.to_string()))?;

    Ok(ScalarField {
        data,
        resolution,
        iso_threshold,
        bounds_min: -2.0,
        bounds_max:  2.0,
    })
}

/// Build a ScalarField directly from an existing Array3 (used in tests).
pub fn field_from_array(
    data:          Array3<f32>,
    iso_threshold: f32,
) -> ScalarField {
    let resolution = data.shape()[0] as u32;
    ScalarField {
        data,
        resolution,
        iso_threshold,
        bounds_min: -2.0,
        bounds_max:  2.0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;
    use std::io::Write;

    fn write_test_raw(path: &std::path::Path, data: &[f32]) {
        let mut f = std::fs::File::create(path).unwrap();
        for &v in data {
            f.write_all(&v.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn reads_correct_shape() {
        let tmp  = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.raw");
        let res  = 4_u32;
        let data: Vec<f32> = (0..res.pow(3)).map(|i| i as f32).collect();
        write_test_raw(&path, &data);

        let field = read_field(&path, [res, res, res], 0.5).unwrap();
        assert_eq!(field.data.shape(), &[4, 4, 4]);
        assert_eq!(field.resolution, 4);
    }

    #[test]
    fn size_mismatch_returns_error() {
        let tmp  = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.raw");
        // Write fewer floats than 4^3 = 64
        write_test_raw(&path, &[1.0_f32; 10]);
        let result = read_field(&path, [4, 4, 4], 0.5);
        assert!(matches!(result, Err(ReadError::SizeMismatch { .. })));
    }

    #[test]
    fn voxel_center_at_origin_index() {
        let data  = Array3::from_elem((4, 4, 4), 1.0_f32);
        let field = field_from_array(data, 0.5);
        // step = 4.0 / 4 = 1.0; corner 0 = -2.0; center 0 = -1.5
        let center = field.voxel_center(0, 0, 0);
        assert!((center.x - -1.5).abs() < 1e-5);
        assert!((center.y - -1.5).abs() < 1e-5);
        assert!((center.z - -1.5).abs() < 1e-5);
    }

    #[test]
    fn get_out_of_bounds_returns_max() {
        let data  = Array3::from_elem((4, 4, 4), 0.0_f32);
        let field = field_from_array(data, 0.5);
        assert_eq!(field.get(-1, 0, 0), f32::MAX);
        assert_eq!(field.get(0, 4, 0),  f32::MAX);
    }

    #[test]
    fn is_inside_threshold() {
        let mut data = Array3::from_elem((4, 4, 4), 1.0_f32);
        data[[2, 2, 2]] = 0.0; // inside
        let field = field_from_array(data, 0.5);
        assert!( field.is_inside(2, 2, 2));
        assert!(!field.is_inside(0, 0, 0));
    }

    #[test]
    fn interior_count_correct() {
        let mut data = Array3::from_elem((4, 4, 4), 1.0_f32);
        data[[1, 1, 1]] = 0.0;
        data[[2, 2, 2]] = 0.0;
        let field = field_from_array(data, 0.5);
        assert_eq!(field.interior_count(), 2);
    }
}
