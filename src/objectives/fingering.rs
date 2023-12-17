use std::collections::HashSet;

type FingeringSet = HashSet<(char, char)>;
type Layout = [Vec<char>; 4];

#[derive(Debug)]
pub struct FingeringTypes {
    same_hand: FingeringSet,
    same_finger_large_jump: FingeringSet,
    same_finger_small_jump: FingeringSet,
    little_finger_interference: FingeringSet,
    awkward_upside_down: FingeringSet,
}

pub fn get_partial_fingering_types(layout: &Layout) -> FingeringTypes {
    let map_char_index_to_finger: [usize; 7] = [2, 2, 3, 4, 5, 5, 5];
    let is_long_finger = |x: usize| x == 3 || x == 4;
    let is_short_finger = |x: usize| x == 2 || x == 5;
    let mut same_hand = FingeringSet::new();
    let mut same_finger_large_jump = FingeringSet::new();
    let mut same_finger_small_jump = FingeringSet::new();
    let mut little_finger_interference = FingeringSet::new();
    let mut awkward_upside_down = FingeringSet::new();
    for (row1, content1) in layout.iter().enumerate() {
        for (row2, content2) in layout.iter().enumerate() {
            for (column1, char1) in content1.iter().enumerate() {
                for (column2, char2) in content2.iter().enumerate() {
                    let pair = (*char1, *char2);
                    same_hand.insert(pair);
                    let finger1 = map_char_index_to_finger[column1];
                    let finger2 = map_char_index_to_finger[column2];
                    let row_diff = row1.abs_diff(row2);
                    if finger1 == finger2 {
                        if row_diff >= 2 {
                            same_finger_large_jump.insert(pair);
                        } else if row_diff == 1 {
                            same_finger_small_jump.insert(pair);
                        }
                    }
                    if (finger1 == 5 && finger2 >= 3)
                        || (finger2 == 5 && finger1 >= 3)
                    {
                        little_finger_interference.insert(pair);
                    }
                    // 短指击上排，长指击下排
                    let awkward1 = row1 < row2 && is_short_finger(finger1) && is_long_finger(finger2);
                    // 长指击下排，短指击上排
                    let awkward2 = row1 > row2 && is_long_finger(finger1) && is_short_finger(finger2);
                    if (awkward1 || awkward2) && row_diff >= 2 {
                        awkward_upside_down.insert(pair);
                    }
                }
            }
        }
    }
    FingeringTypes {
        same_hand,
        same_finger_large_jump,
        same_finger_small_jump,
        little_finger_interference,
        awkward_upside_down,
    }
}

pub fn get_fingering_types() -> FingeringTypes {
    let left_layout: Layout = [
        vec!['5', '4', '3', '2', '1'],
        vec!['t', 'r', 'e', 'w', 'q'],
        vec!['g', 'f', 'd', 's', 'a'],
        vec!['b', 'v', 'c', 'x', 'z'],
    ];
    let right_layout: Layout = [
        vec!['6', '7', '8', '9', '0', '-', '='],
        vec!['y', 'u', 'i', 'o', 'p', '[', ']'],
        vec!['h', 'j', 'k', 'l', ';', '\''],
        vec!['n', 'm', ',', '.', '/'],
    ];
    let mut left_types = get_partial_fingering_types(&left_layout);
    let right_types = get_partial_fingering_types(&right_layout);
    left_types.same_hand.extend(right_types.same_hand);
    left_types
        .same_finger_large_jump
        .extend(right_types.same_finger_large_jump);
    left_types
        .same_finger_small_jump
        .extend(right_types.same_finger_small_jump);
    left_types
        .little_finger_interference
        .extend(right_types.little_finger_interference);
    left_types
        .awkward_upside_down
        .extend(right_types.awkward_upside_down);
    left_types
}
