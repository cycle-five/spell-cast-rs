use crate::models::{Grid, Multiplier, Position};

/// Result of scoring a word, including gems collected
#[derive(Debug, Clone)]
pub struct ScoreResult {
    /// Total score for the word
    pub score: i32,
    /// Number of gems collected from this word
    pub gems_collected: i32,
}

pub struct Scorer;

impl Scorer {
    /// Calculate the score for a word given its positions on the grid.
    /// Returns total score and number of gems collected.
    ///
    /// Scoring rules (SpellCast):
    /// - Each letter has a base value
    /// - DL (Double Letter) multiplies that letter's value by 2
    /// - TL (Triple Letter) multiplies that letter's value by 3
    /// - DW (Double Word) multiplies the ENTIRE word score by 2
    /// - +10 flat bonus for words with 6 or more letters (not multiplied by DW)
    /// - Gems on used letters are collected
    pub fn calculate_score_with_gems(grid: &Grid, positions: &[Position]) -> ScoreResult {
        let mut letter_score_total = 0;
        let mut has_double_word = false;
        let mut gems_collected = 0;

        for pos in positions {
            let cell = &grid[pos.row][pos.col];
            let base_value = cell.value as i32;

            let letter_score = match &cell.multiplier {
                Some(Multiplier::DoubleLetter) => base_value * 2,
                Some(Multiplier::TripleLetter) => base_value * 3,
                Some(Multiplier::DoubleWord) => {
                    has_double_word = true;
                    base_value // Letter itself is not multiplied, just the word
                }
                None => base_value,
            };

            letter_score_total += letter_score;

            // Collect gems
            if cell.has_gem {
                gems_collected += 1;
            }
        }

        // Apply double word multiplier if present
        let word_score = if has_double_word {
            letter_score_total * 2
        } else {
            letter_score_total
        };

        // Add length bonus (flat +10 for 6+ letters, NOT multiplied by DW)
        let length_bonus = Self::length_bonus(positions.len());
        let total_score = word_score + length_bonus;

        ScoreResult {
            score: total_score,
            gems_collected,
        }
    }

    /// Calculate just the score (backwards compatible with existing code)
    pub fn calculate_score(grid: &Grid, positions: &[Position]) -> i32 {
        Self::calculate_score_with_gems(grid, positions).score
    }

    /// Count gems that would be collected from using these positions
    pub fn count_gems(grid: &Grid, positions: &[Position]) -> i32 {
        positions
            .iter()
            .filter(|pos| grid[pos.row][pos.col].has_gem)
            .count() as i32
    }

    /// Calculate bonus points based on word length
    /// SpellCast gives +10 flat bonus for words with 6+ letters
    fn length_bonus(length: usize) -> i32 {
        if length >= 6 { 10 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GridCell;

    #[test]
    fn test_length_bonus() {
        // SpellCast: +10 for 6+ letters only
        assert_eq!(Scorer::length_bonus(3), 0);
        assert_eq!(Scorer::length_bonus(4), 0);
        assert_eq!(Scorer::length_bonus(5), 0);
        assert_eq!(Scorer::length_bonus(6), 10);
        assert_eq!(Scorer::length_bonus(7), 10);
        assert_eq!(Scorer::length_bonus(8), 10);
    }

    #[test]
    fn test_basic_score_calculation() {
        let grid = vec![vec![
            GridCell {
                letter: 'H',
                value: 4,
                multiplier: None,
                has_gem: false,
            },
            GridCell {
                letter: 'E',
                value: 1,
                multiplier: Some(Multiplier::DoubleLetter),
                has_gem: false,
            },
        ]];

        let positions = vec![Position { row: 0, col: 0 }, Position { row: 0, col: 1 }];

        // H(4) + E(1*2) = 6, no length bonus for 2 letters
        let score = Scorer::calculate_score(&grid, &positions);
        assert_eq!(score, 6);
    }

    #[test]
    fn test_double_word_multiplier() {
        let grid = vec![vec![
            GridCell {
                letter: 'C',
                value: 5,
                multiplier: Some(Multiplier::DoubleWord), // Pink 2x
                has_gem: false,
            },
            GridCell {
                letter: 'A',
                value: 1,
                multiplier: None,
                has_gem: false,
            },
            GridCell {
                letter: 'T',
                value: 2,
                multiplier: None,
                has_gem: false,
            },
        ]];

        let positions = vec![
            Position { row: 0, col: 0 },
            Position { row: 0, col: 1 },
            Position { row: 0, col: 2 },
        ];

        // C(5) + A(1) + T(2) = 8, then x2 for DW = 16
        let score = Scorer::calculate_score(&grid, &positions);
        assert_eq!(score, 16);
    }

    #[test]
    fn test_gem_collection() {
        let grid = vec![vec![
            GridCell {
                letter: 'G',
                value: 3,
                multiplier: None,
                has_gem: true,
            },
            GridCell {
                letter: 'E',
                value: 1,
                multiplier: None,
                has_gem: false,
            },
            GridCell {
                letter: 'M',
                value: 4,
                multiplier: None,
                has_gem: true,
            },
        ]];

        let positions = vec![
            Position { row: 0, col: 0 },
            Position { row: 0, col: 1 },
            Position { row: 0, col: 2 },
        ];

        let result = Scorer::calculate_score_with_gems(&grid, &positions);
        assert_eq!(result.score, 8); // G(3) + E(1) + M(4) = 8
        assert_eq!(result.gems_collected, 2); // 2 gems
    }

    #[test]
    fn test_long_word_bonus_not_multiplied() {
        // Create a 6-letter word with DW
        let grid = vec![vec![
            GridCell { letter: 'S', value: 2, multiplier: Some(Multiplier::DoubleWord), has_gem: false },
            GridCell { letter: 'P', value: 4, multiplier: None, has_gem: false },
            GridCell { letter: 'E', value: 1, multiplier: None, has_gem: false },
            GridCell { letter: 'L', value: 3, multiplier: None, has_gem: false },
            GridCell { letter: 'L', value: 3, multiplier: None, has_gem: false },
            GridCell { letter: 'S', value: 2, multiplier: None, has_gem: false },
        ]];

        let positions: Vec<Position> = (0..6).map(|col| Position { row: 0, col }).collect();

        // S(2) + P(4) + E(1) + L(3) + L(3) + S(2) = 15
        // With DW: 15 * 2 = 30
        // With 6-letter bonus: 30 + 10 = 40 (bonus NOT multiplied)
        let score = Scorer::calculate_score(&grid, &positions);
        assert_eq!(score, 40);
    }
}
