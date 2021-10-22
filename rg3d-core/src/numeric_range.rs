use crate::num_traits::Num;
use rand::{distributions::uniform::SampleUniform, Rng};
use std::ops::Range;

fn min<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a > b {
        b
    } else {
        a
    }
}

fn max<T>(a: T, b: T) -> T
where
    T: PartialOrd,
{
    if a > b {
        a
    } else {
        b
    }
}

pub trait RangeExt<T>
where
    T: Num + PartialOrd + SampleUniform + Copy,
{
    fn random(&self) -> T;

    fn clamp_value(&self, value: &mut T) -> T;
}

impl<T: Num + PartialOrd + SampleUniform + Copy> RangeExt<T> for Range<T> {
    fn random(&self) -> T {
        let start = min(self.start, self.end);
        let end = max(self.start, self.end);
        rand::thread_rng().gen_range(Range { start, end })
    }

    fn clamp_value(&self, value: &mut T) -> T {
        let start = min(self.start, self.end);
        let end = max(self.start, self.end);

        if *value < start {
            start
        } else if *value > end {
            end
        } else {
            *value
        }
    }
}
