use std::collections::HashSet;
use std::path::Path;
use tokio::fs;
use anyhow::Result;

pub struct Dictionary {
    words: HashSet<String>,
}

impl Dictionary {
    /// Load dictionary from a file
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let words: HashSet<String> = content
            .lines()
            .map(|line| line.trim().to_uppercase())
            .filter(|word| !word.is_empty() && word.len() >= 2)
            .collect();

        tracing::info!("Loaded {} words into dictionary", words.len());

        Ok(Self { words })
    }

    /// Create an empty dictionary (for testing)
    pub fn empty() -> Self {
        Self {
            words: HashSet::new(),
        }
    }

    /// Check if a word exists in the dictionary
    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(&word.to_uppercase())
    }

    /// Get the number of words in the dictionary
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Check if dictionary is empty
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_dictionary() {
        let dict = Dictionary::empty();
        assert!(dict.is_empty());
        assert!(!dict.contains("TEST"));
    }
}
