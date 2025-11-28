use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Letter values similar to Scrabble scoring
/// TODO: Will be used by game engine once integrated
#[allow(dead_code)]
pub static LETTER_VALUES: Lazy<HashMap<char, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // 1 point letters
    for ch in ['A', 'E', 'I', 'O', 'U', 'L', 'N', 'R', 'S', 'T'] {
        map.insert(ch, 1);
    }

    // 2 points
    for ch in ['D', 'G'] {
        map.insert(ch, 2);
    }

    // 3 points
    for ch in ['B', 'C', 'M', 'P'] {
        map.insert(ch, 3);
    }

    // 4 points
    for ch in ['F', 'H', 'V', 'W', 'Y'] {
        map.insert(ch, 4);
    }

    // 5 points
    map.insert('K', 5);

    // 8 points
    for ch in ['J', 'X'] {
        map.insert(ch, 8);
    }

    // 10 points
    for ch in ['Q', 'Z'] {
        map.insert(ch, 10);
    }

    map
});

/// Letter frequency distribution for English (approximate)
/// Used for weighted random generation
/// TODO: Will be used by grid generator once integrated
#[allow(dead_code)]
pub static LETTER_DISTRIBUTION: Lazy<Vec<(char, f32)>> = Lazy::new(|| {
    vec![
        ('E', 12.70),
        ('T', 9.05),
        ('A', 8.16),
        ('O', 7.50),
        ('I', 6.96),
        ('N', 6.74),
        ('S', 6.32),
        ('H', 6.09),
        ('R', 5.98),
        ('D', 4.25),
        ('L', 4.02),
        ('C', 2.78),
        ('U', 2.75),
        ('M', 2.40),
        ('W', 2.36),
        ('F', 2.22),
        ('G', 2.01),
        ('Y', 1.97),
        ('P', 1.92),
        ('B', 1.49),
        ('V', 0.97),
        ('K', 0.77),
        ('J', 0.15),
        ('X', 0.15),
        ('Q', 0.09),
        ('Z', 0.07),
    ]
});

/// Get the point value for a letter
/// TODO: Will be used by scorer once integrated
#[allow(dead_code)]
pub fn get_letter_value(letter: char) -> u8 {
    let upper = letter.to_ascii_uppercase();
    *LETTER_VALUES.get(&upper).unwrap_or(&1)
}

/// Calculate the cumulative distribution for weighted random selection
/// TODO: Will be used by grid generator once integrated
#[allow(dead_code)]
pub fn get_cumulative_distribution() -> Vec<(char, f32)> {
    let mut cumulative = 0.0;
    LETTER_DISTRIBUTION
        .iter()
        .map(|(ch, freq)| {
            cumulative += freq;
            (*ch, cumulative)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letter_values() {
        assert_eq!(get_letter_value('E'), 1);
        assert_eq!(get_letter_value('Q'), 10);
        assert_eq!(get_letter_value('X'), 8);
        assert_eq!(get_letter_value('D'), 2);
    }

    #[test]
    fn test_cumulative_distribution() {
        let dist = get_cumulative_distribution();
        assert!(dist.len() == 26);
        // Last entry should be close to 100%
        assert!((dist.last().unwrap().1 - 100.0).abs() < 1.0);
    }
}
