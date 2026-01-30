use crate::geometry::{Finger, Hand, Row, GEOMETRY, NUM_KEYS};
use rand::random;
/// Layout structure and methods for 26-key keyboard (letters only)
use std::fmt;

#[derive(Clone)]
pub struct Layout {
    /// Maps position index (0-25) to letter ('a'-'z')
    pub positions: [char; NUM_KEYS],
}

impl Layout {
    /// Create a new layout with alphabetical ordering
    pub fn alphabetical() -> Self {
        let mut positions = ['a'; NUM_KEYS];
        for i in 0..NUM_KEYS {
            positions[i] = (b'a' + i as u8) as char;
        }
        Layout { positions }
    }

    /// Create a layout from a string of 26 letters
    pub fn from_string(s: &str) -> Result<Self, String> {
        let chars: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
        if chars.len() != NUM_KEYS {
            return Err(format!(
                "Expected {} letters, got {}",
                NUM_KEYS,
                chars.len()
            ));
        }

        let mut positions = ['a'; NUM_KEYS];
        for (i, &c) in chars.iter().enumerate() {
            positions[i] = c.to_ascii_lowercase();
        }

        let layout = Layout { positions };
        layout.validate()?;
        Ok(layout)
    }

    /// Validate that all 26 letters appear exactly once
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = [false; 26];
        for &c in &self.positions {
            if !c.is_ascii_lowercase() {
                return Err(format!("Non-lowercase letter found: '{}'", c));
            }
            let idx = (c as u8 - b'a') as usize;
            if seen[idx] {
                return Err(format!("Duplicate letter found: '{}'", c));
            }
            seen[idx] = true;
        }

        for (i, &s) in seen.iter().enumerate() {
            if !s {
                let missing = (b'a' + i as u8) as char;
                return Err(format!("Missing letter: '{}'", missing));
            }
        }

        Ok(())
    }

    /// Swap two positions
    pub fn swap(&mut self, i: usize, j: usize) {
        self.positions.swap(i, j);
    }

    /// Perform random swaps
    pub fn shuffle(&mut self, times: usize) {
        for _ in 0..times {
            let i = random::<usize>() % NUM_KEYS;
            let mut j = random::<usize>() % (NUM_KEYS - 1);
            if j >= i {
                j += 1;
            }
            self.swap(i, j);
        }
    }

    /// Get the position (0-25) of a letter, returns None if not a-z
    pub fn get_position(&self, letter: char) -> Option<usize> {
        let letter = letter.to_ascii_lowercase();
        if !letter.is_ascii_lowercase() {
            return None;
        }
        self.positions.iter().position(|&c| c == letter)
    }

    /// Get letter at position (0-25)
    pub fn get_letter(&self, position: usize) -> Option<char> {
        self.positions.get(position).copied()
    }

    /// Get compact string representation of layout
    pub fn to_compact_string(&self) -> String {
        self.positions.iter().collect()
    }

    /// Create a reverse mapping from letter to KeyPress info
    pub fn get_key_info(&self, letter: char) -> Option<KeyInfo> {
        let position = self.get_position(letter)?;
        let key = &GEOMETRY[position];
        Some(KeyInfo {
            letter,
            position,
            hand: key.hand,
            finger: key.finger,
            row: key.row,
            base_cost: key.base_cost,
        })
    }
}

pub struct KeyInfo {
    pub letter: char,
    pub position: usize,
    pub hand: Hand,
    pub finger: Finger,
    pub row: Row,
    pub base_cost: f64,
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Display format with proper column alignment

        // Top row (4 spaces to align above main home columns)
        write!(f, "    ")?;
        for i in 2..6 {
            write!(f, "{} ", self.positions[i])?;
        }
        write!(f, "   |   ")?;
        for i in 13..17 {
            write!(f, "{} ", self.positions[i])?;
        }
        writeln!(f)?;

        // Home row with pinkies
        write!(f, "{} {} ", self.positions[0], self.positions[1])?;
        for i in 6..10 {
            write!(f, "{} ", self.positions[i])?;
        }
        write!(f, "   |   ")?;
        for i in 17..21 {
            write!(f, "{} ", self.positions[i])?;
        }
        write!(f, "{} {}", self.positions[24], self.positions[25])?;
        writeln!(f)?;

        // Bottom row (align under main columns, not pinkies)
        write!(f, "    ")?;
        for i in 10..13 {
            write!(f, "{} ", self.positions[i])?;
        }
        write!(f, "   ")?;
        write!(f, "  |  ")?;
        write!(f, "   ")?;
        for i in 21..24 {
            write!(f, "{} ", self.positions[i])?;
        }

        Ok(())
    }
}
