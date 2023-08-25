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
    fn random<R: Rng>(&self, rng: &mut R) -> T;

    fn clamp_value(&self, value: &mut T) -> T;
}

impl<T: Num + PartialOrd + SampleUniform + Copy> RangeExt<T> for Range<T> {
    #[inline]
    fn random<R: Rng>(&self, rng: &mut R) -> T {
        let start = min(self.start, self.end);
        let end = max(self.start, self.end);
        if start == end {
            start
        } else {
            rng.gen_range(Range { start, end })
        }
    }

    #[inline]
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

#[cfg(test)]
mod test {
    use rand::thread_rng;

    use super::*;

    #[test]
    fn test_random() {
        let mut rng = thread_rng();

        let res = (1..10).random(&mut rng);
        assert!((1..=10).contains(&res));

        let res = Range { start: 10, end: 1 }.random(&mut rng);
        assert!((1..=10).contains(&res));

        let res = (1..1).random(&mut rng);
        assert!(res == 1);
    }

    #[test]
    fn test_clamp_value() {
        let res = (1..10).clamp_value(&mut 5);
        assert_eq!(res, 5);

        let res = (1..10).clamp_value(&mut 0);
        assert_eq!(res, 1);

        let res = (1..10).clamp_value(&mut 11);
        assert_eq!(res, 10);
    }
}
