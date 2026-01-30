use crate::geometry::NUM_KEYS;
use crate::layout_26::Layout;

pub fn mirror_indices() -> [usize; NUM_KEYS] {
    // Mapping derived from geometry.rs
    // Left pinkies: 0<->25, 1<->24
    // Left top: 2<->16, 3<->15, 4<->14, 5<->13
    // Left home: 6<->20, 7<->19, 8<->18, 9<->17
    // Left bottom: 10<->23, 11<->22, 12<->21
    [
        25, 24, 16, 15, 14, 13, 20, 19, 18, 17, 23, 22, 21, 5, 4, 3, 2, 9, 8, 7, 6, 12, 11, 10, 1,
        0,
    ]
}

pub fn mirror_layout(layout: &Layout) -> Layout {
    let m = mirror_indices();
    let positions = layout.positions;
    // positions maps position -> letter; mirror by moving letters to mirrored positions
    let mut mirrored = positions;
    for i in 0..NUM_KEYS {
        mirrored[m[i]] = positions[i];
    }
    Layout {
        positions: mirrored,
    }
}

pub fn layout_compact_string(layout: &Layout) -> String {
    layout.positions.iter().collect()
}

pub fn canonical_key(layout: &Layout) -> String {
    let s1 = layout_compact_string(layout);
    let ml = mirror_layout(layout);
    let s2 = layout_compact_string(&ml);
    if s1 <= s2 {
        s1
    } else {
        s2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout_26::Layout;

    #[test]
    fn test_mirror_twice_equals_original() {
        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);
        let double_mirrored = mirror_layout(&mirrored);
        assert_eq!(
            layout_compact_string(&layout),
            layout_compact_string(&double_mirrored)
        );
    }

    #[test]
    fn test_canonical_key_same_for_mirrors() {
        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);
        assert_eq!(canonical_key(&layout), canonical_key(&mirrored));
    }

    #[test]
    fn test_specific_mirror_mappings() {
        let layout = Layout::alphabetical();
        let mirrored = mirror_layout(&layout);

        // Check specific mappings from the plan
        // Left pinky 0 (a) <-> Right pinky 25 (z)
        assert_eq!(layout.positions[0], mirrored.positions[25]);
        assert_eq!(layout.positions[25], mirrored.positions[0]);

        // Left pinky 1 (b) <-> Right pinky 24 (y)
        assert_eq!(layout.positions[1], mirrored.positions[24]);
        assert_eq!(layout.positions[24], mirrored.positions[1]);

        // Left top 2 (c) <-> Right top 16 (q)
        assert_eq!(layout.positions[2], mirrored.positions[16]);
        assert_eq!(layout.positions[16], mirrored.positions[2]);
    }
}
