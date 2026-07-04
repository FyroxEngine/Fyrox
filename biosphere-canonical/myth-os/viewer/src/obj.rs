use std::collections::HashMap;
use std::path::Path;

/// Parse a Wavefront OBJ file.
///
/// Returns (positions, normals, indices). Fan-triangulates any quads.
/// If the file has no normals, they are computed from face geometry.
pub fn load(path: &Path) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>), String> {
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    let mut raw_pos: Vec<[f32; 3]> = Vec::new();
    let mut raw_nrm: Vec<[f32; 3]> = Vec::new();

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut indices:   Vec<u32>       = Vec::new();
    let mut map:       HashMap<(u32, u32), u32> = HashMap::new();

    for line in src.lines() {
        let mut parts = line.split_ascii_whitespace();
        match parts.next() {
            Some("v") => {
                raw_pos.push(parse3(&mut parts)?);
            }
            Some("vn") => {
                raw_nrm.push(parse3(&mut parts)?);
            }
            Some("f") => {
                // Each token: v, v/vt, v//vn, v/vt/vn (all 1-based)
                let verts: Vec<(u32, u32)> = parts
                    .map(|tok| {
                        let mut segs = tok.splitn(3, '/');
                        let pi = segs.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1) - 1;
                        let _  = segs.next(); // skip vt
                        let ni = segs.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
                        let ni = if ni > 0 { ni - 1 } else { 0 };
                        (pi, ni)
                    })
                    .collect();

                // Fan triangulation (handles tris and quads cleanly)
                for i in 1..(verts.len().saturating_sub(1)) {
                    for &(pi, ni) in &[verts[0], verts[i], verts[i + 1]] {
                        let vi = *map.entry((pi, ni)).or_insert_with(|| {
                            let idx = positions.len() as u32;
                            positions.push(*raw_pos.get(pi as usize).unwrap_or(&[0.0; 3]));
                            normals.push(*raw_nrm.get(ni as usize).unwrap_or(&[0.0; 3]));
                            idx
                        });
                        indices.push(vi);
                    }
                }
            }
            _ => {}
        }
    }

    if raw_nrm.is_empty() {
        normals = compute_flat_normals(&positions, &indices);
    }

    Ok((positions, normals, indices))
}

fn parse3<'a>(parts: &mut impl Iterator<Item = &'a str>) -> Result<[f32; 3], String> {
    let x = parts.next().ok_or("missing x")?.parse::<f32>().map_err(|e| e.to_string())?;
    let y = parts.next().ok_or("missing y")?.parse::<f32>().map_err(|e| e.to_string())?;
    let z = parts.next().ok_or("missing z")?.parse::<f32>().map_err(|e| e.to_string())?;
    Ok([x, y, z])
}

fn compute_flat_normals(positions: &[[f32; 3]], indices: &[u32]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let [ax, ay, az] = positions[tri[0] as usize];
        let [bx, by, bz] = positions[tri[1] as usize];
        let [cx, cy, cz] = positions[tri[2] as usize];
        let (ux, uy, uz) = (bx - ax, by - ay, bz - az);
        let (vx, vy, vz) = (cx - ax, cy - ay, cz - az);
        let nx = uy * vz - uz * vy;
        let ny = uz * vx - ux * vz;
        let nz = ux * vy - uy * vx;
        for &i in tri {
            let n = &mut normals[i as usize];
            n[0] += nx;
            n[1] += ny;
            n[2] += nz;
        }
    }
    for n in &mut normals {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 1e-7 {
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        }
    }
    normals
}
