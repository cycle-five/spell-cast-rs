use crate::models::{Grid, Multiplier, Position};

pub struct Scorer;

impl Scorer {
    /// Calculate the score for a word given its positions on the grid
    pub fn calculate_score(grid: &Grid, positions: &[Position]) -> i32 {
        let mut total_score = 0;

        for pos in positions {
            let cell = &grid[pos.row][pos.col];
            let base_value = cell.value as i32;

            let letter_score = match &cell.multiplier {
                Some(Multiplier::DoubleLetter) => base_value * 2,
                Some(Multiplier::TripleLetter) => base_value * 3,
                None => base_value,
            };

            total_score += letter_score;
        }

        // Bonus for longer words
        let length_bonus = Self::length_bonus(positions.len());
        total_score += length_bonus;

        total_score
    }

    /// Calculate bonus points based on word length
    fn length_bonus(length: usize) -> i32 {
        match length {
            0..=3 => 0,
            4 => 5,
            5 => 10,
            6 => 15,
            7 => 25,
            8.. => 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GridCell;

    #[test]
    fn test_length_bonus() {
        assert_eq!(Scorer::length_bonus(3), 0);
        assert_eq!(Scorer::length_bonus(4), 5);
        assert_eq!(Scorer::length_bonus(5), 10);
        assert_eq!(Scorer::length_bonus(8), 50);
    }

    #[test]
    fn test_score_calculation() {
        let grid = vec![vec![
            GridCell {
                letter: 'H',
                value: 4,
                multiplier: None,
            },
            GridCell {
                letter: 'E',
                value: 1,
                multiplier: Some(Multiplier::DoubleLetter),
            },
        ]];

        let positions = vec![Position { row: 0, col: 0 }, Position { row: 0, col: 1 }];

        // H(4) + E(1*2) = 6, no length bonus for 2 letters
        let score = Scorer::calculate_score(&grid, &positions);
        assert_eq!(score, 6);
    }
}
