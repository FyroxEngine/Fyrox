use crate::visitor::{Visit, VisitResult, Visitor};
use rand::Rng;
use std::fmt::Debug;

/// Fool-proof numeric range which automatically takes care about order of its boundary values.
#[derive(Debug)]
pub struct NumericRange<T> {
    /// Boundary values, there is **no guarantee** that 0==left, 1==right.
    /// You could set any values here, min and max will be calculated on demand.
    pub bounds: [T; 2],
}

impl<T> Clone for NumericRange<T>
where
    T: Clone + Copy,
{
    fn clone(&self) -> Self {
        Self {
            bounds: self.bounds.clone(),
        }
    }
}

impl<T> Copy for NumericRange<T> where T: Copy {}

impl<T> Default for NumericRange<T>
where
    T: Default + Clone + Copy,
{
    fn default() -> Self {
        Self {
            bounds: [Default::default(); 2],
        }
    }
}

impl<T> NumericRange<T>
where
    T: Copy + Sized + rand::distributions::uniform::SampleUniform + Send + PartialOrd + Debug,
{
    pub fn new(a: T, b: T) -> Self {
        Self { bounds: [a, b] }
    }

    pub fn min(&self) -> T {
        if self.bounds[0] < self.bounds[1] {
            self.bounds[0]
        } else {
            self.bounds[1]
        }
    }

    pub fn max(&self) -> T {
        if self.bounds[0] > self.bounds[1] {
            self.bounds[0]
        } else {
            self.bounds[1]
        }
    }

    pub fn random(&self) -> T {
        rand::thread_rng().gen_range(self.min(), self.max())
    }

    pub fn clamp_value(&self, value: &mut T) -> T {
        if *value < self.min() {
            self.min()
        } else if *value > self.max() {
            self.max()
        } else {
            *value
        }
    }
}

impl<T> Visit for NumericRange<T>
where
    T: Copy + Clone + Default + Visit,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // Keep old names for backward compatibility.
        self.bounds[0].visit("Min", visitor)?;
        self.bounds[1].visit("Max", visitor)?;

        visitor.leave_region()
    }
}
