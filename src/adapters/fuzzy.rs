use crate::domain::entry::Entry;
use nucleo::{Config, Matcher, Utf32String};

/// Kết quả fuzzy match
pub struct MatchResult {
    pub entry: Entry,
    pub score: u32,
    pub indices: Vec<u32>,
}

/// SIMD-accelerated fuzzy matcher sử dụng nucleo
pub struct NucleoMatcher {
    config: Config,
}

impl NucleoMatcher {
    pub fn new() -> Self {
        Self {
            config: Config::DEFAULT,
        }
    }

    /// Tìm các entries khớp với pattern
    pub fn match_entries(&self, pattern: &str, entries: &[Entry]) -> Vec<MatchResult> {
        let trimmed_pattern = pattern.trim();
        if trimmed_pattern.is_empty() {
            return entries
                .iter()
                .map(|e| MatchResult {
                    entry: e.clone(),
                    score: u32::MAX,
                    indices: Vec::new(),
                })
                .collect();
        }

        let mut matcher = Matcher::new(self.config.clone());
        let needle = Utf32String::from(trimmed_pattern.to_lowercase().as_str());

        let mut scored_results: Vec<MatchResult> = entries
            .iter()
            .filter_map(|entry| {
                let display_lower = entry.display.to_lowercase();
                let haystack = Utf32String::from(display_lower.as_str());

                let mut indices = Vec::new();
                let score =
                    matcher.fuzzy_indices(haystack.slice(..), needle.slice(..), &mut indices)?;

                Some(MatchResult {
                    entry: entry.clone(),
                    score: score as u32,
                    indices,
                })
            })
            .collect();

        scored_results.sort_by_key(|r| std::cmp::Reverse(r.score));
        scored_results
    }
}

impl Default for NucleoMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entry::SortPriority;

    fn make_window(name: &str) -> Entry {
        Entry::window(
            "test".into(),
            "0".into(),
            name.into(),
            "/path".into(),
            SortPriority::OtherSessionWindow,
            false,
        )
    }

    #[test]
    fn empty_pattern_returns_all_entries_with_max_score() {
        let matcher = NucleoMatcher::new();
        let entries = vec![make_window("alpha"), make_window("beta")];

        let results = matcher.match_entries("", &entries);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].score, u32::MAX);
        assert_eq!(results[1].score, u32::MAX);
    }

    #[test]
    fn fuzzy_matches_partial_chars() {
        let matcher = NucleoMatcher::new();
        let entries = vec![
            make_window("main"),
            make_window("Makefile"),
            make_window("testing"),
        ];

        let results = matcher.match_entries("mf", &entries);

        assert!(
            !results.is_empty(),
            "Should have at least one match for 'mf'"
        );

        let has_makefile = results.iter().any(|r| r.entry.display.contains("Makefile"));
        assert!(has_makefile, "Should match 'Makefile' window");
    }

    #[test]
    fn results_sorted_by_relevance_score() {
        let matcher = NucleoMatcher::new();
        let entries = vec![
            make_window("alpha"),
            make_window("beta"),
            make_window("gamma"),
        ];

        let results = matcher.match_entries("alp", &entries);

        assert!(!results.is_empty(), "Should have at least one match");

        if results.len() > 1 {
            assert!(
                results[0].score >= results[1].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn indices_match_display_char_positions() {
        let matcher = NucleoMatcher::new();
        let entry = Entry::window(
            "mysession".into(),
            "0".into(),
            "main".into(),
            "/path".into(),
            SortPriority::OtherSessionWindow,
            false,
        );

        let results = matcher.match_entries("main", std::slice::from_ref(&entry));

        assert_eq!(results.len(), 1);
        let indices = &results[0].indices;

        let display_chars: Vec<char> = entry.display.chars().collect();
        for &idx in indices {
            let idx = idx as usize;
            if idx < display_chars.len() {
                let ch = display_chars[idx];
                let ch_lower = ch.to_lowercase().to_string();
                assert!(
                    ch_lower.contains('m')
                        || ch_lower.contains('a')
                        || ch_lower.contains('i')
                        || ch_lower.contains('n'),
                    "Character at index {} should be part of 'main', got '{}'",
                    idx,
                    ch
                );
            }
        }
    }
}
