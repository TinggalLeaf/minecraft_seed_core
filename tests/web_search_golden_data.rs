//! 由 `reference/site/gen_find_tests.mjs` 自动生成（勿手改）。
//! find_biomes 对拍数据：网站 api.wasm 输出 + f_uc A 掩码（bytes 32..47）。
#![allow(dead_code)]

pub struct FindBiomesCase {
    pub mc: i32,
    pub ids: &'static [i32],
    pub x: i32, pub z: i32, pub w: i32, pub h: i32,
    pub start: i64, pub dim: i32, pub y_height: i32,
    /// 网站 api.wasm find_biomes 结果（-1 表示 25s 超时未找到）。
    pub expect: i64,
}

/// f_uc 单 id 解析的 A 掩码（word1 低 32 / word1 高 32 / word2 低 32 / word2 高 32）。
/// 按 (id, [u32; 4]) 排列，未列出的 id 掩码全零。
pub static A_MASKS: &[(i32, [u32; 4])] = &[
    (1, [2, 0, 0, 0]),
    (3, [8, 0, 0, 0]),
    (4, [16, 0, 0, 0]),
    (5, [32, 0, 0, 0]),
    (6, [64, 0, 0, 0]),
    (11, [4096, 0, 0, 0]),
    (12, [4096, 0, 0, 0]),
    (13, [8192, 0, 0, 0]),
    (14, [16384, 0, 0, 0]),
    (17, [131072, 0, 0, 0]),
    (18, [262144, 0, 0, 0]),
    (19, [524288, 0, 0, 0]),
    (21, [2097152, 0, 0, 0]),
    (22, [4194304, 0, 0, 0]),
    (24, [16777216, 0, 0, 0]),
    (27, [134217728, 0, 0, 0]),
    (28, [268435456, 0, 0, 0]),
    (29, [536870912, 0, 0, 0]),
    (30, [1073741824, 0, 0, 0]),
    (31, [2147483648, 0, 0, 0]),
    (32, [0, 1, 0, 0]),
    (33, [0, 2, 0, 0]),
    (34, [0, 4, 0, 0]),
    (35, [0, 8, 0, 0]),
    (36, [0, 16, 0, 0]),
    (37, [0, 32, 0, 0]),
    (38, [0, 64, 0, 0]),
    (39, [0, 128, 0, 0]),
    (47, [16777216, 0, 0, 0]),
    (48, [16777216, 0, 0, 0]),
    (49, [16777216, 0, 0, 0]),
    (50, [16777216, 0, 0, 0]),
    (129, [0, 0, 2, 0]),
    (130, [0, 0, 4, 0]),
    (131, [0, 0, 8, 0]),
    (132, [0, 0, 16, 0]),
    (133, [0, 0, 32, 0]),
    (134, [0, 0, 64, 0]),
    (140, [0, 0, 4096, 0]),
    (149, [0, 0, 2097152, 0]),
    (151, [0, 0, 8388608, 0]),
    (155, [0, 0, 134217728, 0]),
    (156, [0, 0, 268435456, 0]),
    (157, [0, 0, 536870912, 0]),
    (158, [0, 0, 1073741824, 0]),
    (160, [0, 0, 0, 1]),
    (161, [0, 0, 0, 2]),
    (162, [0, 0, 0, 4]),
    (163, [0, 0, 0, 8]),
    (164, [0, 0, 0, 16]),
    (165, [0, 0, 0, 32]),
    (166, [0, 0, 0, 64]),
    (167, [0, 0, 0, 128]),
    (168, [0, 0, 0, 256]),
    (169, [0, 0, 0, 512]),
];

const IDS_0: &[i32] = &[1];
const IDS_1: &[i32] = &[1,4];
const IDS_2: &[i32] = &[14];
const IDS_3: &[i32] = &[35];
const IDS_4: &[i32] = &[21,37];
const IDS_5: &[i32] = &[1,4,5];
const IDS_6: &[i32] = &[37,38];
const IDS_7: &[i32] = &[1];
const IDS_8: &[i32] = &[1,4];
const IDS_9: &[i32] = &[14];
const IDS_10: &[i32] = &[35];
const IDS_11: &[i32] = &[21,37];
const IDS_12: &[i32] = &[1,4,5];
const IDS_13: &[i32] = &[37,38];
const IDS_14: &[i32] = &[1];
const IDS_15: &[i32] = &[1,4];
const IDS_16: &[i32] = &[14];
const IDS_17: &[i32] = &[35];
const IDS_18: &[i32] = &[21,37];
const IDS_19: &[i32] = &[1,4,5];
const IDS_20: &[i32] = &[37,38];
const IDS_21: &[i32] = &[1];
const IDS_22: &[i32] = &[1,4];
const IDS_23: &[i32] = &[14];
const IDS_24: &[i32] = &[35];
const IDS_25: &[i32] = &[21,37];
const IDS_26: &[i32] = &[1,4,5];
const IDS_27: &[i32] = &[37,38];
const IDS_28: &[i32] = &[1];
const IDS_29: &[i32] = &[1,4];
const IDS_30: &[i32] = &[14];
const IDS_31: &[i32] = &[35];
const IDS_32: &[i32] = &[21,37];
const IDS_33: &[i32] = &[1,4,5];
const IDS_34: &[i32] = &[37,38];
const IDS_35: &[i32] = &[1];
const IDS_36: &[i32] = &[1,4];
const IDS_37: &[i32] = &[14];
const IDS_38: &[i32] = &[35];
const IDS_39: &[i32] = &[21,37];
const IDS_40: &[i32] = &[1,4,5];
const IDS_41: &[i32] = &[37,38];
const IDS_42: &[i32] = &[129];
const IDS_43: &[i32] = &[44];
const IDS_44: &[i32] = &[35];
const IDS_45: &[i32] = &[1,4];
const IDS_46: &[i32] = &[170];
const IDS_47: &[i32] = &[171,172];

pub static FIND_BIOMES_CASES: &[FindBiomesCase] = &[
    FindBiomesCase { mc: 10, ids: IDS_0, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 16 },
    FindBiomesCase { mc: 10, ids: IDS_1, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 10, ids: IDS_2, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 489 },
    FindBiomesCase { mc: 10, ids: IDS_3, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 43 },
    FindBiomesCase { mc: 10, ids: IDS_4, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 53263 },
    FindBiomesCase { mc: 10, ids: IDS_5, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 3 },
    FindBiomesCase { mc: 10, ids: IDS_6, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 84 },
    FindBiomesCase { mc: 15, ids: IDS_7, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 16 },
    FindBiomesCase { mc: 15, ids: IDS_8, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 15, ids: IDS_9, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 489 },
    FindBiomesCase { mc: 15, ids: IDS_10, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 43 },
    FindBiomesCase { mc: 15, ids: IDS_11, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 53263 },
    FindBiomesCase { mc: 15, ids: IDS_12, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 3 },
    FindBiomesCase { mc: 15, ids: IDS_13, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 84 },
    FindBiomesCase { mc: 20, ids: IDS_14, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 16 },
    FindBiomesCase { mc: 20, ids: IDS_15, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 20, ids: IDS_16, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 489 },
    FindBiomesCase { mc: 20, ids: IDS_17, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 43 },
    FindBiomesCase { mc: 20, ids: IDS_18, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 120561 },
    FindBiomesCase { mc: 20, ids: IDS_19, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 3 },
    FindBiomesCase { mc: 20, ids: IDS_20, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 84 },
    FindBiomesCase { mc: 22, ids: IDS_21, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 9 },
    FindBiomesCase { mc: 22, ids: IDS_22, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 74 },
    FindBiomesCase { mc: 22, ids: IDS_23, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 262 },
    FindBiomesCase { mc: 22, ids: IDS_24, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 0 },
    FindBiomesCase { mc: 22, ids: IDS_25, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 58944 },
    FindBiomesCase { mc: 22, ids: IDS_26, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 209 },
    FindBiomesCase { mc: 22, ids: IDS_27, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 25, ids: IDS_28, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 9 },
    FindBiomesCase { mc: 25, ids: IDS_29, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 74 },
    FindBiomesCase { mc: 25, ids: IDS_30, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 262 },
    FindBiomesCase { mc: 25, ids: IDS_31, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 0 },
    FindBiomesCase { mc: 25, ids: IDS_32, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 58944 },
    FindBiomesCase { mc: 25, ids: IDS_33, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 209 },
    FindBiomesCase { mc: 25, ids: IDS_34, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 28, ids: IDS_35, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 9 },
    FindBiomesCase { mc: 28, ids: IDS_36, x: 0, z: 0, w: 4, h: 4, start: 0, dim: 0, y_height: 320, expect: 74 },
    FindBiomesCase { mc: 28, ids: IDS_37, x: 0, z: 0, w: 16, h: 16, start: 0, dim: 0, y_height: 320, expect: 262 },
    FindBiomesCase { mc: 28, ids: IDS_38, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 0 },
    FindBiomesCase { mc: 28, ids: IDS_39, x: -1000, z: 2000, w: 8, h: 8, start: 5, dim: 0, y_height: 320, expect: 58944 },
    FindBiomesCase { mc: 28, ids: IDS_40, x: 500, z: -500, w: 32, h: 32, start: 0, dim: 0, y_height: 320, expect: 209 },
    FindBiomesCase { mc: 28, ids: IDS_41, x: 0, z: 0, w: 64, h: 64, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 25, ids: IDS_42, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 95 },
    FindBiomesCase { mc: 25, ids: IDS_43, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 11 },
    FindBiomesCase { mc: 25, ids: IDS_44, x: 0, z: 0, w: 1, h: 1, start: 0, dim: 0, y_height: 320, expect: 88 },
    FindBiomesCase { mc: 25, ids: IDS_45, x: -300, z: 700, w: 64, h: 64, start: 999, dim: 0, y_height: 320, expect: 999 },
    FindBiomesCase { mc: 25, ids: IDS_46, x: 0, z: 0, w: 8, h: 8, start: 0, dim: -1, y_height: 320, expect: 4 },
    FindBiomesCase { mc: 25, ids: IDS_47, x: 0, z: 0, w: 16, h: 16, start: 0, dim: -1, y_height: 320, expect: 72 },
];
