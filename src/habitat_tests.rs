//! Geography-only diagnostics; never run or score agents.
use super::*;

fn mean(a: &[f32]) -> f64 {
    a.iter().map(|v| f64::from(*v)).sum::<f64>() / a.len() as f64
}

fn correlation(a: &[f32], distance: usize) -> f64 {
    let n = RESOURCE_GRID as usize;
    let m = mean(a);
    let variance = a.iter().map(|v| (f64::from(*v) - m).powi(2)).sum::<f64>();
    let mut covariance = 0.0;
    for y in 0..n {
        for x in 0..n {
            let v = f64::from(a[y * n + x]) - m;
            covariance += v * (f64::from(a[y * n + (x + distance) % n]) - m);
            covariance += v * (f64::from(a[((y + distance) % n) * n + x]) - m);
        }
    }
    covariance / (2.0 * variance)
}

// Four-neighbor components on the torus, at explicit habitat thresholds.
fn components(a: &[f32], threshold: f32) -> Vec<usize> {
    let n = RESOURCE_GRID as usize;
    let mut seen = vec![false; a.len()];
    let mut sizes = Vec::new();
    for start in 0..a.len() {
        if seen[start] || a[start] <= threshold {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let mut size = 0;
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % n, i / n);
            for j in [
                y * n + (x + 1) % n,
                y * n + (x + n - 1) % n,
                ((y + 1) % n) * n + x,
                ((y + n - 1) % n) * n + x,
            ] {
                if !seen[j] && a[j] > threshold {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        sizes.push(size);
    }
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes
}

fn seam_rms(a: &[f32]) -> f64 {
    let n = RESOURCE_GRID as usize;
    let sum = (0..n)
        .map(|i| {
            f64::from(a[i * n] - a[i * n + n - 1]).powi(2)
                + f64::from(a[i] - a[(n - 1) * n + i]).powi(2)
        })
        .sum::<f64>();
    (sum / (2 * n) as f64).sqrt()
}

#[test]
fn habitat_old_new_diagnostics() {
    for (seed, epoch) in [(1, 0), (42, 3), (91, 30), (3137, 101)] {
        let old = build_legacy_habitat_at(seed, epoch, 1.0);
        let new = build_habitat_at(seed, epoch, 1.0);
        assert!((mean(&old) - mean(&new)).abs() < 1e-7);
        assert_eq!(new, build_habitat_at(seed, epoch, 1.0));
        assert_ne!(new, build_habitat_at(seed ^ 123, epoch, 1.0));
        assert_ne!(new, build_habitat_at(seed, epoch + 1, 1.0));
        for (label, a) in [("old", &old), ("new", &new)] {
            assert!(a.iter().all(|v| v.is_finite() && *v >= 0.0));
            let barren = a.iter().filter(|v| **v == 0.0).count() as f64 / a.len() as f64;
            let weak = a.iter().filter(|v| **v < 0.01).count() as f64 / a.len() as f64;
            let ac: Vec<_> = [1, 4, 16, 64, 128].map(|d| correlation(a, d)).into();
            let productivity = terrain_pair(a, a)
                .iter()
                .map(|v| f64::from(v[2]))
                .sum::<f64>()
                / a.len() as f64;
            let initial_food: u64 = build_resources(a).iter().map(|v| u64::from(*v)).sum();
            eprintln!(
                "seed={seed} epoch={epoch} {label}: mean={:.8} productivity={productivity:.6} barren={barren:.4} below_0.01={weak:.4} ac[1,4,16,64,128]={ac:.4?} seam_rms={:.6} initial_food={initial_food}",
                mean(a),
                seam_rms(a)
            );
            assert!((productivity - 1.0).abs() < 0.001);
            for threshold in [0.0, 0.01, mean(a) as f32] {
                let sizes = components(a, threshold);
                eprintln!(
                    "  threshold={threshold:.6}: components={} largest={:?}",
                    sizes.len(),
                    &sizes[..sizes.len().min(8)]
                );
                assert!(!sizes.is_empty());
            }
            if label == "new" {
                assert!(barren > 0.05 && barren < 0.95);
                assert!(ac[0] > 0.95 && ac[2] > ac[4]);
            }
        }
        // Quantization may lose at most one milli-food per cell in either map.
        let sum = |a: &[f32]| {
            build_resources(a)
                .iter()
                .map(|v| i64::from(*v))
                .sum::<i64>()
        };
        assert!((sum(&old) - sum(&new)).abs() <= new.len() as i64);
    }
}

#[test]
fn habitat_torus_is_continuous_in_value_and_slope() {
    for seed in [1, 42, 3137] {
        for epoch in [0, 3, 101] {
            let f = |x, y| correlated_habitat(x, y, seed, epoch);
            for i in 0..257 {
                let t = i as f32 / 256.0;
                let e = 0.0001;
                for swap in [false, true] {
                    let g = |s| if swap { f(t, s) } else { f(s, t) };
                    assert_eq!(g(0.0), g(1.0));
                    assert!((g(-e) - g(1.0 - e)).abs() < 1e-6);
                    assert!((g(e) - g(1.0 + e)).abs() < 1e-5);
                    assert!((g(-e) - g(e)).abs() < 0.005);
                    let left = (g(0.0) - g(-e)) / e;
                    let right = (g(e) - g(0.0)) / e;
                    assert!(
                        (left - right).abs() < 0.25,
                        "seed={seed} epoch={epoch} t={t} slopes={left},{right}"
                    );
                }
            }
        }
    }
}

#[test]
fn habitat_rotation_and_contrast_preserve_resource_budgets() {
    let old = build_legacy_habitat_at(42, 3, 1.0);
    for contrast in [0.0, 0.5, 1.0] {
        let a = build_habitat_at(42, 3, contrast);
        let b = build_habitat_at(42, 4, contrast);
        assert!((mean(&a) - mean(&old)).abs() < 1e-7);
        if contrast == 0.0 {
            assert!(a.iter().all(|v| *v == a[0]));
        }
        for turns in 0..4 {
            let rotate = |v| crate::environment::rotate_grid(v, RESOURCE_GRID as usize, turns);
            let rotated = rotate(a.clone());
            assert_eq!(
                build_resources(&rotated),
                crate::environment::rotate_grid(build_resources(&a), RESOURCE_GRID as usize, turns)
            );
            // Rotate the canonical pair as the live simulation does; do not
            // recompute a floating-point mean in a different summation order.
            let pair = terrain_pair(&a, &b);
            let rotated_pair =
                crate::environment::rotate_grid(pair.clone(), RESOURCE_GRID as usize, turns);
            let restored = crate::environment::rotate_grid(
                rotated_pair,
                RESOURCE_GRID as usize,
                (4 - turns) % 4,
            );
            assert_eq!(pair, restored);
            assert!((mean(&rotated) - mean(&a)).abs() < 1e-12);
        }
    }
}
