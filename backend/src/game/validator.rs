use crate::models::{Grid, Position};
use std::collections::HashSet;

pub struct WordValidator {
    dictionary: HashSet<String>,
}

impl WordValidator {
    pub fn new(dictionary: HashSet<String>) -> Self {
        Self { dictionary }
    }

    /// Check if word exists in dictionary
    pub fn is_valid_word(&self, word: &str) -> bool {
        self.dictionary.contains(&word.to_uppercase())
    }

    /// Validate that positions form a valid path on the grid
    pub fn is_valid_path(&self, _grid: &Grid, positions: &[Position]) -> bool {
        if positions.is_empty() {
            return false;
        }

        // Check that each position is adjacent to the previous one
        for window in positions.windows(2) {
            if !Self::are_adjacent(&window[0], &window[1]) {
                return false;
            }
        }

        // Check that no position is used twice
        let unique_positions: HashSet<_> = positions.iter().collect();
        if unique_positions.len() != positions.len() {
            return false;
        }

        // Check that all positions are within bounds
        positions.iter().all(|pos| pos.row < 5 && pos.col < 5)
    }

    /// Check if two positions are adjacent (including diagonals)
    fn are_adjacent(pos1: &Position, pos2: &Position) -> bool {
        let row_diff = (pos1.row as i32 - pos2.row as i32).abs();
        let col_diff = (pos1.col as i32 - pos2.col as i32).abs();

        row_diff <= 1 && col_diff <= 1 && (row_diff + col_diff > 0)
    }

    /// Extract word from grid positions
    pub fn extract_word(&self, grid: &Grid, positions: &[Position]) -> String {
        positions
            .iter()
            .map(|pos| grid[pos.row][pos.col].letter)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjacent_positions() {
        let pos1 = Position { row: 0, col: 0 };
        let pos2 = Position { row: 0, col: 1 };
        let pos3 = Position { row: 1, col: 1 };
        let pos4 = Position { row: 2, col: 2 };

        assert!(WordValidator::are_adjacent(&pos1, &pos2));
        assert!(WordValidator::are_adjacent(&pos2, &pos3));
        assert!(!WordValidator::are_adjacent(&pos1, &pos4));
    }
}
