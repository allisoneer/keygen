/// Corpus normalization and quartad (4-gram) counting for 26-key layout
use std::collections::HashMap;

const BREAK_MARKER: u8 = 255;

/// Normalize a string to lowercase letter codes (0-25) with break markers
///
/// - ASCII letters (a-z/A-Z) are converted to lowercase codes (0-25)
/// - All other characters become break markers (255)
pub fn normalize(s: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(s.len());

    for c in s.chars() {
        if c.is_ascii_alphabetic() {
            let code = c.to_ascii_lowercase() as u8 - b'a';
            result.push(code);
        } else {
            result.push(BREAK_MARKER);
        }
    }

    result
}

/// Count quartads (4-grams) in normalized text
///
/// Quartads are only counted within continuous letter sequences.
/// Break markers (non-letters) reset the sequence.
pub fn quartads(normalized: &[u8]) -> HashMap<[u8; 4], usize> {
    let mut counts = HashMap::new();
    let mut buffer: Vec<u8> = Vec::with_capacity(4);

    for &byte in normalized {
        if byte == BREAK_MARKER {
            // Reset on break
            buffer.clear();
        } else {
            // Add to buffer
            buffer.push(byte);

            // If we have 4+ letters, extract quartad
            if buffer.len() >= 4 {
                // Keep only the last 4
                if buffer.len() > 4 {
                    buffer.remove(0);
                }

                let quartad: [u8; 4] = [buffer[0], buffer[1], buffer[2], buffer[3]];
                *counts.entry(quartad).or_insert(0) += 1;
            }
        }
    }

    counts
}

pub fn monograms(normalized: &[u8]) -> [usize; 26] {
    let mut counts = [0usize; 26];
    for &b in normalized {
        if b != BREAK_MARKER {
            counts[b as usize] += 1;
        }
    }
    counts
}

pub fn letter_count(normalized: &[u8]) -> usize {
    normalized.iter().filter(|&&b| b != BREAK_MARKER).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize() {
        assert_eq!(
            normalize("hello"),
            vec![7, 4, 11, 11, 14] // h=7, e=4, l=11, o=14
        );

        assert_eq!(
            normalize("Hello World!"),
            vec![7, 4, 11, 11, 14, 255, 22, 14, 17, 11, 3, 255]
        );

        assert_eq!(
            normalize("ABC123xyz"),
            vec![0, 1, 2, 255, 255, 255, 23, 24, 25]
        );
    }

    #[test]
    fn test_quartads_simple() {
        let normalized = normalize("hello");
        let quads = quartads(&normalized);

        // "hello" has quartads: "hell" and "ello"
        assert_eq!(quads.len(), 2);
        assert_eq!(quads.get(&[7, 4, 11, 11]).copied(), Some(1)); // "hell"
        assert_eq!(quads.get(&[4, 11, 11, 14]).copied(), Some(1)); // "ello"
    }

    #[test]
    fn test_quartads_with_breaks() {
        let normalized = normalize("hello world");
        let quads = quartads(&normalized);

        // "hello" -> "hell", "ello"
        // " " breaks the sequence
        // "world" -> "worl", "orld"
        assert_eq!(quads.len(), 4);
        assert_eq!(quads.get(&[7, 4, 11, 11]).copied(), Some(1)); // "hell"
        assert_eq!(quads.get(&[4, 11, 11, 14]).copied(), Some(1)); // "ello"
        assert_eq!(quads.get(&[22, 14, 17, 11]).copied(), Some(1)); // "worl"
        assert_eq!(quads.get(&[14, 17, 11, 3]).copied(), Some(1)); // "orld"
    }

    #[test]
    fn test_quartads_repeated() {
        let normalized = normalize("aaaa aaaa");
        let quads = quartads(&normalized);

        // Both "aaaa" sequences create the same quartad
        assert_eq!(quads.len(), 1);
        assert_eq!(quads.get(&[0, 0, 0, 0]).copied(), Some(2));
    }

    #[test]
    fn test_empty_and_short() {
        assert_eq!(quartads(&normalize("")).len(), 0);
        assert_eq!(quartads(&normalize("a")).len(), 0);
        assert_eq!(quartads(&normalize("ab")).len(), 0);
        assert_eq!(quartads(&normalize("abc")).len(), 0);
        assert_eq!(quartads(&normalize("abcd")).len(), 1);
    }

    #[test]
    fn test_non_letters_only() {
        assert_eq!(quartads(&normalize("123 !@# $%^")).len(), 0);
    }

    #[test]
    fn test_monograms_and_letter_count() {
        let n = normalize("aAa!bB  c");
        let counts = monograms(&n);
        let total = letter_count(&n);
        assert_eq!(total, 6);
        assert_eq!(counts[0], 3); // a
        assert_eq!(counts[1], 2); // b
        assert_eq!(counts[2], 1); // c
    }
}
