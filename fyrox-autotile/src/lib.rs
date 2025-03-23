// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Autotiling allows you to fill the content of a grid according to pre-defined rules.
//! Tiles are assigned frequencies and adjacency rules, and then an algorithm is used
//! to attempt to fill a given area with tiles in accordance with the rules and frequencies.
//!
//! This library supports both deterministic autotiling and wave function collapse.
//! Deterministic autotiling tries to find the particular tile that the user wants
//! based upon giving specification for the purpose of streamlining creating tile-based art.
//! Wave function collapse is a process of automatically generating random content where
//! the algorithm is giving freedom to choose whatever tiles it likes within the limits
//! of the adjacency rules.
//!
//! The autotiling algorithm is based upon [Terrain Autotiler](https://github.com/dandeliondino/terrain-autotiler).
//!
//! The wave function collapse algorith is based upon [fast-wfc](https://github.com/math-fehr/fast-wfc).
#![warn(missing_docs)]

use std::{collections::hash_map::Entry, fmt::Debug, hash::Hash};

use fxhash::FxHashMap;
use nalgebra::{Vector2, Vector3};
use rand::Rng;

mod auto;
mod wave;

pub use auto::*;
pub use wave::*;

/// Position types for tiles that provides abstract access to all of the position's
/// adjacent positions.
pub trait OffsetPosition:
    Eq
    + Hash
    + Clone
    + std::ops::Add<Self::Offset, Output = Self>
    + std::ops::Add<Self::Diagonal, Output = Self>
{
    /// An offset from a position to one of its adjacent positions.
    /// Since `OffsetPosition` implements `Add<Offset, Ouput=Self>`, an offset can be added to a position
    /// to get the adjacent position.
    type Offset: std::ops::Neg<Output = Self::Offset> + Clone;
    /// A diagonal offset from a position to one of the positions that are nearby but not adjacent.
    /// Since `OffsetPosition` implements `Add<Diagonal, Ouput=Self>`, a diagonal can be added to a position
    /// to get the diagonal position.
    type Diagonal: std::ops::Neg<Output = Self::Diagonal> + Clone;
    /// An iterator over all offsets, giving access to all adjacent positions from any position by using
    /// position arithmetic: position + offset == adjacent position.
    fn all_offsets() -> impl Iterator<Item = Self::Offset>;
    /// An iterator over all diagonals, giving access to nearby non-adjacent positions by using position
    /// arithmetic: position + diagonal == nearby position.
    fn all_diagonals() -> impl Iterator<Item = Self::Diagonal>;
}

impl OffsetPosition for Vector2<i32> {
    type Offset = Vector2Offset;
    type Diagonal = Vector2Diagonal;
    fn all_offsets() -> impl Iterator<Item = Self::Offset> {
        (0..4).map(Vector2Offset)
    }
    fn all_diagonals() -> impl Iterator<Item = Self::Diagonal> {
        (0..4).map(Vector2Diagonal)
    }
}

/// An offset to move from a `Vector2<i32>` position to an diagonal `Vector2<i32>`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Vector2Diagonal(usize);

impl Vector2Diagonal {
    /// Diagonal (-1, -1)
    pub const LEFT_DOWN: Vector2Diagonal = Vector2Diagonal(0);
    /// Diagonal (1, -1)
    pub const RIGHT_DOWN: Vector2Diagonal = Vector2Diagonal(1);
    /// Diagonal (-1, 1)
    pub const LEFT_UP: Vector2Diagonal = Vector2Diagonal(2);
    /// Diagonal (1, 1)
    pub const RIGHT_UP: Vector2Diagonal = Vector2Diagonal(3);
    /// The bit in a `PatternBits` object that corresponds to this diagonal.
    /// The returned bit is the peer that must match against the diagonal pattern
    /// in this direction.
    fn peering_bit(&self) -> usize {
        DIAGONAL_PEERING_BITS[self.0]
    }
    /// The x difference of the offset, so position.x + dx == adjacent.x
    pub fn dx(&self) -> i32 {
        DIAG2[self.0].x
    }
    /// The y difference of the offset, so position.y + dy == adjacent.y
    pub fn dy(&self) -> i32 {
        DIAG2[self.0].y
    }
}

impl Debug for Vector2Diagonal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vector2Diagonal")
            .field("dx", &self.dx())
            .field("dy", &self.dy())
            .finish()
    }
}

impl std::ops::Add<Vector2Diagonal> for Vector2<i32> {
    type Output = Vector2<i32>;

    fn add(self, rhs: Vector2Diagonal) -> Self::Output {
        self + DIAG2[rhs.0]
    }
}

impl std::ops::Neg for Vector2Diagonal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vector2Diagonal(3 - self.0)
    }
}

/// An offset to move from a `Vector2<i32>` position to an adjacent `Vector2<i32>`
/// in one of the four cardinal directions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Vector2Offset(usize);

const OFFSETS2: [Vector2<i32>; 4] = [
    Vector2::new(-1, 0),
    Vector2::new(0, -1),
    Vector2::new(0, 1),
    Vector2::new(1, 0),
];

const DIAG2: [Vector2<i32>; 4] = [
    Vector2::new(-1, -1),
    Vector2::new(1, -1),
    Vector2::new(-1, 1),
    Vector2::new(1, 1),
];

const fn bit_pos(x: usize, y: usize) -> usize {
    x + y * 3
}

const OFFSET_PEERING_BITS: [[usize; 3]; 4] = [
    [bit_pos(0, 0), bit_pos(0, 1), bit_pos(0, 2)],
    [bit_pos(0, 0), bit_pos(1, 0), bit_pos(2, 0)],
    [bit_pos(0, 2), bit_pos(1, 2), bit_pos(2, 2)],
    [bit_pos(2, 0), bit_pos(2, 1), bit_pos(2, 2)],
];

const DIAGONAL_PEERING_BITS: [usize; 4] =
    [bit_pos(0, 0), bit_pos(2, 0), bit_pos(0, 2), bit_pos(2, 2)];

impl Vector2Offset {
    /// Offset (0, 1)
    pub const UP: Vector2Offset = Vector2Offset(2);
    /// Offset (0, -1)
    pub const DOWN: Vector2Offset = Vector2Offset(1);
    /// Offset (-1, 0)
    pub const LEFT: Vector2Offset = Vector2Offset(0);
    /// Offset (1, 0)
    pub const RIGHT: Vector2Offset = Vector2Offset(3);
    /// Iterator over the three peering bits that are along the edge
    /// of a 3x3 pattern grid in the direction of this offset.
    fn peering_bits(&self) -> impl Iterator<Item = usize> {
        OFFSET_PEERING_BITS[self.0].iter().copied()
    }
    /// The x difference of the offset, so position.x + dx == adjacent.x
    pub fn dx(&self) -> i32 {
        OFFSETS2[self.0].x
    }
    /// The y difference of the offset, so position.y + dy == adjacent.y
    pub fn dy(&self) -> i32 {
        OFFSETS2[self.0].y
    }
}

impl Debug for Vector2Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vector2Offset")
            .field("dx", &self.dx())
            .field("dy", &self.dy())
            .finish()
    }
}

impl From<Vector2Offset> for Vector2<i32> {
    fn from(value: Vector2Offset) -> Self {
        OFFSETS2[value.0]
    }
}

impl std::ops::Add<Vector2Offset> for Vector2<i32> {
    type Output = Vector2<i32>;

    fn add(self, rhs: Vector2Offset) -> Self::Output {
        self + OFFSETS2[rhs.0]
    }
}

impl std::ops::Neg for Vector2Offset {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vector2Offset(3 - self.0)
    }
}

impl OffsetPosition for Vector3<i32> {
    type Offset = Vector3Offset;
    type Diagonal = Vector3Diagonal;
    fn all_offsets() -> impl Iterator<Item = Self::Offset> {
        (0..6).map(Vector3Offset)
    }
    fn all_diagonals() -> impl Iterator<Item = Self::Diagonal> {
        (0..DIAG3.len()).map(|i| DIAG3[i]).map(Vector3Diagonal)
    }
}

/// An offset to move from a `Vector2<i32>` position to an diagonal `Vector2<i32>`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Vector3Diagonal(Vector3<i32>);

const DIAG3: [Vector3<i32>; 20] = [
    Vector3::new(-1, -1, -1),
    Vector3::new(1, -1, -1),
    Vector3::new(-1, 1, -1),
    Vector3::new(1, 1, -1),
    Vector3::new(-1, -1, 0),
    Vector3::new(1, -1, 0),
    Vector3::new(-1, 1, 0),
    Vector3::new(1, 1, 0),
    Vector3::new(-1, -1, 1),
    Vector3::new(1, -1, 1),
    Vector3::new(-1, 1, 1),
    Vector3::new(1, 1, 1),
    Vector3::new(-1, 0, -1),
    Vector3::new(0, -1, -1),
    Vector3::new(1, 0, -1),
    Vector3::new(0, 1, -1),
    Vector3::new(-1, 0, 1),
    Vector3::new(0, -1, 1),
    Vector3::new(1, 0, 1),
    Vector3::new(0, 1, 1),
];

impl std::ops::Add<Vector3Diagonal> for Vector3<i32> {
    type Output = Vector3<i32>;

    fn add(self, rhs: Vector3Diagonal) -> Self::Output {
        self + rhs.0
    }
}

impl std::ops::Neg for Vector3Diagonal {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vector3Diagonal(-self.0)
    }
}

/// An offset to move from a `Vector3<i32>` position to an adjacent `Vector3<i32>`
/// in one of the six cardinal directions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Vector3Offset(usize);

impl Vector3Offset {
    /// The x difference of the offset, so position.x + dx == adjacent.x
    pub fn dx(&self) -> i32 {
        OFFSETS3[self.0].x
    }
    /// The y difference of the offset, so position.y + dy == adjacent.y
    pub fn dy(&self) -> i32 {
        OFFSETS3[self.0].y
    }
    /// The z difference of the offset, so position.z + dz == adjacent.z
    pub fn dz(&self) -> i32 {
        OFFSETS3[self.0].z
    }
}

const OFFSETS3: [Vector3<i32>; 6] = [
    Vector3::new(-1, 0, 0),
    Vector3::new(0, -1, 0),
    Vector3::new(0, 0, -1),
    Vector3::new(0, 0, 1),
    Vector3::new(0, 1, 0),
    Vector3::new(1, 0, 0),
];

impl From<Vector3Offset> for Vector3<i32> {
    fn from(value: Vector3Offset) -> Self {
        OFFSETS3[value.0]
    }
}

impl std::ops::Add<Vector3Offset> for Vector3<i32> {
    type Output = Vector3<i32>;

    fn add(self, rhs: Vector3Offset) -> Self::Output {
        self + OFFSETS3[rhs.0]
    }
}

impl std::ops::Neg for Vector3Offset {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vector3Offset(5 - self.0)
    }
}

/// A set of values of type `V` where each value is associated with a frequency
/// so that a value can be chosen from the set at random with the probability of each
/// value weighted by its frequency.
#[derive(Debug, Clone)]
pub struct ProbabilitySet<V> {
    total: f32,
    content: Vec<(f32, V)>,
}

impl<V> Default for ProbabilitySet<V> {
    fn default() -> Self {
        Self {
            total: 0.0,
            content: Vec::default(),
        }
    }
}

impl<V> ProbabilitySet<V> {
    /// Iterate through all the items in the set and their frequencies.
    pub fn iter(&self) -> impl Iterator<Item = (f32, &V)> {
        self.content.iter().map(|(f, v)| (*f, v))
    }
    /// The number of elements in the set.
    pub fn len(&self) -> usize {
        self.content.len()
    }
    /// True if the set has no elements.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
    /// Remove the content of the set.
    pub fn clear(&mut self) {
        self.total = 0.0;
        self.content.clear();
    }
    /// Add a value and give it a frequency.
    pub fn add(&mut self, frequency: f32, value: V) {
        if frequency > 0.0 {
            self.total += frequency;
            self.content.push((frequency, value));
        }
    }

    /// The sum of the frequencies of all the elements of the set.
    /// The probability of an element being chosen is the frequency of
    /// that element divided by this total.
    pub fn total_frequency(&self) -> f32 {
        self.total
    }

    /// The mean of the frequencies of all the elements of the set.
    pub fn average_frequency(&self) -> f32 {
        let size = self.content.len();
        if size == 0 {
            return 0.0;
        }
        self.total / (size as f32)
    }

    /// Choose a value from the set using the given random number generator.
    /// The probability of each element of the set being chosen is the frequency
    /// of that element divided by the sum of the frequencies of all elements.
    pub fn get_random<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&V> {
        if self.total <= 0.0 {
            return None;
        }
        let mut p = rng.gen_range(0.0..self.total);
        for (f, v) in self.iter() {
            if p < f {
                return Some(v);
            }
            p -= f;
        }
        self.iter().next().map(|v| v.1)
    }
}

/// For the purposes of autotiling, tiles are represented by patterns that
/// hold the data which determines whether two tiles are permitted to appear
/// adjacent to each other. Therefore autotile algorithms place patterns
/// rather than tiles, and translating those patterns into actual tiles
/// is a later step.
pub trait TilePattern {
    /// A pattern is capable of being adjacent to other patterns in several directions.
    /// The pattern's offset type is the type of the offset between a pattern's position
    /// and the position of an adjacent pattern.
    type Offset;
    /// A pattern may connect to another pattern by touching their corners.
    /// This creates a diagonal connection where the patterns are touching
    /// but not adjacent, and in some contexts there may be rules regarding
    /// which patterns may be diagonally connected to which other patterns.
    type Diagonal;
    /// True if this pattern may be adjacent the given other pattern of this type
    /// when that other pattern is positioned at `offset` relative to this pattern.
    fn is_legal(&self, offset: &Self::Offset, to: &Self) -> bool;
    /// True if this pattern may be diagonal to the given other pattern when
    /// that other pattern is positioned at `offset` relative to this pattern.
    fn is_legal_diagonal(&self, offset: &Self::Diagonal, to: &Self) -> bool;
}

/// `PatternBits` represents a tile's pattern as a 3x3 grid of `i8` values
/// for doing autotiling in 2D with `Vector2<i32>` positions.
/// The center of the 3x3 grid is called the pattern's *terrain.*
/// The 8 values around the outside of the grid are called the pattern's
/// *peering bits.*
///
/// The peering bits of this pattern determine whether it is permitted to use this
/// pattern adjacent to other patterns. A pattern is only legal if all three peering
/// bits along each edge match the bits on the nearest edge of the adjacent pattern.
///
/// In the case of diagonal connections, only only bit must match in each pattern:
/// the corner bit that is nearest to the other pattern.
///
/// For the details of how peering bits constrain where this pattern may be used,
/// see the implementation of [`TilePattern`].
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternBits(pub [i8; 9]);

impl PatternBits {
    /// The pattern's terrain ID that can be used to categorize this pattern,
    /// and to determine which peering bits are the most important for pattern sorting.
    /// Patterns that have more peering bits that match its terrain are given higher priority
    /// when sorting and appear earlier in the list.
    pub fn center(&self) -> i8 {
        self.0[bit_pos(1, 1)]
    }
    /// The number of bits in the pattern that match the terrain ID. This count is at most 9
    /// and at least 1 because the center bit always matches itself.
    pub fn center_terrain_count(&self) -> usize {
        let center = self.center();
        self.0.iter().filter(|t| **t == center).count()
    }
    /// The number of distinct distinct values in this pattern. This count is at most 9 and
    /// at least 1, for the case where every bit is the same.
    pub fn unique_terrain_count(&self) -> usize {
        let mut count = 1;
        'outer: for i in 1..9 {
            let current = self.0[i];
            for j in 0..i {
                if self.0[j] == current {
                    continue 'outer;
                }
            }
            count += 1;
        }
        count
    }
}

impl Debug for PatternBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn w(f: &mut std::fmt::Formatter<'_>, bs: &[i8]) -> std::fmt::Result {
            write!(f, "{0},{1},{2}", bs[0], bs[1], bs[2])
        }
        let bs = &self.0;
        f.write_str("[")?;
        w(f, &bs[0..3])?;
        f.write_str("|")?;
        w(f, &bs[3..6])?;
        f.write_str("|")?;
        w(f, &bs[6..9])?;
        f.write_str("]")
    }
}

impl PartialOrd for PatternBits {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PatternBits {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.center_terrain_count()
            .cmp(&other.center_terrain_count())
            .reverse()
            .then_with(|| {
                self.unique_terrain_count()
                    .cmp(&other.unique_terrain_count())
            })
            .then_with(|| self.0.cmp(&other.0))
    }
}

/// Converts an x,y position into index in 0..9. Both x and y must be within 0..3.
#[inline]
fn nine_position_to_index(position: Vector2<usize>) -> usize {
    if position.y > 2 || position.x > 2 {
        panic!("Illegal terrain bit position: {:?}", position);
    }
    bit_pos(position.x, position.y)
}

impl std::ops::Index<Vector2<usize>> for PatternBits {
    type Output = i8;

    fn index(&self, index: Vector2<usize>) -> &Self::Output {
        &self.0[nine_position_to_index(index)]
    }
}

impl std::ops::IndexMut<Vector2<usize>> for PatternBits {
    fn index_mut(&mut self, index: Vector2<usize>) -> &mut Self::Output {
        &mut self.0[nine_position_to_index(index)]
    }
}

impl TilePattern for PatternBits {
    type Offset = Vector2Offset;
    type Diagonal = Vector2Diagonal;
    fn is_legal(&self, offset: &Vector2Offset, to: &Self) -> bool {
        offset
            .peering_bits()
            .zip((-*offset).peering_bits())
            .all(|(a, b)| self.0[a] == to.0[b])
    }
    fn is_legal_diagonal(&self, diagonal: &Vector2Diagonal, to: &Self) -> bool {
        self.0[diagonal.peering_bit()] == to.0[(-*diagonal).peering_bit()]
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use rand::SeedableRng;

    use PatternBits as Bits;

    use super::*;

    fn make_rng(seed: u64) -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(seed)
    }

    #[test]
    fn probability_set_0() {
        let set = ProbabilitySet::<u32>::default();
        assert_eq!(set.get_random(&mut make_rng(0)), None);
    }
    #[test]
    fn probability_set_1() {
        let mut set = ProbabilitySet::<u32>::default();
        set.add(0.65, 27);
        let mut rng = make_rng(0);
        assert_eq!(set.get_random(&mut rng), Some(&27));
        assert_eq!(set.get_random(&mut rng), Some(&27));
        assert_eq!(set.get_random(&mut rng), Some(&27));
    }
    #[test]
    fn probability_set_2() {
        let mut set = ProbabilitySet::<u32>::default();
        set.add(0.5, 1);
        set.add(1.0, 2);
        let mut rng = make_rng(0);
        let mut results = FxHashMap::<Option<u32>, usize>::default();
        for _ in 0..75 {
            *results
                .entry(set.get_random(&mut rng).cloned())
                .or_default() += 1;
        }
        let result_1 = results.get(&Some(1)).copied().unwrap_or_default();
        let result_2 = results.get(&Some(2)).copied().unwrap_or_default();
        assert_eq!(results.get(&None).copied().unwrap_or_default(), 0, "None");
        assert!((21..27).contains(&result_1), "1: {}", result_1);
        assert!((45..55).contains(&result_2), "2: {}", result_2);
        assert_eq!(results.values().sum::<usize>(), 75, "Sum");
    }
    #[test]
    fn probability_set_3() {
        let mut set = ProbabilitySet::<u32>::default();
        set.add(2.0, 1);
        set.add(2.0, 2);
        set.add(2.0, 3);
        let mut rng = make_rng(0);
        let mut results = FxHashMap::<Option<u32>, usize>::default();
        for _ in 0..750 {
            *results
                .entry(set.get_random(&mut rng).cloned())
                .or_default() += 1;
        }
        let result_1 = results.get(&Some(1)).copied().unwrap_or_default();
        let result_2 = results.get(&Some(2)).copied().unwrap_or_default();
        let result_3 = results.get(&Some(2)).copied().unwrap_or_default();
        assert_eq!(results.get(&None).copied().unwrap_or_default(), 0, "None");
        assert!((210..270).contains(&result_1), "1: {}", result_1);
        assert!((210..270).contains(&result_2), "2: {}", result_2);
        assert!((210..270).contains(&result_3), "3: {}", result_3);
        assert_eq!(results.values().sum::<usize>(), 750, "Sum");
    }
    #[test]
    fn terrain_ord_1() {
        let a = Bits([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let b = Bits([1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(a.cmp(&b), Ordering::Less);
    }
    #[test]
    fn terrain_ord_2() {
        let a = Bits([0, 1, 0, 0, 0, 0, 0, 0, 0]);
        let b = Bits([0, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(a.cmp(&b), Ordering::Equal);
    }
    #[test]
    fn terrain_ord_3() {
        let a = Bits([0, 1, 0, 0, 0, 0, 0, 0, 0]);
        let b = Bits([1, 0, 1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(a.cmp(&b), Ordering::Less);
    }
    #[test]
    fn terrain_ord_4() {
        let a = Bits([0, 1, 0, 1, 0, 1, 2, 0, 0]);
        let b = Bits([1, 0, 1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(a.cmp(&b), Ordering::Greater);
    }
    #[test]
    fn terrain_ord_ne() {
        let a = Bits([0, 1, 0, 1, 0, 1, 0, 0, 0]);
        let b = Bits([1, 0, 1, 1, 0, 0, 0, 0, 0]);
        assert_ne!(a.cmp(&b), Ordering::Equal);
    }
    #[test]
    fn tile_pattern_up() {
        let a = Bits([-1, -2, -3, -4, -5, -6, 1, 2, 3]);
        let b = Bits([1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let offset = Vector2Offset::DOWN;
        assert!(b.is_legal(&offset, &a), "{b:?} {offset:?} {a:?}");
    }
    #[test]
    fn tile_pattern_down() {
        let a = Bits([-1, -2, -3, -4, -5, -6, 1, 2, 3]);
        let b = Bits([1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let offset = Vector2Offset::UP;
        assert!(a.is_legal(&offset, &b), "{b:?} {offset:?} {a:?}");
    }
    #[test]
    fn tile_pattern_right() {
        let a = Bits([1, 2, 1, 4, 5, 2, 7, 8, 3]);
        let b = Bits([1, -2, -3, 2, -5, -6, 3, -2, -3]);
        let offset = Vector2Offset::RIGHT;
        assert!(a.is_legal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_right2() {
        let a = Bits([3, 3, 0, 3, 3, 0, 3, 3, 0]);
        let b = Bits([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let offset = Vector2Offset::RIGHT;
        assert!(a.is_legal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_left() {
        let a = Bits([1, 2, 3, 2, 3, 4, 3, 8, 5]);
        let b = Bits([-1, -2, 1, -2, -5, 2, -3, -2, 3]);
        let offset = Vector2Offset::LEFT;
        assert!(a.is_legal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_left2() {
        let a = Bits([0, 3, 3, 0, 3, 3, 0, 3, 3]);
        let b = Bits([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let offset = Vector2Offset::LEFT;
        assert!(a.is_legal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_left_up() {
        let a = Bits([1, 2, 1, 2, 1, 4, 3, 8, 5]);
        let b = Bits([-1, -2, 3, -2, -5, -2, -3, -2, -1]);
        let offset = Vector2Diagonal::LEFT_UP;
        assert!(a.is_legal_diagonal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_right_up() {
        let a = Bits([1, 2, 1, 2, 1, 4, 1, 8, 3]);
        let b = Bits([3, -2, -1, -2, -5, -2, -3, -2, -1]);
        let offset = Vector2Diagonal::RIGHT_UP;
        assert!(a.is_legal_diagonal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_left_down() {
        let a = Bits([3, 2, 1, 2, 1, 4, 1, 8, 5]);
        let b = Bits([-1, -2, -3, -2, -5, -2, -3, -2, 3]);
        let offset = Vector2Diagonal::LEFT_DOWN;
        assert!(a.is_legal_diagonal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
    #[test]
    fn tile_pattern_right_down() {
        let a = Bits([1, 2, 3, 2, 1, 4, 1, 8, 1]);
        let b = Bits([-3, -2, -1, -2, -5, -2, 3, -2, -1]);
        let offset = Vector2Diagonal::RIGHT_DOWN;
        assert!(a.is_legal_diagonal(&offset, &b), "{a:?} {offset:?} {b:?}");
    }
}
