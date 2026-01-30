//! Hard constraints for keyboard layout optimization.
//!
//! This module provides categorical rejection of layouts that violate
//! specified rules. Unlike soft penalties in the cost module, hard constraints
//! are never satisfied probabilistically - they filter the candidate space absolutely.
//!
//! # Performance
//!
//! Constraint checking is designed for minimal overhead:
//! - Precomputes letter→hand mapping in O(26) per candidate
//! - Reuses mapping for all word checks
//! - Fails fast on first violation
//! - No allocations in hot loop
//!
//! # Extensibility
//!
//! Future constraint types can be added as new fields to `Constraints`.
//! All types can share the same precomputed hand mapping. Examples:
//! - Letter-on-hand: `required_left: Vec<u8>`, `required_right: Vec<u8>`
//! - Pairwise different-hands: `different_hand_pairs: Vec<(u8, u8)>`
//! - Position constraints: `fixed_positions: Vec<(u8, usize)>`

use crate::geometry::{Hand, GEOMETRY};
use crate::layout_26::Layout;

#[derive(Clone, Default)]
pub struct Constraints {
    forbid_same_hand_words: Vec<Word>,
}

use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Word {
    codes: Vec<u8>, // 0..=25
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseWordError;

impl fmt::Display for ParseWordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "word must contain at least two ASCII letters")
    }
}

impl std::error::Error for ParseWordError {}

impl Constraints {
    pub fn with_forbid_same_hand_words<T: AsRef<str>>(words: &[T]) -> Self {
        let mut v = Vec::new();
        for w in words {
            if let Ok(parsed) = w.as_ref().parse::<Word>() {
                v.push(parsed);
            }
        }
        Constraints {
            forbid_same_hand_words: v,
        }
    }

    #[inline]
    pub fn check_layout(&self, layout: &Layout) -> bool {
        if self.forbid_same_hand_words.is_empty() {
            return true;
        }
        let hands = compute_letter_hands(layout);
        for w in &self.forbid_same_hand_words {
            if w.codes.len() < 2 {
                continue; // ignore single-letter "words"
            }
            let first = hands[w.codes[0] as usize];
            if w.codes.iter().skip(1).all(|&c| hands[c as usize] == first) {
                return false;
            }
        }
        true
    }
}

#[inline]
pub fn compute_letter_hands(layout: &Layout) -> [Hand; 26] {
    let mut hands = [Hand::Left; 26];
    // O(26) build: read letter at each position and map to hand
    for (&letter, key) in layout.positions.iter().zip(GEOMETRY.iter()) {
        let idx = (letter as u8 - b'a') as usize;
        hands[idx] = key.hand;
    }
    hands
}

impl FromStr for Word {
    type Err = ParseWordError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut codes = Vec::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_ascii_alphabetic() {
                let b = ch.to_ascii_lowercase() as u8;
                codes.push(b - b'a');
            }
        }
        if codes.len() < 2 {
            return Err(ParseWordError);
        }
        Ok(Word { codes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_26::Layout;

    #[test]
    fn test_word_parsing_filters_non_letters() {
        let w: Word = "you!".parse().unwrap();
        assert_eq!(w.codes, vec![24, 14, 20]); // y o u
    }

    #[test]
    fn test_word_parsing_lowercase() {
        let w: Word = "YOU".parse().unwrap();
        assert_eq!(w.codes, vec![24, 14, 20]);
    }

    #[test]
    fn test_single_letter_words_ignored() {
        assert!("I".parse::<Word>().is_err());
        assert!("a".parse::<Word>().is_err());
    }

    #[test]
    fn test_empty_constraints_always_pass() {
        let c = Constraints::default();
        assert!(c.check_layout(&Layout::alphabetical()));
    }

    #[test]
    fn test_compute_letter_hands_correctness() {
        let layout = Layout::alphabetical();
        let hands = compute_letter_hands(&layout);
        for code in 0..26u8 {
            let ch = (b'a' + code) as char;
            let expected = layout.get_key_info(ch).unwrap().hand;
            assert_eq!(hands[code as usize], expected);
        }
    }

    #[test]
    fn test_forbid_same_hand_blocks_violation() {
        let c = Constraints::with_forbid_same_hand_words(&["you"]);
        let alphabetical = Layout::alphabetical();
        assert!(!c.check_layout(&alphabetical)); // y, o, u are all right-hand
    }

    #[test]
    fn test_forbid_same_hand_allows_mixed_hands() {
        // Move 'o' to left hand to split "you"
        let mut layout = Layout::alphabetical();
        // 'o' at position 14 (right index top), swap with left index home at position 8
        layout.swap(14, 8);
        let c = Constraints::with_forbid_same_hand_words(&["you"]);
        assert!(c.check_layout(&layout));
    }

    #[test]
    fn test_non_ascii_letters_dropped() {
        // "mañana" -> "maana" (ñ dropped, leaving m-a-a-n-a)
        let w: Word = "mañana".parse().unwrap();
        assert_eq!(w.codes, vec![12, 0, 0, 13, 0]);

        // "ß" has no ASCII letters; less than 2 ASCII letters should fail
        assert!("ß".parse::<Word>().is_err());
    }
}
