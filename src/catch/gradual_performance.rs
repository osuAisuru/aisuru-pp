use crate::{Beatmap, CatchPP};

use super::{CatchGradualDifficultyAttributes, CatchPerformanceAttributes};

/// Aggregation for a score's current state i.e. what was the
/// maximum combo so far and what are the current hitresults.
///
/// This struct is used for [`CatchGradualPerformanceAttributes`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CatchScoreState {
    /// Maximum combo that the score has had so far.
    /// **Not** the maximum possible combo of the map so far.
    ///
    /// Note that only fruits and droplets are considered for osu!catch combo.
    pub max_combo: usize,
    /// Amount of current fruits (300s).
    pub n_fruits: usize,
    /// Amount of current droplets (100s).
    pub n_droplets: usize,
    /// Amount of current tiny droplets (50s).
    pub n_tiny_droplets: usize,
    /// Amount of current tiny droplet misses (katus).
    pub n_tiny_droplet_misses: usize,
    /// Amount of current misses (fruits and droplets).
    pub misses: usize,
}

impl CatchScoreState {
    /// Create a new empty score state.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Gradually calculate the performance attributes of an osu!catch map.
///
/// After each hit object you can call
/// [`process_next_object`](`CatchGradualPerformanceAttributes::process_next_object`)
/// and it will return the resulting current [`CatchPerformanceAttributes`].
/// To process multiple objects at once, use
/// [`process_next_n_objects`](`CatchGradualPerformanceAttributes::process_next_n_objects`) instead.
///
/// Both methods require a [`CatchScoreState`] that contains the current
/// hitresults as well as the maximum combo so far.
///
/// Note that neither hits nor misses of tiny droplets require
/// to be processed. Only fruits and droplets do.
///
/// If you only want to calculate difficulty attributes use
/// [`CatchGradualDifficultyAttributes`](crate::catch::CatchGradualDifficultyAttributes) instead.
///
/// # Example
///
/// ```
/// use rosu_pp::{Beatmap, catch::{CatchGradualPerformanceAttributes, CatchScoreState}};
///
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
///
/// let mods = 64; // DT
/// let mut gradual_perf = CatchGradualPerformanceAttributes::new(&map, mods);
/// let mut state = CatchScoreState::new(); // empty state, everything is on 0.
///
/// // The first 10 hitresults are only fruits
/// for _ in 0..10 {
///     state.n_fruits += 1;
///     state.max_combo += 1;
///
///     # /*
///     let performance = gradual_perf.process_next_object(state.clone()).unwrap();
///     println!("PP: {}", performance.pp);
///     # */
///     # let _ = gradual_perf.process_next_object(state.clone());
/// }
///
/// // Then comes a miss.
/// // Note that state's max combo won't be incremented for
/// // the next few objects because the combo is reset.
/// state.misses += 1;
/// # /*
/// let performance = gradual_perf.process_next_object(state.clone()).unwrap();
/// println!("PP: {}", performance.pp);
/// # */
/// # let _ = gradual_perf.process_next_object(state.clone());
///
/// // The next 10 objects will be a mixture of fruits and droplets.
/// // Notice how tiny droplets from sliders do not count as hit objects
/// // that require processing. Only fruits and droplets do.
/// // Also notice how all 10 objects will be processed in one go.
/// state.n_fruits += 4;
/// state.n_droplets += 6;
/// state.n_tiny_droplets += 12;
/// # /*
/// let performance = gradual_perf.process_next_n_objects(state.clone(), 10).unwrap();
/// println!("PP: {}", performance.pp);
/// # */
/// # let _ = gradual_perf.process_next_n_objects(state.clone(), 10);
///
/// // Now comes another fruit. Note that the max combo gets incremented again.
/// state.n_fruits += 1;
/// state.max_combo += 1;
/// # /*
/// let performance = gradual_perf.process_next_object(state.clone()).unwrap();
/// println!("PP: {}", performance.pp);
/// # */
/// # let _ = gradual_perf.process_next_object(state.clone());
///
/// // Skip to the end
/// # /*
/// state.max_combo = ...
/// state.n_fruits = ...
/// state.n_droplets = ...
/// state.n_tiny_droplets = ...
/// state.n_tiny_droplet_misses = ...
/// state.misses = ...
/// let final_performance = gradual_perf.process_next_n_objects(state.clone(), usize::MAX).unwrap();
/// println!("PP: {}", performance.pp);
/// # */
/// # let _ = gradual_perf.process_next_n_objects(state.clone(), usize::MAX);
///
/// // Once the final performance was calculated,
/// // attempting to process further objects will return `None`.
/// assert!(gradual_perf.process_next_object(state).is_none());
/// ```
#[derive(Clone, Debug)]
pub struct CatchGradualPerformanceAttributes<'map> {
    difficulty: CatchGradualDifficultyAttributes<'map>,
    performance: CatchPP<'map>,
}

impl<'map> CatchGradualPerformanceAttributes<'map> {
    /// Create a new gradual performance calculator for osu!standard maps.
    pub fn new(map: &'map Beatmap, mods: u32) -> Self {
        let difficulty = CatchGradualDifficultyAttributes::new(map, mods);
        let performance = CatchPP::new(map).mods(mods).passed_objects(0);

        Self {
            difficulty,
            performance,
        }
    }

    /// Process the next hit object and calculate the
    /// performance attributes for the resulting score state.
    ///
    /// Note that neither hits nor misses of tiny droplets require
    /// to be processed. Only fruits and droplets do.
    pub fn process_next_object(
        &mut self,
        state: CatchScoreState,
    ) -> Option<CatchPerformanceAttributes> {
        self.process_next_n_objects(state, 1)
    }

    /// Same as [`process_next_object`](`CatchGradualPerformanceAttributes::process_next_object`)
    /// but instead of processing only one object it process `n` many.
    ///
    /// If `n` is 0 it will be considered as 1.
    /// If there are still objects to be processed but `n` is larger than the amount
    /// of remaining objects, `n` will be considered as the amount of remaining objects.
    pub fn process_next_n_objects(
        &mut self,
        state: CatchScoreState,
        n: usize,
    ) -> Option<CatchPerformanceAttributes> {
        let mut difficulty = None;

        for _ in 0..n.max(1) {
            match self.difficulty.next() {
                Some(attrs) => difficulty = Some(attrs),
                None => break,
            }
        }

        let difficulty = difficulty?;

        let performance = self
            .performance
            .clone()
            .attributes(difficulty)
            .state(state)
            .passed_objects(self.difficulty.idx)
            .calculate();

        Some(performance)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(not(any(feature = "async_tokio", feature = "async_std")))]
    #[test]
    fn correct_empty() {
        let map = Beatmap::from_path("./maps/2118524.osu").expect("failed to parse map");
        let mods = 64;

        let mut gradual = CatchGradualPerformanceAttributes::new(&map, mods);
        let state = CatchScoreState::default();

        assert!(gradual
            .process_next_n_objects(state.clone(), usize::MAX)
            .is_some());
        assert!(gradual.process_next_object(state).is_none());
    }

    #[cfg(not(any(feature = "async_tokio", feature = "async_std")))]
    #[test]
    fn next_and_next_n() {
        let map = Beatmap::from_path("./maps/2118524.osu").expect("failed to parse map");
        let mods = 64;
        let state = CatchScoreState::default();

        let mut gradual1 = CatchGradualPerformanceAttributes::new(&map, mods);
        let mut gradual2 = CatchGradualPerformanceAttributes::new(&map, mods);

        for _ in 0..20 {
            let _ = gradual1.process_next_object(state.clone());
            let _ = gradual2.process_next_object(state.clone());
        }

        let n = 80;

        for _ in 1..n {
            let _ = gradual1.process_next_object(state.clone());
        }

        let state = CatchScoreState {
            max_combo: 101,
            n_fruits: 99,
            n_droplets: 2,
            n_tiny_droplets: 68,
            n_tiny_droplet_misses: 0,
            misses: 0,
        };

        let next = gradual1.process_next_object(state.clone());
        let next_n = gradual2.process_next_n_objects(state, n);

        assert_eq!(next_n, next);
    }

    #[cfg(not(any(feature = "async_tokio", feature = "async_std")))]
    #[test]
    fn gradual_end_eq_regular() {
        let map = Beatmap::from_path("./maps/2118524.osu").expect("failed to parse map");
        let mods = 64;
        let regular = CatchPP::new(&map).mods(mods).calculate();

        let mut gradual = CatchGradualPerformanceAttributes::new(&map, mods);

        let state = CatchScoreState {
            max_combo: 730,
            n_fruits: 728,
            n_droplets: 2,
            n_tiny_droplets: 291,
            n_tiny_droplet_misses: 0,
            misses: 0,
        };

        let gradual_end = gradual.process_next_n_objects(state, usize::MAX).unwrap();

        assert_eq!(regular, gradual_end);
    }

    #[cfg(not(any(feature = "async_tokio", feature = "async_std")))]
    #[test]
    fn gradual_eq_regular_passed() {
        let map = Beatmap::from_path("./maps/2118524.osu").expect("failed to parse map");
        let mods = 64;
        let n = 100;
        let regular = CatchPP::new(&map).mods(mods).passed_objects(n).calculate();

        let mut gradual = CatchGradualPerformanceAttributes::new(&map, mods);

        let state = CatchScoreState {
            max_combo: 101,
            n_fruits: 99,
            n_droplets: 2,
            n_tiny_droplets: 68,
            n_tiny_droplet_misses: 0,
            misses: 0,
        };

        let gradual = gradual.process_next_n_objects(state, n).unwrap();

        assert_eq!(regular, gradual);
    }
}
