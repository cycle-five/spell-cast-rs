use rand::Rng;

use crate::{
    models::{Grid, GridCell, Multiplier},
    utils::letters::{get_cumulative_distribution, get_letter_value},
};

pub struct GridGenerator;

impl GridGenerator {
    /// Generate a new 5x5 grid with weighted letter distribution
    pub fn generate() -> Grid {
        let mut rng = rand::rng();
        let cumulative_dist = get_cumulative_distribution();
        let total = cumulative_dist.last().unwrap().1;

        let mut grid = Vec::with_capacity(5);

        for _ in 0..5 {
            let mut row = Vec::with_capacity(5);
            for _ in 0..5 {
                let letter = Self::random_letter(&cumulative_dist, total, &mut rng);
                row.push(GridCell {
                    letter,
                    value: get_letter_value(letter),
                    multiplier: None,
                });
            }
            grid.push(row);
        }

        // Add multipliers
        Self::add_multipliers(&mut grid, &mut rng);

        grid
    }

    fn random_letter(cumulative_dist: &[(char, f32)], total: f32, rng: &mut impl Rng) -> char {
        let random_value = rng.random::<f32>() * total;

        for (letter, cumulative) in cumulative_dist {
            if random_value <= *cumulative {
                return *letter;
            }
        }

        'E' // Fallback
    }

    fn add_multipliers(grid: &mut Grid, rng: &mut impl Rng) {
        // Add 3-5 double letter multipliers
        let dl_count = rng.random_range(3..=5);
        for _ in 0..dl_count {
            let row = rng.random_range(0..5);
            let col = rng.random_range(0..5);
            if grid[row][col].multiplier.is_none() {
                grid[row][col].multiplier = Some(Multiplier::DoubleLetter);
            }
        }

        // Add 2-3 triple letter multipliers
        let tl_count = rng.random_range(2..=3);
        for _ in 0..tl_count {
            let row = rng.random_range(0..5);
            let col = rng.random_range(0..5);
            if grid[row][col].multiplier.is_none() {
                grid[row][col].multiplier = Some(Multiplier::TripleLetter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_generation() {
        let grid = GridGenerator::generate();
        assert_eq!(grid.len(), 5);
        assert!(grid.iter().all(|row| row.len() == 5));
    }

    #[test]
    fn test_grid_has_multipliers() {
        let grid = GridGenerator::generate();
        let multiplier_count = grid
            .iter()
            .flatten()
            .filter(|cell| cell.multiplier.is_some())
            .count();
        assert!(multiplier_count >= 5 && multiplier_count <= 8);
    }
}
