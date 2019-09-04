use crate::{
    utils::visitor::{Visit, Visitor, VisitResult},
    gui::draw::Color,
};
use std::cmp::Ordering;

#[derive(Debug)]
pub struct GradientPoint {
    location: f32,
    color: Color,
}

impl Visit for GradientPoint {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.location.visit("Location", visitor)?;
        self.color.visit("Color", visitor)?;

        visitor.leave_region()
    }
}

impl GradientPoint {
    pub fn new(location: f32, color: Color) -> Self {
        Self {
            location,
            color,
        }
    }
}

impl Default for GradientPoint {
    fn default() -> Self {
        Self {
            location: 0.0,
            color: Color::white(),
        }
    }
}

impl Clone for GradientPoint {
    fn clone(&self) -> Self {
        Self {
            location: self.location,
            color: self.color,
        }
    }
}

pub struct ColorGradient {
    points: Vec<GradientPoint>
}

impl Clone for ColorGradient {
    fn clone(&self) -> Self {
        Self {
            points: self.points.clone()
        }
    }
}

impl Visit for ColorGradient {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.points.visit("Points", visitor)?;

        visitor.leave_region()
    }
}

impl Default for ColorGradient {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorGradient {
    pub fn new() -> Self {
        Self {
            points: Vec::new()
        }
    }

    pub fn add_point(&mut self, pt: GradientPoint) {
        self.points.push(pt);

        self.points.sort_by(|a, b| {
            if a.location < b.location {
                Ordering::Less
            } else if a.location > b.location {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
    }

    pub fn get_color(&self, location: f32) -> Color {
        if self.points.is_empty() {
            // stub - opaque white
            return Color::white();
        } else if self.points.len() == 1 {
            // single point - just return its color
            return self.points.first().unwrap().color;
        } else if self.points.len() == 2 {
            // special case for two points (much faster than generic)
            // "unsafe" is safe here because we already checked bounds
            let pt_a = self.points.get(0).unwrap();
            let pt_b = self.points.get(1).unwrap();
            if location >= pt_a.location && location <= pt_b.location {
                // linear interpolation
                let span = pt_b.location - pt_a.location;
                let t = (location - pt_a.location) / span;
                return pt_a.color.lerp(pt_b.color, t);
            } else if location < pt_a.location {
                return pt_a.color;
            } else {
                return pt_b.color;
            }
        }

        // generic case
        // check for out-of-bounds
        let first = self.points.first().unwrap();
        let last = self.points.last().unwrap();
        if location <= first.location {
            first.color
        } else if location >= last.location {
            last.color
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

            // linear interpolation
            let span = pt_b.location - pt_a.location;
            let t = (location - pt_a.location) / span;
            pt_a.color.lerp(pt_b.color, t)
        }
    }

    pub fn clear(&mut self) {
        self.points.clear()
    }
}