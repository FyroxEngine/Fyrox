#![warn(clippy::too_many_arguments)]

pub mod csm;
pub mod point;
pub mod spot;

pub fn cascade_size(base_size: usize, cascade: usize) -> usize {
    match cascade {
        0 => base_size,
        1 => (base_size / 2).max(1),
        2 => (base_size / 4).max(1),
        _ => unreachable!(),
    }
}
