//! Minecraft 版本与维度枚举，对齐 cubiomes 的 `MCVersion` / `Dimension`。
//!
//! 支持 Beta 1.7 到最新正式版；枚举按发布时间排序，可直接用 `>=`
//! 比较版本先后（如 `version >= McVersion::V1_18`）。

/// Minecraft 版本号（`B1_X` 为 Beta 版本，`V1_X` 表示该大版本的最新补丁，
/// 与 cubiomes 一致）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(u8)]
pub enum McVersion {
    /// Beta 1.7（cubiomes `MC_B1_7`）。
    B1_7,
    /// Beta 1.8（cubiomes `MC_B1_8`）。
    B1_8,
    V1_0,
    V1_1,
    V1_2,
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
    V1_8,
    V1_9,
    V1_10,
    V1_11,
    V1_12,
    V1_13,
    V1_14,
    V1_15,
    /// 1.16.1 的结构/群系规则与 1.16.5 略有差异，单列。
    V1_16_1,
    V1_16,
    V1_17,
    V1_18,
    /// 1.19.2 的群系参数与 1.19.4 不同（对应 cubiomes MC_1_19_2）。
    V1_19_2,
    V1_19,
    V1_20,
    V1_21_1,
    V1_21_3,
    /// 1.21.4+（Winter Drop，含 pale_garden），cubiomes 的 MC_1_21。
    V1_21,
}

impl McVersion {
    /// 库支持的最新版本。
    pub const NEWEST: McVersion = McVersion::V1_21;
    /// 库支持的最早版本。
    pub const OLDEST: McVersion = McVersion::B1_7;

    /// 所有支持版本，按时间升序。
    pub const ALL: &'static [McVersion] = &[
        McVersion::B1_7,
        McVersion::B1_8,
        McVersion::V1_0,
        McVersion::V1_1,
        McVersion::V1_2,
        McVersion::V1_3,
        McVersion::V1_4,
        McVersion::V1_5,
        McVersion::V1_6,
        McVersion::V1_7,
        McVersion::V1_8,
        McVersion::V1_9,
        McVersion::V1_10,
        McVersion::V1_11,
        McVersion::V1_12,
        McVersion::V1_13,
        McVersion::V1_14,
        McVersion::V1_15,
        McVersion::V1_16_1,
        McVersion::V1_16,
        McVersion::V1_17,
        McVersion::V1_18,
        McVersion::V1_19_2,
        McVersion::V1_19,
        McVersion::V1_20,
        McVersion::V1_21_1,
        McVersion::V1_21_3,
        McVersion::V1_21,
    ];

    /// 是否使用 1.18+ 的多噪声（multi-noise）群系生成。
    #[inline]
    pub fn has_multi_noise_biomes(self) -> bool {
        self >= McVersion::V1_18
    }

    /// 人类可读的版本字符串，如 `"1.18.2"`（Beta 版本为 `"b1.7.3"` /
    /// `"b1.8.1"`，对应 cubiomes `mc2str` 的 `"Beta 1.7"` / `"Beta 1.8"`）。
    pub fn name(self) -> &'static str {
        match self {
            McVersion::B1_7 => "b1.7.3",
            McVersion::B1_8 => "b1.8.1",
            McVersion::V1_0 => "1.0.0",
            McVersion::V1_1 => "1.1",
            McVersion::V1_2 => "1.2.5",
            McVersion::V1_3 => "1.3.2",
            McVersion::V1_4 => "1.4.7",
            McVersion::V1_5 => "1.5.2",
            McVersion::V1_6 => "1.6.4",
            McVersion::V1_7 => "1.7.10",
            McVersion::V1_8 => "1.8.9",
            McVersion::V1_9 => "1.9.4",
            McVersion::V1_10 => "1.10.2",
            McVersion::V1_11 => "1.11.2",
            McVersion::V1_12 => "1.12.2",
            McVersion::V1_13 => "1.13.2",
            McVersion::V1_14 => "1.14.4",
            McVersion::V1_15 => "1.15.2",
            McVersion::V1_16_1 => "1.16.1",
            McVersion::V1_16 => "1.16.5",
            McVersion::V1_17 => "1.17.1",
            McVersion::V1_18 => "1.18.2",
            McVersion::V1_19_2 => "1.19.2",
            McVersion::V1_19 => "1.19.4",
            McVersion::V1_20 => "1.20.6",
            McVersion::V1_21_1 => "1.21.1",
            McVersion::V1_21_3 => "1.21.3",
            McVersion::V1_21 => "1.21.4",
        }
    }
}

/// 维度，对齐 cubiomes 的 `Dimension` 数值。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(i8)]
pub enum Dimension {
    Nether = -1,
    Overworld = 0,
    End = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_ordered() {
        for w in McVersion::ALL.windows(2) {
            assert!(w[0] < w[1], "{:?} should precede {:?}", w[0], w[1]);
        }
        assert!(McVersion::V1_18.has_multi_noise_biomes());
        assert!(!McVersion::V1_17.has_multi_noise_biomes());
        assert_eq!(McVersion::NEWEST, *McVersion::ALL.last().unwrap());
        assert_eq!(McVersion::OLDEST, McVersion::ALL[0]);
        assert!(McVersion::B1_7 < McVersion::B1_8 && McVersion::B1_8 < McVersion::V1_0);
    }

    #[test]
    fn dimension_discriminants_match_cubiomes() {
        assert_eq!(Dimension::Nether as i8, -1);
        assert_eq!(Dimension::Overworld as i8, 0);
        assert_eq!(Dimension::End as i8, 1);
    }
}
