use crate::{
    color::Color,
    math::{cubicf, lerpf},
    visitor::prelude::*,
};
use std::cmp::Ordering;

pub trait Interpolatable: Sized + Clone + Copy + Default {
    fn step(&self, other: &Self, t: f32) -> Self {
        if t.eq(&1.0) {
            *other
        } else {
            *self
        }
    }

    fn lerp(&self, other: &Self, t: f32) -> Self;

    fn cubic(&self, other: &Self, t: f32, m0: f32, m1: f32) -> Self;
}

impl Interpolatable for f32 {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        lerpf(*self, *other, t)
    }

    // https://en.wikipedia.org/wiki/Cubic_Hermite_spline
    fn cubic(&self, other: &Self, t: f32, m0: f32, m1: f32) -> Self {
        cubicf(*self, *other, t, m0, m1)
    }
}

impl Interpolatable for Color {
    fn lerp(&self, other: &Self, t: f32) -> Self {
        let p1 = self.as_frgba();
        let p2 = other.as_frgba();
        let k = p1.lerp(&p2, t).scale(255.0);
        Color::from_rgba(k.x as u8, k.y as u8, k.z as u8, k.w as u8)
    }

    fn cubic(&self, other: &Self, t: f32, m0: f32, m1: f32) -> Self {
        let p0 = self.as_frgba();
        let p1 = other.as_frgba();
        let r = cubicf(p0.x, p1.x, t, m0, m1) * 255.0;
        let g = cubicf(p0.y, p1.y, t, m0, m1) * 255.0;
        let b = cubicf(p0.z, p1.z, t, m0, m1) * 255.0;
        let a = cubicf(p0.w, p1.w, t, m0, m1) * 255.0;
        Color::from_rgba(r as u8, g as u8, b as u8, a as u8)
    }
}

#[derive(Visit)]
pub enum CurvePointKind {
    Constant,
    Linear,
    Cubic {
        left_tangent: f32,
        right_tangent: f32,
    },
}

#[derive(Visit)]
pub struct CurvePoint<T: Interpolatable> {
    location: f32,
    pub value: T,
    pub kind: CurvePointKind,
}

impl<T: Interpolatable> CurvePoint<T> {
    pub fn interpolate(&self, other: &Self, t: f32) -> T {
        match self.kind {
            CurvePointKind::Constant => self.value.step(&other.value, t),
            CurvePointKind::Linear => self.value.lerp(&other.value, t),
            CurvePointKind::Cubic {
                left_tangent,
                right_tangent,
            } => self
                .value
                .cubic(&other.value, t, left_tangent, right_tangent),
        }
    }
}

#[derive(Visit, Default)]
pub struct Curve<T: Interpolatable> {
    points: Vec<CurvePoint<T>>,
}

fn sort_points<T: Interpolatable>(points: &mut [CurvePoint<T>]) {
    points.sort_by(|a, b| {
        if a.location < b.location {
            Ordering::Less
        } else if a.location > b.location {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
}

impl<T: Interpolatable> From<Vec<CurvePoint<T>>> for Curve<T> {
    fn from(mut points: Vec<CurvePoint<T>>) -> Self {
        sort_points(&mut points);
        Self { points }
    }
}

impl<T: Interpolatable> Curve<T> {
    pub fn clear(&mut self) {
        self.points.clear()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn add_point(&mut self, new_point: CurvePoint<T>) {
        let mut insert_at = 0;
        for (i, point) in self.points.iter().enumerate() {
            if new_point.location < point.location {
                insert_at = i;
                break;
            }
        }
        self.points.insert(insert_at, new_point);
    }

    pub fn move_point(&mut self, point_id: usize, location: f32) {
        if let Some(point) = self.points.get_mut(point_id) {
            point.location = location;
            sort_points(&mut self.points);
        }
    }

    pub fn value_at(&self, location: f32) -> T {
        if self.points.is_empty() {
            // stub - zero
            return Default::default();
        } else if self.points.len() == 1 {
            // single point - just return its value
            return self.points.first().unwrap().value;
        } else if self.points.len() == 2 {
            // special case for two points (much faster than generic)
            let pt_a = self.points.get(0).unwrap();
            let pt_b = self.points.get(1).unwrap();
            if location >= pt_a.location && location <= pt_b.location {
                // linear interpolation
                let span = pt_b.location - pt_a.location;
                let t = (location - pt_a.location) / span;
                return pt_a.interpolate(pt_b, t);
            } else if location < pt_a.location {
                return pt_a.value;
            } else {
                return pt_b.value;
            }
        }

        // generic case
        // check for out-of-bounds
        let first = self.points.first().unwrap();
        let last = self.points.last().unwrap();
        if location <= first.location {
            first.value
        } else if location >= last.location {
            last.value
        } else {
            // find span first
            let mut pt_a_index = 0;
            for (i, pt) in self.points.iter().enumerate() {
                if location >= pt.location {
                    pt_a_index = i;
                }
            }
            let pt_b_index = pt_a_index + 1;

            let pt_a = self.points.get(pt_a_index).unwrap();
            let pt_b = self.points.get(pt_b_index).unwrap();

            let span = pt_b.location - pt_a.location;
            let t = (location - pt_a.location) / span;
            pt_a.interpolate(pt_b, t)
        }
    }
}
