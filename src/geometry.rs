/// Geometry definitions for 26-key layout (letters only)

pub const NUM_KEYS: usize = 26;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    Pinky,
    Ring,
    Middle,
    Index,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Top,
    Home,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub idx: u8,
    pub hand: Hand,
    pub finger: Finger,
    pub row: Row,
    pub column: u8, // 0=outer edge, increasing towards center
    pub base_cost: f64,
}

// Left hand base costs matrix from the plan
const LEFT_HAND_BASE_COSTS: [[f64; 6]; 3] = [
    // Top row: 4 keys (pinky: 1, ring: 1, middle: 1, index: 1)
    [3.0, 2.0, 1.5, 2.1, 0.0, 0.0],
    // Home row: 6 keys (pinky: 1, ring: 1, middle: 1, index: 3)
    [2.0, 1.0, 0.5, 0.0, 0.0, 1.5],
    // Bottom row: 3 keys (ring: 1, middle: 1, index: 1)
    [2.5, 1.5, 1.6, 0.0, 0.0, 0.0],
];

// Generate geometry array
pub static GEOMETRY: [Key; NUM_KEYS] = [
    // Left hand (13 keys)
    // Pinky column (2 keys) - side position
    Key {
        idx: 0,
        hand: Hand::Left,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 0, // Pinky side (outer)
        base_cost: 2.0,
    },
    Key {
        idx: 1,
        hand: Hand::Left,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 1, // Pinky home
        base_cost: 1.0,
    },
    // Top row: ring, middle, index, index-side
    Key {
        idx: 2,
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Top,
        column: 2, // Ring
        base_cost: 3.0,
    },
    Key {
        idx: 3,
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Top,
        column: 3, // Middle
        base_cost: 2.0,
    },
    Key {
        idx: 4,
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Top,
        column: 4, // Index
        base_cost: 1.5,
    },
    Key {
        idx: 5,
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Top,
        column: 5, // Index side (toward center)
        base_cost: 2.1,
    },
    // Home row: ring, middle, index, index-right
    Key {
        idx: 6,
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Home,
        column: 2, // Ring
        base_cost: 0.5,
    },
    Key {
        idx: 7,
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Home,
        column: 3, // Middle
        base_cost: 0.0,
    },
    Key {
        idx: 8,
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Home,
        column: 4, // Index
        base_cost: 0.0,
    },
    Key {
        idx: 9,
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Home,
        column: 5, // Index side (toward center)
        base_cost: 1.5,
    },
    // Bottom row: ring, middle, index
    Key {
        idx: 10,
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Bottom,
        column: 2, // Ring
        base_cost: 2.5,
    },
    Key {
        idx: 11,
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Bottom,
        column: 3, // Middle
        base_cost: 1.5,
    },
    Key {
        idx: 12,
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Bottom,
        column: 4, // Index
        base_cost: 1.6,
    },
    // Right hand (13 keys) - mirrored
    // Index-side, index, middle, ring on top
    Key {
        idx: 13,
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Top,
        column: 5, // Index side (toward center)
        base_cost: 2.1,
    },
    Key {
        idx: 14,
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Top,
        column: 4, // Index
        base_cost: 1.5,
    },
    Key {
        idx: 15,
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Top,
        column: 3, // Middle
        base_cost: 2.0,
    },
    Key {
        idx: 16,
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Top,
        column: 2, // Ring
        base_cost: 3.0,
    },
    // Home row: index-left, index, middle, ring
    Key {
        idx: 17,
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Home,
        column: 5, // Index side (toward center)
        base_cost: 1.5,
    },
    Key {
        idx: 18,
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Home,
        column: 4, // Index
        base_cost: 0.0,
    },
    Key {
        idx: 19,
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Home,
        column: 3, // Middle
        base_cost: 0.0,
    },
    Key {
        idx: 20,
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Home,
        column: 2, // Ring
        base_cost: 0.5,
    },
    // Bottom row: index, middle, ring
    Key {
        idx: 21,
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Bottom,
        column: 4, // Index
        base_cost: 1.6,
    },
    Key {
        idx: 22,
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Bottom,
        column: 3, // Middle
        base_cost: 1.5,
    },
    Key {
        idx: 23,
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Bottom,
        column: 2, // Ring
        base_cost: 2.5,
    },
    // Pinky column (2 keys) - side position
    Key {
        idx: 24,
        hand: Hand::Right,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 1, // Pinky home
        base_cost: 1.0,
    },
    Key {
        idx: 25,
        hand: Hand::Right,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 0, // Pinky side (outer)
        base_cost: 2.0,
    },
];
