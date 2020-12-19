use crate::visitor::{Visit, VisitResult, Visitor};
use rand::Rng;
use std::fmt::Debug;

/// Fool-proof numeric range which automatically takes care about order of its boundary values.
#[derive(Debug, Copy, Clone)]
pub struct NumericRange {
    /// Boundary values, there is **no guarantee** that 0==left, 1==right.
    /// You could set any values here, min and max will be calculated on demand.
    pub bounds: [f32; 2],
}

impl NumericRange {
    pub fn new(a: f32, b: f32) -> Self {
        Self { bounds: [a, b] }
    }

    pub fn min(&self) -> f32 {
        if self.bounds[0] < self.bounds[1] {
            self.bounds[0]
        } else {
            self.bounds[1]
        }
    }

    pub fn max(&self) -> f32 {
        if self.bounds[0] > self.bounds[1] {
            self.bounds[0]
        } else {
            self.bounds[1]
        }
    }

    pub fn random(&self) -> f32 {
        let random = rand::thread_rng().gen::<f32>();
        let min = self.min();
        let max = self.max();
        min + random * (max - min)
    }

    pub fn clamp_value(&self, value: &mut f32) -> f32 {
        if *value < self.min() {
            self.min()
        } else if *value > self.max() {
            self.max()
        } else {
            *value
        }
    }
}

impl Visit for NumericRange {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Keep old names for backward compatibility.
        self.bounds[0].visit("Min", visitor)?;
        self.bounds[1].visit("Max", visitor)?;

        visitor.leave_region()
    }
}
