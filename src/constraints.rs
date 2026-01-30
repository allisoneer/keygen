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

use crate::geometry::{Hand, GEOMETRY, NUM_KEYS};
use crate::layout_26::Layout;

#[derive(Clone, Default)]
pub struct Constraints {
    forbid_same_hand_words: Vec<Word>,
}

#[derive(Clone, Debug)]
pub struct Word {
    codes: Vec<u8>, // 0..=25
}

impl Constraints {
    pub fn with_forbid_same_hand_words<T: AsRef<str>>(words: &[T]) -> Self {
        let mut v = Vec::new();
        for w in words {
            if let Some(parsed) = Word::from_str(w.as_ref()) {
                v.push(parsed);
            }
        }
        Constraints { forbid_same_hand_words: v }
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
    for pos in 0..NUM_KEYS {
        let letter = layout.positions[pos];
        let idx = (letter as u8 - b'a') as usize;
        hands[idx] = GEOMETRY[pos].hand;
    }
    hands
}

impl Word {
    pub fn from_str(s: &str) -> Option<Word> {
        let mut codes = Vec::with_capacity(s.len());
        for ch in s.chars() {
            if ch.is_ascii_alphabetic() {
                let b = ch.to_ascii_lowercase() as u8;
                if (b'a'..=b'z').contains(&b) {
                    codes.push(b - b'a');
                }
            }
        }
        if codes.len() < 2 {
            return None;
        }
        Some(Word { codes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_26::Layout;

    #[test]
    fn test_word_parsing_filters_non_letters() {
        let w = Word::from_str("you!").unwrap();
        assert_eq!(w.codes, vec![24, 14, 20]); // y o u
    }

    #[test]
    fn test_word_parsing_lowercase() {
        let w = Word::from_str("YOU").unwrap();
        assert_eq!(w.codes, vec![24, 14, 20]);
    }

    #[test]
    fn test_single_letter_words_ignored() {
        assert!(Word::from_str("I").is_none());
        assert!(Word::from_str("a").is_none());
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
}
