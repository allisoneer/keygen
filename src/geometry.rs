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
    pub hand: Hand,
    pub finger: Finger,
    pub row: Row,
    pub column: u8, // 0=outer edge, increasing towards center
    pub base_cost: f64,
}

// Generate geometry array
pub static GEOMETRY: [Key; NUM_KEYS] = [
    // Left hand (13 keys)
    // Pinky column (2 keys) - side position
    Key {
        hand: Hand::Left,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 0, // Pinky side (outer)
        base_cost: 2.0,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 1, // Pinky home
        base_cost: 1.0,
    },
    // Top row: ring, middle, index, index-side
    Key {
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Top,
        column: 2, // Ring
        base_cost: 3.0,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Top,
        column: 3, // Middle
        base_cost: 2.0,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Top,
        column: 4, // Index
        base_cost: 1.5,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Top,
        column: 5, // Index side (toward center)
        base_cost: 2.1,
    },
    // Home row: ring, middle, index, index-right
    Key {
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Home,
        column: 2, // Ring
        base_cost: 0.5,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Home,
        column: 3, // Middle
        base_cost: 0.0,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Home,
        column: 4, // Index
        base_cost: 0.0,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Home,
        column: 5, // Index side (toward center)
        base_cost: 1.5,
    },
    // Bottom row: ring, middle, index
    Key {
        hand: Hand::Left,
        finger: Finger::Ring,
        row: Row::Bottom,
        column: 2, // Ring
        base_cost: 2.5,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Middle,
        row: Row::Bottom,
        column: 3, // Middle
        base_cost: 1.5,
    },
    Key {
        hand: Hand::Left,
        finger: Finger::Index,
        row: Row::Bottom,
        column: 4, // Index
        base_cost: 1.6,
    },
    // Right hand (13 keys) - mirrored
    // Index-side, index, middle, ring on top
    Key {
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Top,
        column: 5, // Index side (toward center)
        base_cost: 2.1,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Top,
        column: 4, // Index
        base_cost: 1.5,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Top,
        column: 3, // Middle
        base_cost: 2.0,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Top,
        column: 2, // Ring
        base_cost: 3.0,
    },
    // Home row: index-left, index, middle, ring
    Key {
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Home,
        column: 5, // Index side (toward center)
        base_cost: 1.5,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Home,
        column: 4, // Index
        base_cost: 0.0,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Home,
        column: 3, // Middle
        base_cost: 0.0,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Home,
        column: 2, // Ring
        base_cost: 0.5,
    },
    // Bottom row: index, middle, ring
    Key {
        hand: Hand::Right,
        finger: Finger::Index,
        row: Row::Bottom,
        column: 4, // Index
        base_cost: 1.6,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Middle,
        row: Row::Bottom,
        column: 3, // Middle
        base_cost: 1.5,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Ring,
        row: Row::Bottom,
        column: 2, // Ring
        base_cost: 2.5,
    },
    // Pinky column (2 keys) - side position
    Key {
        hand: Hand::Right,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 1, // Pinky home
        base_cost: 1.0,
    },
    Key {
        hand: Hand::Right,
        finger: Finger::Pinky,
        row: Row::Home,
        column: 0, // Pinky side (outer)
        base_cost: 2.0,
    },
];
