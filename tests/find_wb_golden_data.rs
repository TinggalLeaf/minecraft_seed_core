//! 由 reference/site/gen_find_tests.mjs 自动生成（勿手改）。
#![allow(dead_code)]
pub struct FindWbCase { pub mc: i32, pub stype: i32, pub biomes: &'static [i32], pub x: i32, pub z: i32, pub range: i32, pub start: i64, pub dim: i32, pub y_height: i32, pub expect: i64 }
const WB_IDS_0: &[i32] = &[1];
const WB_IDS_1: &[i32] = &[1,4];
const WB_IDS_2: &[i32] = &[1];
const WB_IDS_3: &[i32] = &[2];
pub static FIND_WB_CASES: &[FindWbCase] = &[
    FindWbCase { mc: 25, stype: 5, biomes: WB_IDS_0, x: 0, z: 0, range: 16, start: 0, dim: 0, y_height: 320, expect: 5348024557502692 },
    FindWbCase { mc: 25, stype: 5, biomes: WB_IDS_1, x: 0, z: 0, range: 32, start: 0, dim: 0, y_height: 320, expect: 17732923532771552 },
    FindWbCase { mc: 28, stype: 5, biomes: WB_IDS_2, x: 0, z: 0, range: 16, start: 7, dim: 0, y_height: 320, expect: 5348024557502692 },
    FindWbCase { mc: 15, stype: 1, biomes: WB_IDS_3, x: 0, z: 0, range: 16, start: 0, dim: 0, y_height: 320, expect: 2814749767106739 },
];
