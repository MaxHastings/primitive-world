//! World-coordinate augmentation only. No controller or behavior changes.

#[cfg(test)]
pub fn rotate_point(mut point: [f32; 2], extent: f32, turns: u32) -> [f32; 2] {
    assert!(turns < 4);
    for _ in 0..turns {
        point = [extent - point[1], point[0]];
    }
    point
}

pub fn rotate_point_rect(point: [f32; 2], width: f32, height: f32, turns: u32) -> [f32; 2] {
    assert!(turns < 4 && width > 0.0 && height > 0.0);
    let mut normalized = [point[0] / width, point[1] / height];
    for _ in 0..turns {
        normalized = [1.0 - normalized[1], normalized[0]];
    }
    [normalized[0] * width, normalized[1] * height]
}

pub fn rotate_grid<T: Copy>(grid: Vec<T>, side: usize, turns: u32) -> Vec<T> {
    assert!(turns < 4 && side > 0 && grid.len() == side * side);
    if turns == 0 {
        return grid;
    }
    (0..grid.len())
        .map(|index| {
            let (mut x, mut y) = (index % side, index / side);
            for _ in 0..turns {
                (x, y) = (y, side - 1 - x);
            }
            grid[y * side + x]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_turns_are_exact_permutations_and_match_cell_center_positions() {
        let grid: Vec<_> = (0..16).collect();
        for turns in 0..4 {
            let rotated = rotate_grid(grid.clone(), 4, turns);
            let mut sorted = rotated.clone();
            sorted.sort();
            assert_eq!(sorted, grid);
            for (i, _) in grid.iter().enumerate() {
                let point = rotate_point([(i % 4) as f32 + 0.5, (i / 4) as f32 + 0.5], 4.0, turns);
                assert_eq!(rotated[point[1] as usize * 4 + point[0] as usize], i);
            }
            assert_eq!(rotate_grid(rotated, 4, (4 - turns) % 4), grid);
        }
    }
}
