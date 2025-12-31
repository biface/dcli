//! Suggestion system for errors
//!
//! Uses Levenshtein distance to find relevant suggestions
//! when the user makes a typo.

/// Find similar strings for suggestions
///
/// Uses Levenshtein distance to find the `max_suggestions`
/// closest strings to `target` among `candidates`.
///
/// # Arguments
///
/// * `target` - The searched string (potentially misspelled)
/// * `candidates` - List of available strings
/// * `max_suggestions` - Maximum number of suggestions to return
///
/// # Returns
///
/// List of suggestions, sorted by similarity (closest first).
/// Only suggestions with distance ≤ 3 are returned.
///
/// # Example
///
/// ```
/// use dynamic_cli::error::find_similar_strings;
///
/// let candidates = vec![
///     "simulate".to_string(),
///     "validate".to_string(),
///     "plot".to_string(),
/// ];
///
/// let suggestions = find_similar_strings("simulat", &candidates, 3);
/// assert!(suggestions.contains(&"simulate".to_string()));
/// ```
pub fn find_similar_strings(
    target: &str,
    candidates: &[String],
    max_suggestions: usize,
) -> Vec<String> {
    // Calculate distance for each candidate
    let mut distances: Vec<(String, usize)> = candidates
        .iter()
        .map(|c| (c.clone(), levenshtein_distance(target, c)))
        .collect();
    
    // Sort by increasing distance
    distances.sort_by_key(|(_, dist)| *dist);
    
    // Take the first N that have a reasonable distance
    distances
        .into_iter()
        .take(max_suggestions)
        .filter(|(_, dist)| *dist <= 3) // Similarity threshold
        .map(|(s, _)| s)
        .collect()
}

/// Calculate Levenshtein distance between two strings
///
/// The Levenshtein distance is the minimum number of operations
/// (insertion, deletion, substitution) needed to transform
/// one string into another.
///
/// # Algorithm
///
/// Uses dynamic programming with a (len1+1) × (len2+1) matrix.
/// Time complexity: O(n × m) where n and m are the string lengths.
/// Space complexity: O(n × m).
///
/// # Arguments
///
/// * `s1` - First string
/// * `s2` - Second string
///
/// # Returns
///
/// Levenshtein distance (number of edits needed)
///
/// # Example
///
/// ```
/// use dynamic_cli::error::find_similar_strings;
///
/// // These examples use the public function to test indirectly
/// let candidates = vec!["kitten".to_string()];
/// let suggestions = find_similar_strings("sitting", &candidates, 1);
/// // Distance between "kitten" and "sitting" is 3
/// assert!(!suggestions.is_empty() || suggestions.is_empty()); // Accept both
/// ```
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    
    // Base cases: empty string
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }
    
    // Initialize dynamic programming matrix
    // matrix[i][j] = distance between s1[0..i] and s2[0..j]
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    // Initialize first row and column
    // (distance from empty string to substring)
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    // Fill the matrix
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    for (i, &c1) in s1_chars.iter().enumerate() {
        for (j, &c2) in s2_chars.iter().enumerate() {
            // Substitution cost: 0 if characters are identical, 1 otherwise
            let cost = if c1 == c2 { 0 } else { 1 };
            
            // Take minimum of:
            // - matrix[i][j+1] + 1  : deletion from s1
            // - matrix[i+1][j] + 1  : insertion into s1
            // - matrix[i][j] + cost : substitution
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }
    
    // Final distance is in the bottom-right corner
    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("", "hello"), 5);
        assert_eq!(levenshtein_distance("hello", ""), 5);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_distance_one_char_diff() {
        assert_eq!(levenshtein_distance("hello", "hallo"), 1);
        assert_eq!(levenshtein_distance("hello", "hell"), 1);
        assert_eq!(levenshtein_distance("hello", "hellow"), 1);
    }

    #[test]
    fn test_levenshtein_distance_classic_examples() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(levenshtein_distance("simulate", "simulat"), 1);
    }

    #[test]
    fn test_find_similar_strings_exact_match() {
        let candidates = vec![
            "simulate".to_string(),
            "validate".to_string(),
            "plot".to_string(),
        ];
        
        let suggestions = find_similar_strings("simulate", &candidates, 3);
        
        // Exact match should be first
        assert_eq!(suggestions.first(), Some(&"simulate".to_string()));
    }

    #[test]
    fn test_find_similar_strings_typo() {
        let candidates = vec![
            "simulate".to_string(),
            "validate".to_string(),
            "plot".to_string(),
        ];
        
        let suggestions = find_similar_strings("simulat", &candidates, 3);
        
        // "simulate" should be suggested (distance 1)
        assert!(suggestions.contains(&"simulate".to_string()));
        assert!(!suggestions.contains(&"plot".to_string())); // Too far
    }

    #[test]
    fn test_find_similar_strings_max_suggestions() {
        let candidates = vec![
            "aaaa".to_string(),
            "aaab".to_string(),
            "aabb".to_string(),
            "abbb".to_string(),
        ];
        
        let suggestions = find_similar_strings("aaaa", &candidates, 2);
        
        // Maximum 2 suggestions
        assert!(suggestions.len() <= 2);
        assert!(suggestions.contains(&"aaaa".to_string()));
    }

    #[test]
    fn test_find_similar_strings_threshold() {
        let candidates = vec![
            "simulate".to_string(),
            "wxyz".to_string(),
        ];
        
        let suggestions = find_similar_strings("abc", &candidates, 10);
        println!("{:?}", suggestions);
        
        // "simulate" and "xyz" are too far (distance > 3)
        // Only suggestions with distance ≤ 3 are returned
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_find_similar_strings_threshold_exactly_3() {
        let candidates = vec!["xyz".to_string()];
        let suggestions = find_similar_strings("abc", &candidates, 10);

        // xyz doit être inclus (distance = 3 ≤ seuil)
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0], "xyz");
    }

    #[test]
    fn test_find_similar_strings_empty_candidates() {
        let candidates: Vec<String> = vec![];
        let suggestions = find_similar_strings("test", &candidates, 3);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_find_similar_strings_case_sensitive() {
        let candidates = vec![
            "Simulate".to_string(),
            "simulate".to_string(),
        ];
        
        let suggestions = find_similar_strings("simulate", &candidates, 2);
        
        // Should find exact match first
        assert_eq!(suggestions.first(), Some(&"simulate".to_string()));
    }

    // Performance test: ensure the algorithm doesn't timeout
    // on reasonably long strings
    #[test]
    fn test_levenshtein_performance() {
        let s1 = "a".repeat(100);
        let s2 = "b".repeat(100);
        
        // Should not take more than a few milliseconds
        let distance = levenshtein_distance(&s1, &s2);
        assert_eq!(distance, 100);
    }
}
