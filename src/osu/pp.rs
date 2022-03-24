use super::{OsuDifficultyAttributes, OsuPerformanceAttributes, OsuScoreState};
use crate::{Beatmap, DifficultyAttributes, Mods, OsuStars, PerformanceAttributes};

/// Performance calculator on osu!standard maps.
///
/// # Example
///
/// ```
/// use rosu_pp::{OsuPP, Beatmap};
///
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
///
/// let pp_result = OsuPP::new(&map)
///     .mods(8 + 64) // HDDT
///     .combo(1234)
///     .misses(1)
///     .accuracy(98.5) // should be set last
///     .calculate();
///
/// println!("PP: {} | Stars: {}", pp_result.pp(), pp_result.stars());
///
/// let next_result = OsuPP::new(&map)
///     .attributes(pp_result)  // reusing previous results for performance
///     .mods(8 + 64)           // has to be the same to reuse attributes
///     .accuracy(99.5)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", next_result.pp(), next_result.stars());
/// ```
#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct OsuPP<'map> {
    map: &'map Beatmap,
    attributes: Option<OsuDifficultyAttributes>,
    mods: u32,
    acc: Option<f64>,
    pub(crate) combo: Option<usize>,

    pub(crate) n300: Option<usize>,
    pub(crate) n100: Option<usize>,
    pub(crate) n50: Option<usize>,
    pub(crate) n_misses: usize,
    pub(crate) passed_objects: Option<usize>,
    clock_rate: Option<f64>,
}

impl<'map> OsuPP<'map> {
    /// Create a new performance calculator for osu!standard maps.
    #[inline]
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map,
            attributes: None,
            mods: 0,
            acc: None,
            combo: None,

            n300: None,
            n100: None,
            n50: None,
            n_misses: 0,
            passed_objects: None,
            clock_rate: None,
        }
    }

    /// Provide the result of a previous difficulty or performance calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: impl OsuAttributeProvider) -> Self {
        if let Some(attributes) = attributes.attributes() {
            self.attributes.replace(attributes);
        }

        self
    }

    /// Specify mods through their bit values.
    ///
    /// See [https://github.com/ppy/osu-api/wiki#mods](https://github.com/ppy/osu-api/wiki#mods)
    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }

    /// Specify the max combo of the play.
    #[inline]
    pub fn combo(mut self, combo: usize) -> Self {
        self.combo.replace(combo);

        self
    }

    /// Specify the amount of 300s of a play.
    #[inline]
    pub fn n300(mut self, n300: usize) -> Self {
        self.n300.replace(n300);

        self
    }

    /// Specify the amount of 100s of a play.
    #[inline]
    pub fn n100(mut self, n100: usize) -> Self {
        self.n100.replace(n100);

        self
    }

    /// Specify the amount of 50s of a play.
    #[inline]
    pub fn n50(mut self, n50: usize) -> Self {
        self.n50.replace(n50);

        self
    }

    /// Specify the amount of misses of a play.
    #[inline]
    pub fn misses(mut self, n_misses: usize) -> Self {
        self.n_misses = n_misses;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// If you want to calculate the performance after every few objects, instead of
    /// using [`OsuPP`] multiple times with different `passed_objects`, you should use
    /// [`OsuGradualPerformanceAttributes`](crate::osu::OsuGradualPerformanceAttributes).
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects.replace(passed_objects);

        self
    }

    /// Adjust the clock rate used in the calculation.
    /// If none is specified, it will take the clock rate based on the mods
    /// i.e. 1.5 for DT, 0.75 for HT and 1.0 otherwise.
    #[inline]
    pub fn clock_rate(mut self, clock_rate: f64) -> Self {
        self.clock_rate = Some(clock_rate);

        self
    }

    /// Provide parameters through an [`OsuScoreState`].
    #[inline]
    pub fn state(mut self, state: OsuScoreState) -> Self {
        let OsuScoreState {
            max_combo,
            n300,
            n100,
            n50,
            misses,
        } = state;

        self.combo = Some(max_combo);
        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.n50 = Some(n50);
        self.n_misses = misses;

        self
    }

    /// Generate the hit results with respect to the given accuracy between `0` and `100`.
    ///
    /// Be sure to set `misses` beforehand!
    /// In case of a partial play, be also sure to set `passed_objects` beforehand!
    pub fn accuracy(mut self, acc: f64) -> Self {
        let n_objects = self
            .passed_objects
            .unwrap_or_else(|| self.map.hit_objects.len());

        let mut acc = acc / 100.0;

        if self.n100.or(self.n50).is_some() {
            let mut n100 = self.n100.unwrap_or(0);
            let mut n50 = self.n50.unwrap_or(0);

            let placed_points = 2 * n100 + n50 + self.n_misses;
            let missing_objects = n_objects - n100 - n50 - self.n_misses;
            let missing_points =
                ((6.0 * acc * n_objects as f64).round() as usize).saturating_sub(placed_points);

            let mut n300 = missing_objects.min(missing_points / 6);
            n50 += missing_objects - n300;

            if let Some(orig_n50) = self.n50.filter(|_| self.n100.is_none()) {
                // Only n50s were changed, try to load some off again onto n100s
                let difference = n50 - orig_n50;
                let n = n300.min(difference / 4);

                n300 -= n;
                n100 += 5 * n;
                n50 -= 4 * n;
            }

            self.n300 = Some(n300);
            self.n100 = Some(n100);
            self.n50 = Some(n50);

            acc = (6 * n300 + 2 * n100 + n50) as f64 / (6 * n_objects) as f64;
        } else {
            let misses = self.n_misses.min(n_objects);
            let target_total = (acc * n_objects as f64 * 6.0).round() as usize;
            let delta = target_total - (n_objects - misses);

            let mut n300 = delta / 5;
            let mut n100 = (delta % 5).min(n_objects - n300 - misses);
            let mut n50 = n_objects - n300 - n100 - misses;

            // Sacrifice n300s to transform n50s into n100s
            let n = n300.min(n50 / 4);
            n300 -= n;
            n100 += 5 * n;
            n50 -= 4 * n;

            self.n300 = Some(n300);
            self.n100 = Some(n100);
            self.n50 = Some(n50);

            acc = (6 * n300 + 2 * n100 + n50) as f64 / (6 * n_objects) as f64;
        }

        self.acc = Some(acc);

        self
    }

    fn assert_hitresults(self, attributes: OsuDifficultyAttributes) -> OsuPPInner {
        let mut n300 = self.n300;
        let mut n100 = self.n100;
        let mut n50 = self.n50;

        let n_objects = self
            .passed_objects
            .unwrap_or_else(|| self.map.hit_objects.len());

        if let Some(acc) = self.acc {
            let n300 = n300.unwrap_or(0);
            let n100 = n100.unwrap_or(0);
            let n50 = n50.unwrap_or(0);

            let total_hits = (n300 + n100 + n50 + self.n_misses).min(n_objects) as f64;

            let effective_misses =
                calculate_effective_misses(&attributes, self.combo, self.n_misses, total_hits);

            OsuPPInner {
                attributes,
                mods: self.mods,
                combo: self.combo,
                acc,
                n300,
                n100,
                n50,
                total_hits,
                effective_misses,
            }
        } else {
            let n_objects = self
                .passed_objects
                .unwrap_or_else(|| self.map.hit_objects.len());

            let remaining = n_objects
                .saturating_sub(n300.unwrap_or(0))
                .saturating_sub(n100.unwrap_or(0))
                .saturating_sub(n50.unwrap_or(0))
                .saturating_sub(self.n_misses);

            if remaining > 0 {
                if let Some(n300) = n300.as_mut() {
                    if n100.is_none() {
                        n100 = Some(remaining);
                    } else if n50.is_none() {
                        n50 = Some(remaining);
                    } else {
                        *n300 += remaining;
                    }
                } else {
                    n300 = Some(remaining);
                }
            }

            let n300 = n300.unwrap_or(0);
            let n100 = n100.unwrap_or(0);
            let n50 = n50.unwrap_or(0);

            let numerator = n300 * 6 + n100 * 2 + n50;

            let acc = if n_objects > 0 {
                numerator as f64 / n_objects as f64 / 6.0
            } else {
                0.0
            };

            let total_hits = (n300 + n100 + n50 + self.n_misses).min(n_objects) as f64;

            let effective_misses =
                calculate_effective_misses(&attributes, self.combo, self.n_misses, total_hits);

            OsuPPInner {
                attributes,
                mods: self.mods,
                combo: self.combo,
                acc,
                n300,
                n100,
                n50,
                total_hits,
                effective_misses,
            }
        }
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> OsuPerformanceAttributes {
        let attributes = self.attributes.take().unwrap_or_else(|| {
            let mut calculator = OsuStars::new(self.map).mods(self.mods);

            if let Some(passed_objects) = self.passed_objects {
                calculator = calculator.passed_objects(passed_objects);
            }

            if let Some(clock_rate) = self.clock_rate {
                calculator = calculator.clock_rate(clock_rate);
            }

            calculator.calculate()
        });

        let id = self.map.beatmap_id.clone();
        self.assert_hitresults(attributes).calculate(&id)
    }
}

struct OsuPPInner {
    attributes: OsuDifficultyAttributes,
    mods: u32,
    acc: f64,
    combo: Option<usize>,

    n300: usize,
    n100: usize,
    n50: usize,

    total_hits: f64,
    effective_misses: usize,
}

impl OsuPPInner {
    fn calculate(self, map_id: &i32) -> OsuPerformanceAttributes {
        let (aim_value, speed_value, acc_value, flashlight_value, pp) =
            if self.total_hits.abs() <= f64::EPSILON {
                (0.0, 0.0, 0.0, 0.0, 0.0)
            } else {
                let mut multiplier = 1.12;

                // NF penalty
                if self.mods.nf() {
                    multiplier *= (1.0 - 0.02 * (self.effective_misses as f64)).max(0.9);
                }

                // SO penalty
                if self.mods.so() {
                    let n_spinners = self.attributes.n_spinners;
                    multiplier *= 1.0 - (n_spinners as f64 / self.total_hits).powf(0.85);
                }

                let mut aim_value = self.compute_aim_value();
                let speed_value = self.compute_speed_value();
                let acc_value = self.compute_accuracy_value();
                let flashlight_value = self.compute_flashlight_value();

                // RX stream penalty
                if self.mods.rx() {
                    let stream_factor = aim_value / speed_value;

                    if stream_factor < 1.0 {
                        let depression_factor = if self.acc >= 0.97 {
                            0.94 - ((0.99 - self.acc.round()) * 2.0)
                        } else {
                            0.87
                        };

                        aim_value *= depression_factor;
                    }
                }

                let mut pp = if self.mods.rx() {
                    (aim_value.powf(1.17) + acc_value.powf(1.15) + flashlight_value.powf(1.1))
                        .powf(1.0 / 1.1)
                        * multiplier
                } else if self.mods.ap() {
                    (acc_value.powf(1.15) + flashlight_value.powf(1.1)).powf(1.0 / 1.1) * multiplier
                } else {
                    (aim_value.powf(1.1)
                        + speed_value.powf(1.1)
                        + acc_value.powf(1.1)
                        + flashlight_value.powf(1.1))
                    .powf(1.0 / 1.1)
                        * multiplier
                };

                if self.mods.rx() {
                    match map_id {
                        1808605 => {
                            // Louder than steel
                            pp *= 0.7;
                        }
                        1821147 => {
                            // Over the top
                            pp *= 0.6;
                        }
                        1849420 => {
                            // Ascension to heaven (mattay)
                            pp *= 0.6;
                        }
                        _ => {}
                    }
                }

                (aim_value, speed_value, acc_value, flashlight_value, pp)
            };

        OsuPerformanceAttributes {
            difficulty: self.attributes,
            pp_acc: acc_value,
            pp_aim: aim_value,
            pp_flashlight: flashlight_value,
            pp_speed: speed_value,
            pp,
        }
    }

    fn compute_aim_value(&self) -> f64 {
        let attributes = &self.attributes;
        let total_hits = self.total_hits;

        // TD penalty
        let raw_aim = if self.mods.td() {
            attributes.aim_strain.powf(0.8)
        } else {
            attributes.aim_strain
        };

        let mut aim_value = (5.0 * (raw_aim / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0;

        // Longer maps are worth more
        let len_bonus = 0.95
            + 0.4 * (total_hits / 2000.0).min(1.0)
            + (total_hits > 2000.0) as u8 as f64 * 0.5 * (total_hits / 2000.0).log10();
        aim_value *= len_bonus;

        // Penalize misses
        let effective_misses = self.effective_misses as f64;
        if effective_misses > 0.0 {
            aim_value *=
                calculate_miss_penalty(effective_misses, attributes.aim_difficult_strain_count);
        }

        // AR bonus
        let ar_factor = if self.mods.rx() {
            if attributes.ar > 10.7 {
                0.4 * (attributes.ar - 10.7)
            } else {
                0.0
            }
        } else {
            if attributes.ar > 10.33 {
                0.3 * (attributes.ar - 10.33)
            } else {
                0.0
            }
        };

        if ar_factor > 0.0 {
            aim_value *= 1.0 + ar_factor * len_bonus; // * Buff for longer maps with high AR.
        } else if attributes.ar < 8.0 {
            let mut buff = 1.3;

            if attributes.ar <= 5.0 {
                buff += (5.0 - attributes.ar) / 50.0;
            }

            aim_value *= (buff * len_bonus).min(1.75);
        }

        // CS bonus
        if attributes.cs > 6.0 && self.mods.rx() {            
            let diff = attributes.cs - 6.0;
            aim_value *= 1.03 + (diff / 20.0);  
        }

        // HD bonus (this would include the Blinds mod but it's currently not representable)
        let hd_factor = match self.mods.rx() {
            true => (0.05, 11.0),
            _ => (0.04, 12.0),
        };

        if self.mods.hd() {
            aim_value *= 1.0 + hd_factor.0 * (hd_factor.1 - attributes.ar);
        }

        if attributes.n_sliders > 0 {
            // * We assume 15% of sliders in a map are difficult since
            // * there's no way to tell from the performance calculator.
            let estimate_difficult_sliders = attributes.n_sliders as f64 * 0.15;

            let non_300s = self.total_hits - self.n300 as f64;
            let missing_combo = attributes.max_combo - self.combo.unwrap_or(attributes.max_combo);

            let estimate_slider_ends_dropped = non_300s
                .min(missing_combo as f64)
                .clamp(0.0, estimate_difficult_sliders);

            let base = 1.0 - estimate_slider_ends_dropped / estimate_difficult_sliders;
            let slider_nerf_factor =
                (1.0 - attributes.slider_factor) * base * base * base + attributes.slider_factor;

            aim_value *= slider_nerf_factor;
        }

        aim_value *= self.acc;
        aim_value *= 0.98 + attributes.od * attributes.od / 2500.0;

        aim_value
    }

    fn compute_speed_value(&self) -> f64 {
        let attributes = &self.attributes;
        let total_hits = self.total_hits;

        let mut speed_value =
            (5.0 * (attributes.speed_strain / 0.0675).max(1.0) - 4.0).powi(3) / 100_000.0;

        // Longer maps are worth more
        let len_bonus = 0.95
            + 0.4 * (total_hits / 2000.0).min(1.0)
            + (total_hits > 2000.0) as u8 as f64 * 0.5 * (total_hits / 2000.0).log10();
        speed_value *= len_bonus;

        // Penalize misses
        let effective_misses = self.effective_misses as f64;
        if effective_misses > 0.0 {
            speed_value *=
                calculate_miss_penalty(effective_misses, attributes.aim_difficult_strain_count);
        }

        // AR bonus
        let ar_factor = if self.mods.rx() {
            if attributes.ar > 10.7 {
                0.4 * (attributes.ar - 10.7)
            } else {
                0.0
            }
        } else {
            if attributes.ar > 10.33 {
                0.3 * (attributes.ar - 10.33)
            } else {
                0.0
            }
        };

        speed_value *= 1.0 + ar_factor * len_bonus; // * Buff for longer maps with high AR.

        // HD bonus (this would include the Blinds mod but it's currently not representable)
        let hd_factor = match self.mods.rx() {
            true => (0.05, 11.0),
            _ => (0.04, 12.0),
        };

        if self.mods.hd() {
            speed_value *= 1.0 + hd_factor.0 * (hd_factor.1 - attributes.ar);
        }

        // Scaling the speed value with accuracy and OD
        let od_factor = 0.95 + attributes.od * attributes.od / 750.0;
        let acc_factor = self.acc.powf((14.5 - attributes.od.max(8.0)) / 2.0);
        speed_value *= od_factor * acc_factor;

        // Penalize n50s
        speed_value *= 0.98_f64.powf(
            (self.n50 as f64 >= total_hits / 500.0) as u8 as f64
                * (self.n50 as f64 - total_hits / 500.0),
        );

        speed_value
    }

    fn compute_accuracy_value(&self) -> f64 {
        let attributes = &self.attributes;
        let total_hits = self.total_hits;
        let n_circles = attributes.n_circles as f64;
        let n300 = self.n300 as f64;
        let n100 = self.n100 as f64;
        let n50 = self.n50 as f64;

        let better_acc_percentage = (n_circles > 0.0) as u8 as f64
            * (((n300 - (total_hits - n_circles)) * 6.0 + n100 * 2.0 + n50) / (n_circles * 6.0))
                .max(0.0);

        let mut acc_value = 1.52163_f64.powf(attributes.od) * better_acc_percentage.powi(24) * 2.83;

        // Bonus for many hitcircles
        acc_value *= ((n_circles as f64 / 1000.0).powf(0.3)).min(1.15);

        // HD bonus (this would include the Blinds mod but it's currently not representable)
        if self.mods.hd() {
            acc_value *= 1.08;
        }

        // FL bonus
        if self.mods.fl() {
            acc_value *= 1.02;
        }

        acc_value
    }

    fn compute_flashlight_value(&self) -> f64 {
        if !self.mods.fl() {
            return 0.0;
        }

        let attributes = &self.attributes;
        let total_hits = self.total_hits;

        // TD penalty
        let raw_flashlight = if self.mods.td() {
            attributes.flashlight_rating.powf(0.8)
        } else {
            attributes.flashlight_rating
        };

        let mut flashlight_value = raw_flashlight * raw_flashlight * 25.0;

        // Add an additional bonus for HDFL
        if self.mods.hd() {
            flashlight_value *= 1.3;
        }

        // Penalize misses by assessing # of misses relative to the total # of objects.
        // Default a 3% reduction for any # of misses
        let effective_misses = self.effective_misses as f64;
        if effective_misses > 0.0 {
            flashlight_value *= 0.97
                * (1.0 - (effective_misses / total_hits).powf(0.775))
                    .powf(effective_misses.powf(0.875));
        }

        // Combo scaling
        if let Some(combo) = self.combo.filter(|_| attributes.max_combo > 0) {
            flashlight_value *= ((combo as f64 / attributes.max_combo as f64).powf(0.8)).min(1.0);
        }

        // Account for shorter maps having a higher ratio of 0 combo/100 combo flashlight radius
        flashlight_value *= 0.7
            + 0.1 * (total_hits / 200.0).min(1.0)
            + (total_hits > 200.0) as u8 as f64 * (0.2 * ((total_hits - 200.0) / 200.0).min(1.0));

        // Scale the aim value with accuracy _slightly_
        flashlight_value *= 0.5 + self.acc / 2.0;

        // It is important to also consider accuracy difficulty when doing that
        flashlight_value *= 0.98 + attributes.od * attributes.od / 2500.0;

        flashlight_value
    }
}

fn calculate_miss_penalty(n_misses: f64, difficult_strain_count: f64) -> f64 {
    // Miss penalty assumes that a player will miss on the hardest parts of a map,
    // so we use the amount of relatively difficult sections to adjust miss penalty
    // to make it more punishing on maps with lower amount of hard sections.
    0.94 / ((n_misses / (2.0 * f64::sqrt(difficult_strain_count))) + 1.0)
}

fn calculate_effective_misses(
    attributes: &OsuDifficultyAttributes,
    combo: Option<usize>,
    n_misses: usize,
    total_hits: f64,
) -> usize {
    // * Guess the number of misses + slider breaks from combo
    let mut combo_based_misses: f64 = 0.0;

    if attributes.n_sliders > 0 {
        let full_combo_threshold = attributes.max_combo as f64 - 0.1 * attributes.n_sliders as f64;

        let f64_combo = combo.map(|c| c as f64);

        if let Some(combo) = f64_combo.filter(|&c| c < full_combo_threshold) {
            combo_based_misses = full_combo_threshold / combo.max(1.0);
        }
    }

    // * Clamp misscount since it's derived from combo and can be
    // * higher than total hits and that breaks some calculations
    combo_based_misses = combo_based_misses.min(total_hits);

    n_misses.max(combo_based_misses.floor() as usize)
}

/// Abstract type to provide flexibility when passing difficulty attributes to a performance calculation.
pub trait OsuAttributeProvider {
    /// Provide the actual difficulty attributes.
    fn attributes(self) -> Option<OsuDifficultyAttributes>;
}

impl OsuAttributeProvider for OsuDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self)
    }
}

impl OsuAttributeProvider for OsuPerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        Some(self.difficulty)
    }
}

impl OsuAttributeProvider for DifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Osu(attributes) = self {
            Some(attributes)
        } else {
            None
        }
    }
}

impl OsuAttributeProvider for PerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<OsuDifficultyAttributes> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Osu(attributes) = self {
            Some(attributes.difficulty)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Beatmap;

    #[test]
    fn osu_only_accuracy() {
        let map = Beatmap::default();

        let total_objects = 1234;
        let target_acc = 97.5;

        let calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .accuracy(target_acc);

        let numerator = 6 * calculator.n300.unwrap_or(0)
            + 2 * calculator.n100.unwrap_or(0)
            + calculator.n50.unwrap_or(0);
        let denominator = 6 * total_objects;
        let acc = 100.0 * numerator as f64 / denominator as f64;

        assert!(
            (target_acc - acc).abs() < 1.0,
            "Expected: {} | Actual: {}",
            target_acc,
            acc
        );
    }

    #[test]
    fn osu_accuracy_and_n50() {
        let map = Beatmap::default();

        let total_objects = 1234;
        let target_acc = 97.5;
        let n50 = 30;

        let calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .n50(n50)
            .accuracy(target_acc);

        assert!(
            (calculator.n50.unwrap() as i32 - n50 as i32).abs() <= 4,
            "Expected: {} | Actual: {}",
            n50,
            calculator.n50.unwrap()
        );

        let numerator = 6 * calculator.n300.unwrap_or(0)
            + 2 * calculator.n100.unwrap_or(0)
            + calculator.n50.unwrap_or(0);
        let denominator = 6 * total_objects;
        let acc = 100.0 * numerator as f64 / denominator as f64;

        assert!(
            (target_acc - acc).abs() < 1.0,
            "Expected: {} | Actual: {}",
            target_acc,
            acc
        );
    }

    #[test]
    fn osu_missing_objects() {
        let map = Beatmap::default();
        let attributes = OsuDifficultyAttributes::default();

        let total_objects = 1234;
        let n300 = 1000;
        let n100 = 200;
        let n50 = 30;

        let calculator = OsuPP::new(&map)
            .passed_objects(total_objects)
            .n300(n300)
            .n100(n100)
            .n50(n50)
            .assert_hitresults(attributes);

        let n_objects = calculator.n300 + calculator.n100 + calculator.n50;

        assert_eq!(
            total_objects, n_objects,
            "Expected: {} | Actual: {}",
            total_objects, n_objects
        );
    }
}
