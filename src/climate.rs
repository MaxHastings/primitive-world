//! Smooth multi-timescale forcing. No named climate modes, fixed season,
//! population feedback, rescue condition or evolutionary schedule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    pub rainfall: f32,
    pub temperature: f32,
}
fn unit(mut seed: u32) -> f64 {
    seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb_352d);
    seed = (seed ^ (seed >> 15)).wrapping_mul(0x846c_a68b);
    f64::from(seed ^ (seed >> 16)) / f64::from(u32::MAX)
}
fn component(tick: u32, seed: u32, period: u32, initial_range: [f64; 2]) -> f64 {
    let epoch = tick / period;
    let t = f64::from(tick % period) / f64::from(period);
    let t = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    // Only the starting keyframe is conditioned. The ordinary interpolation
    // and every subsequent random keyframe are unchanged; no opening timer.
    let a = if epoch == 0 {
        initial_range[0] + (initial_range[1] - initial_range[0]) * unit(seed)
    } else {
        unit(seed ^ epoch.wrapping_mul(0x9e37_79b9))
    };
    let b = unit(seed ^ epoch.wrapping_add(1).wrapping_mul(0x9e37_79b9));
    a + (b - a) * t
}
/// Random-like keyframes at unrelated long timescales, with C2 interpolation.
/// These intervals are correlation scales, not durations of good/bad conditions.
pub fn at(tick: u32, seed: u32, flourishing_start: bool) -> Climate {
    let wet = if flourishing_start {
        [0.8, 0.95]
    } else {
        [0.0, 1.0]
    };
    let mild = if flourishing_start {
        [0.45, 0.55]
    } else {
        [0.0, 1.0]
    };
    let moisture = 0.20 * component(tick, seed ^ 11, 173_003, wet)
        + 0.35 * component(tick, seed ^ 23, 1_100_009, wet)
        + 0.45 * component(tick, seed ^ 37, 4_700_021, wet);
    let temperature = 0.25 * component(tick, seed ^ 51, 281_003, mild)
        + 0.40 * component(tick, seed ^ 71, 1_700_029, mild)
        + 0.35 * component(tick, seed ^ 97, 6_100_033, mild);
    Climate {
        rainfall: (4.5 * (moisture - 0.5)).exp() as f32,
        temperature: temperature as f32,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flourishing_initial_conditions_have_a_long_but_unprotected_opening() {
        let mut first_unfavorable = Vec::new();
        for seed in 0..256 {
            let start = at(0, seed, true);
            assert!(start.rainfall > 3.8);
            assert!((0.45..=0.55).contains(&start.temperature));
            let first = (0..=8_000_000)
                .step_by(10_000)
                .find(|&tick| {
                    let c = at(tick, seed, true);
                    c.rainfall < 1.0 || !(0.25..=0.75).contains(&c.temperature)
                })
                .unwrap_or(8_000_000);
            assert!(first >= 500_000, "seed {seed}: {first}");
            first_unfavorable.push(first);
            // Once all starting keyframes are behind us, conditioning has no
            // remaining effect: the same ordinary seeded process continues.
            for tick in [6_100_033, 12_000_000, 24_000_000] {
                assert_eq!(at(tick, seed, true), at(tick, seed, false));
            }
        }
        first_unfavorable.sort_unstable();
        println!(
            "256 seeds: first unfavorable global forcing (10k sampling), min={} median={} max={} ticks",
            first_unfavorable[0], first_unfavorable[128], first_unfavorable[255]
        );
        assert_ne!(first_unfavorable[0], first_unfavorable[255]);
    }

    #[test]
    fn old_settings_keep_their_unconditioned_climate_and_new_settings_persist() {
        let settings = crate::model::SimSettings::default();
        let mut saved = serde_json::to_value(&settings).unwrap();
        let restored: crate::model::SimSettings = serde_json::from_value(saved.clone()).unwrap();
        assert!(restored.flourishing_start);
        saved.as_object_mut().unwrap().remove("flourishing_start");
        let old: crate::model::SimSettings = serde_json::from_value(saved).unwrap();
        assert!(!old.flourishing_start);
        assert_ne!(at(0, 42, true), at(0, 42, old.flourishing_start));
    }
    #[test]
    fn forcing_is_smooth_bounded_irregular_and_reproducible_over_millions_of_ticks() {
        for seed in [1, 42, 91, 3137] {
            let mut min = f32::MAX;
            let mut max = 0.0f32;
            let mut previous = at(0, seed, true);
            for tick in (1000..=24_000_000).step_by(1000) {
                let now = at(tick, seed, true);
                assert!((0.10..=9.5).contains(&now.rainfall));
                assert!((0.0..=1.0).contains(&now.temperature));
                assert!((now.rainfall - previous.rainfall).abs() < 0.15);
                min = min.min(now.rainfall);
                max = max.max(now.rainfall);
                previous = now;
            }
            assert!(max / min > 2.0);
            for tick in [173_002, 1_100_008, 4_700_020, u32::MAX - 1] {
                assert!(
                    (at(tick + 1, seed, true).rainfall - at(tick, seed, true).rainfall).abs()
                        < 0.0001
                );
                assert_eq!(at(tick, seed, true), at(tick, seed, true));
            }
            assert_ne!(at(0, seed, true), at(4_700_021, seed, true));
        }
        assert_ne!(at(0, 1, true), at(0, 42, true));
    }
}
