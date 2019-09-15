use rand::Rng;
use crate::visitor::{Visitor, Visit, VisitResult};

pub struct NumericRange<T> {
    pub min: T,
    pub max: T,
}

impl<T> Clone for NumericRange<T> where T: Clone + Copy {
    fn clone(&self) -> Self {
        Self {
            min: self.min,
            max: self.max
        }
    }
}

impl<T> Copy for NumericRange<T> where T: Copy {}

impl<T> Default for NumericRange<T> where T: Default {
    fn default() -> Self {
        Self {
            min: Default::default(),
            max: Default::default(),
        }
    }
}

impl<T> NumericRange<T> where T: Copy + Sized + rand::distributions::uniform::SampleUniform + Send {
    pub fn new(min: T, max: T) -> Self {
        Self {
            min,
            max,
        }
    }

    pub fn random(&self) -> T {
        rand::thread_rng().gen_range(self.min, self.max)
    }
}

impl<T> Visit for NumericRange<T> where T: Copy + Clone + Default + Visit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.min.visit("Min", visitor)?;
        self.max.visit("Max", visitor)?;

        visitor.leave_region()
    }
}