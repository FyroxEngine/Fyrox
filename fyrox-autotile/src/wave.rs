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

//! Wave function collapse algorithm based upon [fast-wfc](https://github.com/math-fehr/fast-wfc).

use std::{
    error::Error,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use super::*;

type Entropy = f64;

/// A `WavePosition` is like an `OffsetPosition` but it also has a counter type
/// that the wave propagator can use to keep count of how many possible matching
/// patterns are available from each `Offset`.
pub trait WavePosition: OffsetPosition {
    /// The type that holds count of patterns in each direction.
    type Counter: WaveOffsetCounter<Offset = Self::Offset>;
}

impl WavePosition for Vector2<i32> {
    type Counter = Vector2OffsetCounter;
}

/// A trait for counting types that store a `usize` for each of the offsets from
/// some `OffsetPosition` type. During the wave function collapse process, these
/// counts can be reduced as adjacent patterns possibilities are removed so
/// that the algorithm can keep track of how many ways a pattern is possible.
/// For a pattern to be possible, it must match some possible pattern in all
/// offsets, so if any of these counts become 0 the algorithm will know to
/// remove the corresponding pattern.
pub trait WaveOffsetCounter: Default + Clone {
    /// The type that will be the index into the array of `usize` counts.
    type Offset;
    /// The count for the given offset.
    fn count(&self, offset: &Self::Offset) -> usize;
    /// The count for the given offset.
    fn count_mut(&mut self, offset: &Self::Offset) -> &mut usize;
}

/// A [`WaveOffsetCounter`] for `Vector3` offsets, containing 4 `usize` values,
/// one for each adjacent cell of a pattern in 2D.
#[derive(Debug, Clone, Default)]
pub struct Vector2OffsetCounter([usize; 4]);

impl WaveOffsetCounter for Vector2OffsetCounter {
    type Offset = Vector2Offset;

    fn count(&self, offset: &Vector2Offset) -> usize {
        self.0[offset.0]
    }

    fn count_mut(&mut self, offset: &Vector2Offset) -> &mut usize {
        &mut self.0[offset.0]
    }
}

impl WavePosition for Vector3<i32> {
    type Counter = Vector3OffsetCounter;
}

/// A [`WaveOffsetCounter`] for `Vector3` offsets, containing 6 `usize` values,
/// one for each adjacent cell of a pattern in 3D.
#[derive(Debug, Clone, Default)]
pub struct Vector3OffsetCounter([usize; 6]);

impl WaveOffsetCounter for Vector3OffsetCounter {
    type Offset = Vector3Offset;

    fn count(&self, offset: &Vector3Offset) -> usize {
        self.0[offset.0]
    }

    fn count_mut(&mut self, offset: &Vector3Offset) -> &mut usize {
        &mut self.0[offset.0]
    }
}

/// Trait for an object that specifies the rules by which wave function collapse
/// tiles are chosen.
pub trait WfcConstrain {
    /// The offset type that is used to specify the ways in which one pattern
    /// may be adjacent to another pattern.
    type Offset;
    /// The type of patterns that wave function collapse will choose.
    type Pattern: Eq + Hash + Clone;
    /// Iterator for all the patterns that wave function collapse is allowed to choose.
    /// Note that any pattern with a probability of 0.0 must not be included.
    fn all_patterns(&self) -> impl Iterator<Item = &Self::Pattern>;
    /// The probability of choosing a given pattern when there are no restrictions on patterns
    /// in any of the adjacent cells. This should be between 0.0 and 1.0.
    fn probability_of(&self, pattern: &Self::Pattern) -> f32;
    /// True if the `from` pattern may be chosen when the `to` pattern has already been chosen
    /// for the cell at position `offset` relative to `from`.
    fn is_legal(&self, from: &Self::Pattern, offset: &Self::Offset, to: &Self::Pattern) -> bool;
}

/// An implementation for `WfcConstrain` that stores the information for each pattern
/// in a hash table and also keeps track of a which values each pattern may represent,
/// so that patterns may be translated into tiles after wave function collapse is complete.
pub struct HashWfcConstraint<Pat, V> {
    pattern_map: FxHashMap<Pat, WfcPatternConstraint<V>>,
}

/// The data for a pattern, with probability of the pattern
/// and probability of each tile that matches the pattern.
struct WfcPatternConstraint<V> {
    /// The probability of the pattern being chosen, between 0.0 and 1.0.
    probability: f32,
    /// The set of values that may be randomly chosen to represent the pattern
    /// when patterns are converted into tiles.
    value_set: ProbabilitySet<V>,
}

impl<V> Default for WfcPatternConstraint<V> {
    fn default() -> Self {
        Self {
            probability: 0.0,
            value_set: ProbabilitySet::default(),
        }
    }
}

impl<Pat, V> Default for HashWfcConstraint<Pat, V> {
    fn default() -> Self {
        Self {
            pattern_map: FxHashMap::default(),
        }
    }
}

impl<Pat, V> HashWfcConstraint<Pat, V>
where
    Pat: Eq + Hash,
{
    /// True if this constraint contains no patterns.
    pub fn is_empty(&self) -> bool {
        self.pattern_map.is_empty()
    }
    /// Remove all data, reseting this object to empty so it is ready
    /// to be reused with new pattern data.
    pub fn clear(&mut self) {
        self.pattern_map.clear();
    }
    /// Add a new value to the data with the given pattern and frequency.
    /// The frequency does not need to be between 0.0 and 1.0.
    /// Frequencies of 0.0 or less will be silently ignored, and
    /// frequencies will be automatically normalized into probabilities
    /// when [`finalize`](Self::finalize) is called.
    pub fn add(&mut self, pattern: Pat, frequency: f32, value: V)
    where
        Pat: Debug,
        V: Debug,
    {
        if frequency <= 0.0 {
            return;
        }
        self.pattern_map
            .entry(pattern)
            .or_default()
            .value_set
            .add(frequency, value);
    }
    /// Calculate the probability of each pattern based on the frequencies
    /// of all values that were added with [`add`](Self::add).
    /// The sum of the frequencies is calculated and then each frequency is
    /// divided by the sum to normalize the frequencies into probabilities between
    /// 0.0 and 1.0.
    pub fn finalize(&mut self) {
        let sum: f32 = self
            .pattern_map
            .values()
            .map(|v| v.value_set.total_frequency())
            .sum();
        if sum > 0.0 {
            for v in self.pattern_map.values_mut() {
                v.probability = v.value_set.total_frequency() / sum;
            }
        }
    }
    /// Calculate the probability of each pattern based on the frequencies
    /// of all values that were added with [`add`](Self::add).
    /// Divide the frequency of each pattern by the total number of patterns
    /// in that pattern's terrain so that terrains with many patterns are not
    /// given an advantage over terrains with few patterns.
    /// The `terrain` function is used to determine the terrain of each pattern.
    /// The sum of the frequencies is calculated and then each frequency is
    /// divided by the sum to normalize the frequencies into probabilities between
    /// 0.0 and 1.0.
    pub fn finalize_with_terrain_normalization<Ter, F>(&mut self, terrain: F)
    where
        Ter: Hash + Eq,
        F: Fn(&Pat) -> Ter,
    {
        let mut terrain_count = FxHashMap::<Ter, usize>::default();
        for p in self.pattern_map.keys() {
            *terrain_count.entry(terrain(p)).or_default() += 1;
        }
        let sum: f32 = self
            .pattern_map
            .iter()
            .map(|(p, v)| {
                let count = terrain_count.get(&terrain(p)).copied().unwrap_or_default() as f32;
                if count > 0.0 {
                    v.value_set.total_frequency() / count
                } else {
                    1.0
                }
            })
            .sum();
        if sum > 0.0 {
            for v in self.pattern_map.values_mut() {
                v.probability = v.value_set.total_frequency() / sum;
            }
        }
    }
    /// A random value that matches the given pattern, based upon previous calls to
    /// [`add`](Self::add). Often there may be more than one tile that matches a particular
    /// pattern, and so this method helps with the final step after wave function collapse:
    /// converting the chosen patterns into actual tiles.
    pub fn get_random<R: Rng + ?Sized>(&self, rng: &mut R, pattern: &Pat) -> Option<&V> {
        self.pattern_map.get(pattern)?.value_set.get_random(rng)
    }
}

impl<Pat, V> WfcConstrain for HashWfcConstraint<Pat, V>
where
    Pat: Eq + Hash + Clone + TilePattern,
{
    type Offset = Pat::Offset;
    type Pattern = Pat;

    fn all_patterns(&self) -> impl Iterator<Item = &Self::Pattern> {
        self.pattern_map.keys()
    }

    fn probability_of(&self, pattern: &Self::Pattern) -> f32 {
        self.pattern_map
            .get(pattern)
            .map(|s| s.probability)
            .unwrap_or_default()
    }

    fn is_legal(&self, from: &Self::Pattern, offset: &Self::Offset, to: &Self::Pattern) -> bool {
        from.is_legal(offset, to)
    }
}

#[derive(Debug, Clone)]
struct Wave<Pos: WavePosition, Pat>(FxHashMap<Pos, WaveCell<Pos::Counter, Pat>>);

impl<K: WavePosition, P> Default for Wave<K, P> {
    fn default() -> Self {
        Self(FxHashMap::default())
    }
}

impl<Pos: WavePosition, Pat> Deref for Wave<Pos, Pat> {
    type Target = FxHashMap<Pos, WaveCell<Pos::Counter, Pat>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Pos: WavePosition, Pat> DerefMut for Wave<Pos, Pat> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
struct WaveCell<Count, Pat> {
    /// The possible patterns, each with a count of the possible patterns in other
    /// cells that make this pattern possible.
    pattern_possibilities: FxHashMap<Pat, Count>,
    /// The sum of p(P) * log(p(P)) for all possible patterns P in the cell.
    plogp_sum: Entropy,
    /// The sum of p(P) for all possible patterns P in the cell.
    sum: Entropy,
    /// The log of `sum`.
    log_sum: Entropy,
    /// The entropy of the cell
    entropy: Entropy,
}

impl<Count, Pat> Default for WaveCell<Count, Pat> {
    fn default() -> Self {
        Self {
            pattern_possibilities: FxHashMap::default(),
            plogp_sum: 0.0,
            sum: 0.0,
            log_sum: 0.0,
            entropy: 0.0,
        }
    }
}

impl<Count, Pat> WaveCell<Count, Pat> {
    pub fn single_pattern(&self) -> Option<&Pat> {
        let mut keys = self.pattern_possibilities.keys();
        let pat = keys.next();
        if keys.next().is_none() {
            pat
        } else {
            None
        }
    }
}

/// Statistical information derived from a [`WfcConstrain`] object.
/// A  `WaveLimits` object is only valid for a particular `WfcConstrain` object
/// so long as the output of the `WfcConstrain` object's methods do not change.
#[derive(Debug, Clone)]
struct WaveLimits<Count, Pat> {
    max_cell: WaveCell<Count, Pat>,
    plogp_map: FxHashMap<Pat, Entropy>,
    maximum_noise: Entropy,
}

impl<Count, Pat> Default for WaveLimits<Count, Pat> {
    fn default() -> Self {
        Self {
            max_cell: WaveCell::default(),
            plogp_map: FxHashMap::default(),
            maximum_noise: 0.0,
        }
    }
}

impl<Count: WaveOffsetCounter, Pat: Clone + Hash + Eq> WaveLimits<Count, Pat> {
    /// Use the data in a [`WfcConstrain`] object to initialize the limits
    /// for a new wave function collapse using the given constraint.
    pub fn fill_from<
        Pos: WavePosition<Counter = Count, Offset = Count::Offset>,
        Con: WfcConstrain<Pattern = Pat, Offset = Count::Offset>,
    >(
        &mut self,
        constraint: &Con,
    ) {
        self.plogp_map.clear();
        let mut plogp_sum = 0.0;
        let mut sum = 0.0;
        let mut min_abs_plogp = Entropy::INFINITY;
        self.max_cell.pattern_possibilities.clear();
        for pattern in constraint.all_patterns() {
            let p = constraint.probability_of(pattern) as Entropy;
            let plogp = p * p.ln();
            self.plogp_map.insert(pattern.clone(), plogp);
            let abs_plogp = plogp.abs();
            if abs_plogp < min_abs_plogp {
                min_abs_plogp = abs_plogp;
            }
            plogp_sum += plogp;
            sum += p;
            let mut count = Count::default();
            for offset in Pos::all_offsets() {
                *count.count_mut(&offset) = constraint
                    .all_patterns()
                    .filter(|p| constraint.is_legal(p, &offset, pattern))
                    .count();
            }
            self.max_cell
                .pattern_possibilities
                .insert(pattern.clone(), count);
        }
        let log_sum = sum.log10();
        self.max_cell.plogp_sum = plogp_sum;
        self.max_cell.sum = sum;
        self.max_cell.log_sum = log_sum;
        self.max_cell.entropy = log_sum - plogp_sum / sum;
        self.maximum_noise = min_abs_plogp / 2.0;
    }
}

/// Wave function collapse is a multi-step that may require a significant amount of time,
/// depending on how may cells need to be filled. `WfcControlFlow` allows wave propagation
/// methods to specify in their return value whether they have completed their task or whether
/// further work is required. This allows the user to spread the wave function collapse
/// across multiple method calls so it can be mixed with other work or even aborted.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WfcControlFlow {
    /// More work is required.
    Continue,
    /// The method has finished whatever it was trying to do.
    Finish,
}

/// Wave function collapse failed to find a solution. Due to the randomization of the algorithm,
/// a failed attempt does not necessarily indicate a problem, but may instead be due to chance.
/// Another attempt my succeed where a previous one failed.
#[derive(Debug)]
pub struct WfcFailure;

impl Error for WfcFailure {}

impl Display for WfcFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Wave function collapse failed to find a solution.")
    }
}

/// The propagator is responsible for the wave function collapse algorithm and
/// it contains all the necessary data except for the [`WfcConstrain`] object
/// that contains the rules for which patterns may be chosen.
///
/// The contraint object is not owned by the propagator because the wave function
/// collapse algorithm may be spread out across many steps and it may be necessary
/// for the constraint object to not live as long as the propagator, and be
/// reconstructed at each step.
///
/// Every constraint object at each step should always fully agree with the constraint
/// objects at all previous steps, or else the algorithm may produce undesired results.
#[derive(Debug, Default, Clone)]
pub struct WfcPropagator<Pos: WavePosition, Pat> {
    limits: WaveLimits<Pos::Counter, Pat>,
    wave: Wave<Pos, Pat>,
    propagating: Vec<(Pos, Pat)>,
    pending: Vec<(Pos, Pat)>,
    backtrack_cells: Vec<WaveCell<Pos::Counter, Pat>>,
    backtrack_map: FxHashMap<Pos, WaveCell<Pos::Counter, Pat>>,
}

impl<Pos: WavePosition + Debug, Pat: Clone + Hash + Eq + Debug> WfcPropagator<Pos, Pat> {
    /// This propagator contains no cells.
    pub fn is_empty(&self) -> bool {
        self.wave.is_empty()
    }
    /// An iterator over the positions of every cell of the wave.
    pub fn positions(&self) -> impl Iterator<Item = &Pos> {
        self.wave.keys()
    }
    /// True if the wave has a cell at the given position.
    pub fn contains_cell(&self, position: &Pos) -> bool {
        self.wave.contains_key(position)
    }
    /// Iterator over the cells that have been given a pattern by the wave function collapse.
    pub fn assigned_patterns(&self) -> impl Iterator<Item = (&Pos, &Pat)> {
        self.wave
            .iter()
            .filter_map(|(p, c)| Some((p, c.single_pattern()?)))
    }
    /// Use the data in a [`WfcConstrain`] object to initialize the propagator
    /// for a new wave function collapse using the given constraint.
    /// After calling this method, the next step is to call [`add_cell`](Self::add_cell) for each
    /// cell that needs to be filled by wave function collapse.
    pub fn fill_from<Con: WfcConstrain<Pattern = Pat, Offset = <Pos as OffsetPosition>::Offset>>(
        &mut self,
        constraint: &Con,
    ) {
        self.limits.fill_from::<Pos, Con>(constraint);
        self.wave.clear();
        self.propagating.clear();
    }
    /// Create a new cell at the given position, assuming that wave function collapse has not yet begun
    /// and all the surrounding cells have no restrictions.
    /// Calling this after [`restrict_edge`](Self::restrict_edge), [`observe_random_cell`](Self::observe_random_cell),
    /// or [`observe_all`](Self::observe_all) may produce undesirable consequences.
    /// After all the cells have been added, the next step is to optionally call `restrict_edge` to constrain which patterns
    /// the may be put into the border cells.
    pub fn add_cell(&mut self, position: Pos) {
        let _ = self.wave.insert(position, self.limits.max_cell.clone());
    }
    fn save_cell(&mut self, position: Pos) {
        if let Entry::Vacant(entry) = self.backtrack_map.entry(position.clone()) {
            let Some(cell) = self.wave.get(&position) else {
                return;
            };
            let mut backtrack_cell = self.backtrack_cells.pop().unwrap_or_default();
            backtrack_cell.clone_from(cell);
            entry.insert(backtrack_cell);
        }
    }
    fn clear_backtrack(&mut self) {
        for (_, backtrack_cell) in self.backtrack_map.drain() {
            self.backtrack_cells.push(backtrack_cell);
        }
    }
    fn backtrack(&mut self) {
        for (pos, backtrack_cell) in self.backtrack_map.drain() {
            let Some(cell) = self.wave.get_mut(&pos) else {
                continue;
            };
            cell.clone_from(&backtrack_cell);
            self.backtrack_cells.push(backtrack_cell);
        }
    }
    /// Randomly choose a low-entropy cell. This is used by [`observe_random_cell`](Self::observe_random_cell)
    /// to decide which cell will be observed.
    pub fn find_min_entropy<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<Pos>
    where
        Pos::Counter: Debug,
    {
        let mut min_pos = None;
        let mut min_entropy = Entropy::INFINITY;
        for (position, cell) in self.wave.iter() {
            if cell.pattern_possibilities.len() <= 1 || cell.entropy >= min_entropy {
                continue;
            }
            let noise = rng.gen_range(0.0..=self.limits.maximum_noise);
            let entropy = cell.entropy + noise;
            if entropy < min_entropy {
                min_entropy = entropy;
                min_pos = Some(position.clone());
            }
        }
        min_pos
    }
    /// Choose a random pattern from among the potential patterns for the given cell based
    /// on probabilities given in `constraint`. This is called by
    /// [`observe_random_cell`](Self::observe_random_cell) to decide which pattern to assign.
    pub fn choose_random_pattern<R, Con>(
        &self,
        position: &Pos,
        rng: &mut R,
        constraint: &Con,
    ) -> Option<Pat>
    where
        R: Rng + ?Sized,
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        let cell = self.wave.get(position)?;
        let mut target = rng.gen_range(0.0..cell.sum);
        for pattern in cell.pattern_possibilities.keys() {
            let p = constraint.probability_of(pattern) as Entropy;
            target -= p;
            if target <= 0.0 {
                return Some(pattern.clone());
            }
        }
        cell.pattern_possibilities.keys().next().cloned()
    }
    /// Constrain the cells around the given position based on the assumption that the given
    /// position has the given pattern. Calling this method twice on the same position may put
    /// the surrounding cells into an invalid state, even if both calls have the same pattern.
    /// This should be called after all cells have been added using [`add_cell`](Self::add_cell),
    /// and there should be no cell at the given position.
    ///
    /// The next step is to call [`observe_random_cell`](Self::observe_random_cell) or
    /// [`observe_all`](Self::observe_all) to begin collapsing the wave function.
    pub fn restrict_edge<Con>(
        &mut self,
        position: &Pos,
        pattern: &Pat,
        constraint: &Con,
    ) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
        Pos: Debug,
    {
        for offset in Pos::all_offsets() {
            let other_pos = position.clone() + offset.clone();
            let Some(other_cell) = self.wave.get_mut(&other_pos) else {
                continue;
            };
            other_cell
                .pattern_possibilities
                .retain(|other_pattern, _counter| {
                    if !constraint.is_legal(pattern, &offset, other_pattern) {
                        self.pending
                            .push((other_pos.clone(), other_pattern.clone()));
                        false
                    } else {
                        true
                    }
                });
            if other_cell.pattern_possibilities.is_empty() {
                return Err(WfcFailure);
            }
        }
        Ok(())
    }
    fn set_cell<Con>(
        &mut self,
        position: &Pos,
        pattern: &Pat,
        constraint: &Con,
    ) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        let cell = self.wave.get_mut(position).expect("Missing wave cell");
        let mut possibilities = std::mem::take(&mut cell.pattern_possibilities);
        let (pattern, count) = possibilities
            .remove_entry(pattern)
            .expect("Missing pattern");
        for (p, _) in possibilities.drain() {
            self.after_restrict(position, &p, constraint)?;
        }
        possibilities.insert(pattern, count);
        let cell = self.wave.get_mut(position).unwrap();
        cell.pattern_possibilities = possibilities;
        self.verify(position, constraint)
    }
    /// Completely collapse the wave function. This method repeatedly observes a random cell and
    /// propagates each observation until all cells have been observed or the wave function collapse
    /// fails due to not being able to find a valid pattern for some cell.
    ///
    /// If the number of cells is large, this method may be slow. Use [`observe_random_cell`](Self::observe_random_cell)
    /// to progress the collapse one cell at a time and thereby have more control over the process
    /// and allow the possibility of aborting.
    pub fn observe_all<R, Con>(&mut self, rng: &mut R, constraint: &Con) -> Result<(), WfcFailure>
    where
        R: Rng + ?Sized,
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
        Pos: Debug,
        Pos::Counter: Debug,
    {
        while self.observe_random_cell(rng, constraint)? == WfcControlFlow::Continue {}
        Ok(())
    }
    /// Observing a cell means choosing a random pattern for that cell from all the potential patterns
    /// for that particular cell. Each cell keeps its own independent list of possible patterns,
    /// and after a cell has been observed the possibilities for the surrounding cells may need to be
    /// restricted. The restriction of the surrounding cells is called *propagation* and it should be
    /// performed after each observation by calling [`propagate`](Self::propagate) or
    /// [`propagate_until_finished`](Self::propagate_until_finished).
    ///
    /// If propagation is not complete when this method is called, then `propagate_until_finished` is
    /// automatically called before the cell is observed.
    ///
    /// [`WfcControlFlow::Continue`] is returned if a cell was successfully observed, meaning that propagation
    /// and more observations may be required to complete the collapse.
    /// [`WfcControlFlow::Finish`] is returned if no cell could be observed because the pattern for all
    /// cells has already been determined and the wave function collapse is complete.
    pub fn observe_random_cell<R, Con>(
        &mut self,
        rng: &mut R,
        constraint: &Con,
    ) -> Result<WfcControlFlow, WfcFailure>
    where
        R: Rng + ?Sized,
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
        Pos: Debug,
        Pos::Counter: Debug,
    {
        self.propagate_until_finished(constraint)?;
        let Some(position) = self.find_min_entropy(rng) else {
            return Ok(WfcControlFlow::Finish);
        };
        let pattern = self
            .choose_random_pattern(&position, rng, constraint)
            .unwrap();
        self.save_cell(position.clone());
        for offset in Pos::all_offsets() {
            let p = position.clone() + offset;
            self.save_cell(p);
        }
        match self.set_cell(&position, &pattern, constraint) {
            Ok(()) => {
                self.clear_backtrack();
                self.propagating.append(&mut self.pending);
            }
            Err(_) => {
                self.backtrack();
                self.pending.clear();
                self.restrict(&position, &pattern, constraint)?;
                self.propagating.append(&mut self.pending);
            }
        }
        Ok(WfcControlFlow::Continue)
    }
    /// Repeatedly call [`propagate`](Self::propagate) until it returns [`WfcControlFlow::Finish`],
    /// thus ensuring that all the cells are prepared for the next observation. This is called
    /// automatically by [`observe_random_cell`](Self::observe_random_cell).
    pub fn propagate_until_finished<Con>(&mut self, constraint: &Con) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        while self.propagate(constraint)? == WfcControlFlow::Continue {}
        Ok(())
    }
    /// Propagate the restrictions from the most recent calls to [`observe_random_cell`](Self::observe_random_cell)
    /// or [`restrict_edge`](Self::restrict_edge) by one step, so that appropriate restrictions are spread across
    /// the cells of the wave. This method should be repeatedly called until it returns [`WfcControlFlow::Finish`]
    /// before another cell is observed so that the observation is based upon an accurate list of possible patterns
    /// for the cell.
    pub fn propagate<Con>(&mut self, constraint: &Con) -> Result<WfcControlFlow, WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        let Some((position, pattern)) = self.propagating.pop() else {
            return Ok(WfcControlFlow::Finish);
        };
        self.restrict(&position, &pattern, constraint)?;
        self.propagating.append(&mut self.pending);
        Ok(WfcControlFlow::Continue)
    }
    /// The given pattern is no longer valid at the given position. Update the surrouding cells
    /// to reflect this restriction and if any of the surrounding cells need to be restricted,
    /// add that restriction to the propagation list for future consideration.
    /// Return false if the restriction was impossible due to leaving the cell with no remaining
    /// possibilities.
    fn restrict<Con>(
        &mut self,
        position: &Pos,
        pattern: &Pat,
        constraint: &Con,
    ) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        let Some(cell) = self.wave.get_mut(position) else {
            return Ok(());
        };
        match cell.pattern_possibilities.entry(pattern.clone()) {
            Entry::Occupied(entry) => drop(entry.remove()),
            Entry::Vacant(_) => return Ok(()),
        }
        match cell.pattern_possibilities.len() {
            0 => return Err(WfcFailure),
            1 => self.verify(position, constraint)?,
            _ => (),
        }
        let Some(cell) = self.wave.get_mut(position) else {
            return Ok(());
        };
        if let Some(plogp) = self.limits.plogp_map.get(pattern) {
            let p = constraint.probability_of(pattern) as Entropy;
            cell.plogp_sum -= plogp;
            cell.sum -= p;
            cell.log_sum = cell.sum.ln();
            cell.entropy = cell.log_sum - cell.plogp_sum / cell.sum;
            self.after_restrict(position, pattern, constraint)
        } else {
            Ok(())
        }
    }
    fn after_restrict<Con>(
        &mut self,
        position: &Pos,
        pattern: &Pat,
        constraint: &Con,
    ) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
        Pos: Debug,
    {
        for offset in Pos::all_offsets() {
            let other_pos = position.clone() + offset.clone();
            let Some(other_cell) = self.wave.get_mut(&other_pos) else {
                continue;
            };
            other_cell
                .pattern_possibilities
                .retain(|other_pattern, counter| {
                    if constraint.is_legal(pattern, &offset, other_pattern) {
                        let c = counter.count_mut(&offset);
                        *c -= 1;
                        if *c == 0 {
                            self.pending
                                .push((other_pos.clone(), other_pattern.clone()));
                        }
                        *c > 0
                    } else {
                        true
                    }
                });
            if other_cell.pattern_possibilities.is_empty() {
                return Err(WfcFailure);
            }
        }
        Ok(())
    }
    fn verify<Con>(&self, position: &Pos, constraint: &Con) -> Result<(), WfcFailure>
    where
        Con: WfcConstrain<Pattern = Pat, Offset = Pos::Offset>,
    {
        let Some(cell) = self.wave.get(position) else {
            return Ok(());
        };
        let Some(pat) = cell.single_pattern() else {
            return Ok(());
        };
        for offset in Pos::all_offsets() {
            let other_pos = position.clone() + offset.clone();
            let Some(other_cell) = self.wave.get(&other_pos) else {
                continue;
            };
            if !other_cell
                .pattern_possibilities
                .keys()
                .any(|p| constraint.is_legal(pat, &offset, p))
            {
                return Err(WfcFailure);
            }
        }
        Ok(())
    }
}
