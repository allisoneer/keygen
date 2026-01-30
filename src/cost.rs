use crate::geometry::{Finger, Hand, Key, Row};
use crate::layout_26::KeyInfo;

/// Cost/penalty calculation for 26-key layouts
/// Configuration for penalty weights
#[derive(Clone)]
pub struct Config {
    pub base_weight: f64,
    pub same_finger_bigram: f64,
    pub same_hand_bigram: f64,
    pub alt_hand_bigram: f64,       // NEW: reward per alternation bigram
    pub movement_penalty_base: f64, // penalty per unit of distance
    pub extreme_movement_threshold: f64, // distance threshold for extra penalty
    pub extreme_movement_extra: f64, // additional penalty for extreme movements
    pub pinky_ring_twist: f64,
    pub roll_reversal: f64,
    pub same_hand_4: f64,
    pub alternating_hand_4: f64,
    pub roll_out: f64,
    pub roll_in: f64,
    pub broken_roll: f64,
    pub sandwich_distance_penalty: f64, // penalty for sandwich patterns scaled by distance
    pub same_hand_finger_repeat: f64,   // penalty for using same finger twice in same-hand sequence
    pub twist: f64,
    // NEW: hand-balance penalty configuration
    pub balance_mild_start: f64,      // e.g., 0.50
    pub balance_moderate_start: f64,  // e.g., 0.52
    pub balance_severe_start: f64,    // e.g., 0.54
    pub balance_mild_weight: f64,     // per-letter scaled weight
    pub balance_moderate_weight: f64, // "
    pub balance_severe_weight: f64,   // "
    pub balance_severe_exponent: f64, // e.g., 3.0
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base_weight: 1.0,         // increased from 1.0 to prioritize key placement
            same_finger_bigram: 20.0, // increased from 5.0 to strongly avoid same-finger sequences
            same_hand_bigram: 2.0,
            alt_hand_bigram: -2.0, // symmetry: reward magnitude equals penalty
            movement_penalty_base: 0.0, // 3 penalty per unit distance
            extreme_movement_threshold: 3.5, // movements > 3 units are extreme
            extreme_movement_extra: 5.0, // +5 penalty for extreme movements
            pinky_ring_twist: 10.0,
            roll_reversal: 20.0,
            same_hand_4: 3.0,
            alternating_hand_4: -0.5, // set to 0 to avoid double counting;
            // can re-enable as small synergy if desired
            roll_out: -1.0,
            roll_in: -1.5,
            broken_roll: 4.0,
            sandwich_distance_penalty: 2.0, // scaled by total distance
            same_hand_finger_repeat: 15.0,  // severe penalty for finger returns
            twist: 10.0,
            balance_mild_start: 0.56, // changed from 0.50 - only start penalizing after 56%
            balance_moderate_start: 0.58, // changed from 0.52
            balance_severe_start: 0.59, // changed from 0.54
            balance_mild_weight: 2.0, // reduced from 10.0
            balance_moderate_weight: 10.0, // reduced from 50.0
            balance_severe_weight: 100.0, // reduced from 500.0
            balance_severe_exponent: 3.0,
        }
    }
}

pub struct PenaltyComponent {
    pub name: &'static str,
    pub total: f64,
}

pub fn score_all(
    quartads: &std::collections::HashMap<[u8; 4], usize>,
    monograms: &[usize; 26],
    letters_total: usize,
    layout: &crate::layout_26::Layout,
    cfg: &Config,
) -> (f64, Vec<PenaltyComponent>) {
    let mut total = 0.0;
    let mut components = vec![
        PenaltyComponent {
            name: "base",
            total: 0.0,
        },
        PenaltyComponent {
            name: "same_finger_bigram",
            total: 0.0,
        },
        PenaltyComponent {
            name: "same_hand_bigram",
            total: 0.0,
        },
        PenaltyComponent {
            name: "movement_distance",
            total: 0.0,
        },
        PenaltyComponent {
            name: "extreme_movement",
            total: 0.0,
        },
        PenaltyComponent {
            name: "pinky_ring_twist",
            total: 0.0,
        },
        PenaltyComponent {
            name: "roll_reversal",
            total: 0.0,
        },
        PenaltyComponent {
            name: "same_hand_4",
            total: 0.0,
        },
        PenaltyComponent {
            name: "alternating_hand_4",
            total: 0.0,
        },
        PenaltyComponent {
            name: "roll_out",
            total: 0.0,
        },
        PenaltyComponent {
            name: "roll_in",
            total: 0.0,
        },
        PenaltyComponent {
            name: "sandwich_distance",
            total: 0.0,
        },
        PenaltyComponent {
            name: "twist",
            total: 0.0,
        },
    ];
    // Append new components at the end, keeping all existing indices stable
    let alt_idx = components.len();
    components.push(PenaltyComponent {
        name: "alt_hand_bigram",
        total: 0.0,
    });
    let balance_idx = components.len();
    components.push(PenaltyComponent {
        name: "hand_balance",
        total: 0.0,
    });
    let broken_roll_idx = components.len();
    components.push(PenaltyComponent {
        name: "broken_roll",
        total: 0.0,
    });
    let _finger_repeat_idx = components.len();
    components.push(PenaltyComponent {
        name: "same_hand_finger_repeat",
        total: 0.0,
    });

    // Index variables for roll penalties with debug assertions
    let roll_out_idx = 9;
    let roll_in_idx = 10;
    debug_assert_eq!(components[roll_out_idx].name, "roll_out");
    debug_assert_eq!(components[roll_in_idx].name, "roll_in");

    for (quartad, &count_usize) in quartads {
        let count = count_usize as f64;

        // Convert quartad codes to KeyInfo
        let keys: Vec<Option<KeyInfo>> = quartad
            .iter()
            .map(|&code| {
                let letter = (b'a' + code) as char;
                layout.get_key_info(letter)
            })
            .collect();

        // Skip if any key not found (shouldn't happen with valid quartads)
        if keys.iter().any(|k| k.is_none()) {
            continue;
        }

        let keys: Vec<KeyInfo> = keys.into_iter().map(|k| k.unwrap()).collect();

        // Base penalty (all 4 keys)
        for key in &keys {
            let penalty = key.base_cost * cfg.base_weight * count;
            components[0].total += penalty;
            total += penalty;
        }

        // Two-key penalties (3 bigrams in quartad)
        for i in 1..4 {
            let curr = &keys[i];
            let prev = &keys[i - 1];

            // Same finger bigram
            if curr.finger == prev.finger
                && curr.hand == prev.hand
                && curr.position != prev.position
            {
                let penalty = cfg.same_finger_bigram * count;
                components[1].total += penalty;
                total += penalty;
            }

            // Same hand bigram
            if curr.hand == prev.hand {
                let penalty = cfg.same_hand_bigram * count;
                components[2].total += penalty;
                total += penalty;
            }

            // NEW: Alternating hand bigram reward (symmetry to same_hand_bigram)
            if curr.hand != prev.hand {
                let reward = cfg.alt_hand_bigram * count; // expected negative
                components[alt_idx].total += reward;
                total += reward;
            }

            // Distance-based movement penalties
            let distance = keyinfo_distance(curr, prev);
            if distance > 1.8 {
                // Only penalize movements beyond diagonal
                // Base movement penalty scaled by distance
                let base_penalty = cfg.movement_penalty_base * distance * count;
                components[3].total += base_penalty;
                total += base_penalty;

                // Extra penalty for extreme movements
                if distance > cfg.extreme_movement_threshold {
                    let extreme_penalty = cfg.extreme_movement_extra * count;
                    components[4].total += extreme_penalty;
                    total += extreme_penalty;
                }
            }

            // Pinky/ring twist (clarified parentheses)
            #[allow(clippy::nonminimal_bool)]
            // explicit parentheses for clarity, not simplification
            if curr.hand == prev.hand
                && ((curr.finger == Finger::Ring
                    && prev.finger == Finger::Pinky
                    && ((curr.row == Row::Home && prev.row == Row::Top)
                        || (curr.row == Row::Bottom && prev.row == Row::Top)))
                    || (curr.finger == Finger::Pinky
                        && prev.finger == Finger::Ring
                        && ((curr.row == Row::Top && prev.row == Row::Home)
                            || (curr.row == Row::Top && prev.row == Row::Bottom))))
            {
                let penalty = cfg.pinky_ring_twist * count;
                components[5].total += penalty;
                total += penalty;
            }

            // Roll out - only reward if same row
            if curr.hand == prev.hand
                && curr.row == prev.row
                && is_roll_out(curr.hand, curr.finger, prev.finger)
            {
                let penalty = cfg.roll_out * count;
                components[roll_out_idx].total += penalty;
                total += penalty;
            }

            // Roll in - only reward if same row
            if curr.hand == prev.hand
                && curr.row == prev.row
                && is_roll_in(curr.hand, curr.finger, prev.finger)
            {
                let penalty = cfg.roll_in * count;
                components[roll_in_idx].total += penalty;
                total += penalty;
            }

            // Penalize roll patterns that are too far apart
            if curr.hand == prev.hand
                && curr.finger != prev.finger
                && (is_roll_out(curr.hand, curr.finger, prev.finger)
                    || is_roll_in(curr.hand, curr.finger, prev.finger))
            {
                let broken_roll_distance = keyinfo_distance(curr, prev);
                if broken_roll_distance >= 1.85 {
                    // Too far to be a smooth roll
                    let penalty = cfg.broken_roll * count;
                    components[broken_roll_idx].total += penalty;
                    total += penalty;
                }
            }
        }

        // Three-key penalties
        if keys.len() >= 3 {
            // Roll reversal
            for i in 2..4 {
                let curr = &keys[i];
                let prev1 = &keys[i - 1];
                let prev2 = &keys[i - 2];

                if curr.hand == prev1.hand
                    && prev1.hand == prev2.hand
                    && ((curr.finger == Finger::Middle
                        && prev1.finger == Finger::Pinky
                        && prev2.finger == Finger::Ring)
                        || (curr.finger == Finger::Ring
                            && prev1.finger == Finger::Pinky
                            && prev2.finger == Finger::Middle))
                {
                    let penalty = cfg.roll_reversal * count;
                    components[6].total += penalty;
                    total += penalty;
                }

                // Twist
                if curr.hand == prev1.hand
                    && prev1.hand == prev2.hand
                    && ((curr.row == Row::Top
                        && prev1.row == Row::Home
                        && prev2.row == Row::Bottom)
                        || (curr.row == Row::Bottom
                            && prev1.row == Row::Home
                            && prev2.row == Row::Top))
                    && ((is_roll_out(curr.hand, curr.finger, prev1.finger)
                        && is_roll_out(prev1.hand, prev1.finger, prev2.finger))
                        || (is_roll_in(curr.hand, curr.finger, prev1.finger)
                            && is_roll_in(prev1.hand, prev1.finger, prev2.finger)))
                {
                    let penalty = cfg.twist * count;
                    components[12].total += penalty;
                    total += penalty;
                }
            }

            // Sandwich distance penalty (A-B-A patterns)
            for i in 2..4 {
                let curr = &keys[i];
                let prev2 = &keys[i - 2];

                if curr.hand == prev2.hand {
                    let sandwich_distance = keyinfo_distance(curr, prev2);

                    // Only penalize significant distances (threshold of 2.0)
                    if sandwich_distance > 2.0 {
                        let penalty = cfg.sandwich_distance_penalty * sandwich_distance * count;
                        components[11].total += penalty;
                        total += penalty;
                    }
                }
            }
        }

        // Four-key penalties
        if keys.len() == 4 {
            // Same hand 4
            if keys[0].hand == keys[1].hand
                && keys[1].hand == keys[2].hand
                && keys[2].hand == keys[3].hand
            {
                let penalty = cfg.same_hand_4 * count;
                components[7].total += penalty;
                total += penalty;
            }

            // Alternating hand 4
            if keys[0].hand != keys[1].hand
                && keys[1].hand != keys[2].hand
                && keys[2].hand != keys[3].hand
            {
                let penalty = cfg.alternating_hand_4 * count;
                components[8].total += penalty;
                total += penalty;
            }

            // Same hand finger repeat - detect using same finger twice before hand switch
            let mut left_fingers_seen = Vec::new();
            let mut right_fingers_seen = Vec::new();
            let mut penalty_applied = false;

            for key in &keys {
                if key.hand == Hand::Left {
                    if left_fingers_seen.contains(&(key.finger, key.position)) {
                        // Same finger, same position is already handled by same_finger_bigram
                        continue;
                    }
                    if left_fingers_seen.iter().any(|(f, _)| *f == key.finger) && !penalty_applied {
                        // Same finger, different position on same hand!
                        let penalty = cfg.same_hand_finger_repeat * count;
                        components[13].total += penalty;
                        total += penalty;
                        penalty_applied = true;
                    }
                    left_fingers_seen.push((key.finger, key.position));
                    right_fingers_seen.clear(); // Other hand resets
                } else {
                    if right_fingers_seen.contains(&(key.finger, key.position)) {
                        continue;
                    }
                    if right_fingers_seen.iter().any(|(f, _)| *f == key.finger) && !penalty_applied
                    {
                        let penalty = cfg.same_hand_finger_repeat * count;
                        components[13].total += penalty;
                        total += penalty;
                        penalty_applied = true;
                    }
                    right_fingers_seen.push((key.finger, key.position));
                    left_fingers_seen.clear(); // Other hand resets
                }
            }
        }
    }

    // Global hand balance penalty (monogram-based)
    // Compute p_left/p_right by mapping letters to hands via layout and monograms.
    let letters_total_f = letters_total as f64;
    if letters_total > 0 {
        let mut left = 0usize;
        let mut right = 0usize;
        #[allow(clippy::needless_range_loop)] // clearer than enumerate for letter codes
        for code in 0..26usize {
            let letter = (b'a' + code as u8) as char;
            if let Some(info) = layout.get_key_info(letter) {
                if info.hand == Hand::Left {
                    left += monograms[code];
                } else {
                    right += monograms[code];
                }
            }
        }
        let p_left = (left as f64) / letters_total_f;
        let p_right = (right as f64) / letters_total_f;
        let hb = hand_balance_penalty(p_left, p_right, letters_total, cfg);
        components[balance_idx].total += hb;
        total += hb;
    }

    (total, components)
}

fn is_roll_out(hand: Hand, curr: Finger, prev: Finger) -> bool {
    match hand {
        Hand::Left => {
            // For left hand: pinky -> ring -> middle -> index is outward
            matches!(
                (prev, curr),
                (Finger::Pinky, Finger::Ring)
                    | (Finger::Pinky, Finger::Middle)
                    | (Finger::Pinky, Finger::Index)
                    | (Finger::Ring, Finger::Middle)
                    | (Finger::Ring, Finger::Index)
                    | (Finger::Middle, Finger::Index)
            )
        }
        Hand::Right => {
            // For right hand: index -> middle -> ring -> pinky is outward
            matches!(
                (prev, curr),
                (Finger::Index, Finger::Middle)
                    | (Finger::Index, Finger::Ring)
                    | (Finger::Index, Finger::Pinky)
                    | (Finger::Middle, Finger::Ring)
                    | (Finger::Middle, Finger::Pinky)
                    | (Finger::Ring, Finger::Pinky)
            )
        }
    }
}

fn is_roll_in(hand: Hand, curr: Finger, prev: Finger) -> bool {
    // Roll in is the opposite of roll out
    is_roll_out(hand, prev, curr)
}

/// Calculate distance between two KeyInfo structs using their geometry positions
fn keyinfo_distance(info1: &KeyInfo, info2: &KeyInfo) -> f64 {
    // Get the actual Key structs from geometry
    let key1 = &crate::geometry::GEOMETRY[info1.position];
    let key2 = &crate::geometry::GEOMETRY[info2.position];
    key_distance(key1, key2)
}

/// Calculate distance between two keys for movement penalty
fn key_distance(key1: &Key, key2: &Key) -> f64 {
    // Only calculate distance for same-hand movements
    if key1.hand != key2.hand {
        return 0.0;
    }

    // Calculate column distance with special handling for index positions
    let col_dist = if key1.finger == Finger::Index && key2.finger == Finger::Index {
        // Movement between index positions (home vs side) is only 0.5 distance
        0.5 * (key1.column as i32 - key2.column as i32).abs() as f64
    } else {
        (key1.column as i32 - key2.column as i32).abs() as f64
    };

    // Scale row distances to reflect physical keyboard layout
    // Rows are typically ~1.5x further apart than columns
    let row_dist = match (key1.row, key2.row) {
        (Row::Top, Row::Top) | (Row::Home, Row::Home) | (Row::Bottom, Row::Bottom) => 0.0,
        (Row::Top, Row::Home)
        | (Row::Home, Row::Top)
        | (Row::Home, Row::Bottom)
        | (Row::Bottom, Row::Home) => 1.5,
        (Row::Top, Row::Bottom) | (Row::Bottom, Row::Top) => 3.0,
    };

    // Use Euclidean distance for diagonal movements
    (col_dist * col_dist + row_dist * row_dist).sqrt()
}

fn hand_balance_penalty(p_left: f64, p_right: f64, letters_total: usize, cfg: &Config) -> f64 {
    if letters_total == 0 {
        return 0.0;
    }
    let r = p_left.max(p_right); // dominant hand usage fraction [0,1]

    let s0 = cfg.balance_mild_start;
    let s1 = cfg.balance_moderate_start;
    let s2 = cfg.balance_severe_start;

    if r <= s0 {
        return 0.0;
    }

    // Build a continuous, piecewise penalty. Each segment contributes additively.
    let mut penalty_per_letter = 0.0;

    // Mild: quadratic growth from s0 to s1
    if r > s0 {
        let x = ((r.min(s1) - s0) / (s1 - s0)).max(0.0);
        penalty_per_letter += cfg.balance_mild_weight * x * x;
    }

    // Moderate: quadratic growth from s1 to s2
    if r > s1 {
        let x = ((r.min(s2) - s1) / (s2 - s1)).max(0.0);
        penalty_per_letter += cfg.balance_moderate_weight * x * x;
    }

    // Severe: polynomial growth beyond s2
    if r > s2 {
        let x = (r - s2) / (1.0 - s2); // normalized to [0,1] over the remaining space
        penalty_per_letter += cfg.balance_severe_weight * x.powf(cfg.balance_severe_exponent);
    }

    penalty_per_letter * (letters_total as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_26::Layout;
    use std::collections::HashMap;

    fn simple_config() -> Config {
        Config {
            // isolate hand transition and balance for tests
            base_weight: 0.0,
            same_finger_bigram: 0.0,
            same_hand_bigram: 2.0,
            alt_hand_bigram: -2.0,
            movement_penalty_base: 0.0,
            extreme_movement_threshold: 3.0,
            extreme_movement_extra: 0.0,
            pinky_ring_twist: 0.0,
            roll_reversal: 0.0,
            same_hand_4: 0.0,
            alternating_hand_4: 0.0,
            roll_out: 0.0,
            roll_in: 0.0,
            sandwich_distance_penalty: 0.0,
            twist: 0.0,
            same_hand_finger_repeat: 0.0,
            broken_roll: 0.0,
            balance_mild_start: 0.50,
            balance_moderate_start: 0.52,
            balance_severe_start: 0.54,
            balance_mild_weight: 0.0,
            balance_moderate_weight: 0.0,
            balance_severe_weight: 0.0,
            balance_severe_exponent: 3.0,
        }
    }

    #[test]
    fn test_per_bigram_alternation_symmetry() {
        // Quartad "abab" -> 3 transitions: a-b, b-a, a-b
        let mut quartads = HashMap::new();
        quartads.insert([0, 1, 0, 1], 1);

        let cfg = simple_config();
        let layout = Layout::alphabetical(); // 'a' left, 'b' left initially

        // Case 1: both on left -> 3 same-hand penalties, 0 alternation rewards
        let monograms = {
            let mut m = [0usize; 26];
            m[0] = 2;
            m[1] = 2;
            m
        };
        let letters_total = 4;

        let (total_same, comps_same) =
            score_all(&quartads, &monograms, letters_total, &layout, &cfg);
        let same_hand_bigram_total = comps_same
            .iter()
            .find(|c| c.name == "same_hand_bigram")
            .unwrap()
            .total;
        let alt_total = comps_same
            .iter()
            .find(|c| c.name == "alt_hand_bigram")
            .unwrap()
            .total;
        assert_eq!(same_hand_bigram_total, 3.0 * 2.0);
        assert_eq!(alt_total, 0.0);

        // Case 2: move 'b' to right hand -> 3 alternations, 0 same-hand
        let mut layout2 = layout.clone();
        // swap 'b' (pos1) with a known right-hand key (e.g., pos18 is right index home in geometry.rs)
        layout2.swap(1, 18);

        let (total_alt, comps_alt) =
            score_all(&quartads, &monograms, letters_total, &layout2, &cfg);
        let same2 = comps_alt
            .iter()
            .find(|c| c.name == "same_hand_bigram")
            .unwrap()
            .total;
        let alt2 = comps_alt
            .iter()
            .find(|c| c.name == "alt_hand_bigram")
            .unwrap()
            .total;
        assert_eq!(same2, 0.0);
        assert_eq!(alt2, 3.0 * (-2.0));
        assert!(total_alt < total_same);
    }

    #[test]
    fn test_hand_balance_penalty_regions() {
        // Use defaults to stay aligned with current tuning and avoid freezing values
        let cfg = Config::default();
        let letters_total = 1000usize;

        let s0 = cfg.balance_mild_start;
        let s1 = cfg.balance_moderate_start;
        let s2 = cfg.balance_severe_start;

        // Exactly at mild start -> zero penalty
        let p_at_s0 = hand_balance_penalty(s0, 1.0 - s0, letters_total, &cfg);
        assert_eq!(p_at_s0, 0.0);

        // Just above mild start
        let eps = (s1 - s0) * 0.05; // 5% into the mild region
        let p_mild = hand_balance_penalty(s0 + eps, 1.0 - (s0 + eps), letters_total, &cfg);
        assert!(p_mild > 0.0);

        // In moderate region (midpoint between s1 and s2)
        let mid_mod = s1 + (s2 - s1) * 0.5;
        let p_mod = hand_balance_penalty(mid_mod, 1.0 - mid_mod, letters_total, &cfg);
        assert!(p_mod > p_mild);

        // In severe region: comfortably beyond s2
        let severe_point = (s2 + 0.06).min(0.99);
        let p_sev = hand_balance_penalty(severe_point, 1.0 - severe_point, letters_total, &cfg);
        assert!(p_sev > p_mod);
    }

    #[test]
    fn test_mirror_invariance() {
        use crate::symmetry::mirror_layout;

        // Create a sample corpus
        let mut quartads = HashMap::new();
        quartads.insert([0, 1, 2, 3], 10); // abcd
        quartads.insert([4, 5, 6, 7], 15); // efgh
        quartads.insert([0, 4, 0, 4], 20); // aeae

        let mut monograms = [0usize; 26];
        #[allow(clippy::needless_range_loop)]
        for i in 0..8 {
            monograms[i] = 100 + i;
        }
        let letters_total = monograms.iter().sum();

        // Symmetric roll weights for this test only
        let mut cfg = Config::default();
        // Make inward/outward rolls equally rewarded to ensure mirror symmetry
        let equal_roll = (cfg.roll_out + cfg.roll_in) / 2.0;
        cfg.roll_out = equal_roll;
        cfg.roll_in = equal_roll;

        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);

        let (penalty1, _) = score_all(&quartads, &monograms, letters_total, &layout, &cfg);
        let (penalty2, _) = score_all(&quartads, &monograms, letters_total, &mirrored, &cfg);

        assert!(
            (penalty1 - penalty2).abs() < 1e-9,
            "Mirror invariance should hold under symmetric roll weights: {} != {}",
            penalty1,
            penalty2
        );
    }

    #[test]
    fn test_mirror_non_invariance_with_asym_rolls() {
        use crate::symmetry::mirror_layout;

        let mut quartads = HashMap::new();
        quartads.insert([0, 1, 2, 3], 10);
        quartads.insert([4, 5, 6, 7], 15);
        quartads.insert([0, 4, 0, 4], 20);

        let mut monograms = [0usize; 26];
        #[allow(clippy::needless_range_loop)]
        for i in 0..8 {
            monograms[i] = 100 + i;
        }
        let letters_total = monograms.iter().sum();

        // Default config uses asymmetric roll rewards by design (inward stronger than outward)
        let cfg = Config::default();
        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);

        let (penalty1, _) = score_all(&quartads, &monograms, letters_total, &layout, &cfg);
        let (penalty2, _) = score_all(&quartads, &monograms, letters_total, &mirrored, &cfg);

        assert!(
            (penalty1 - penalty2).abs() > 1e-9,
            "With asymmetric roll weights, mirror invariance is not expected"
        );
    }
}
