//! Bedrock 版版本枚举。
//!
//! mcseedmap.com 的 Bedrock 模式使用的版本标签与 wasm 内部的 `mcVersion`
//! 整数一一对应（14=1.16.0 … 28=26.50）。结构配置表中的版本分派只用到
//! `mc > 17` 一个分支点（见 [`crate::bedrock::structure`]）。

/// Minecraft Bedrock 版版本号（对应 wasm `mcVersion` 整数）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[repr(i32)]
pub enum BedrockVersion {
    V1_16_0 = 14,
    V1_16_220 = 15,
    V1_17_40 = 17,
    V1_18_0 = 18,
    V1_18_30 = 19,
    V1_19_0 = 21,
    V1_19_80 = 22,
    V1_20_0 = 23,
    V1_20_80 = 24,
    V1_21_0 = 25,
    V1_21_50 = 26,
    V26_30 = 27,
    V26_50 = 28,
}

impl BedrockVersion {
    /// 库支持的最新版本。
    pub const NEWEST: BedrockVersion = BedrockVersion::V26_50;
    /// 库支持的最早版本。
    pub const OLDEST: BedrockVersion = BedrockVersion::V1_16_0;

    /// 所有支持版本，按时间升序。
    pub const ALL: &'static [BedrockVersion] = &[
        BedrockVersion::V1_16_0,
        BedrockVersion::V1_16_220,
        BedrockVersion::V1_17_40,
        BedrockVersion::V1_18_0,
        BedrockVersion::V1_18_30,
        BedrockVersion::V1_19_0,
        BedrockVersion::V1_19_80,
        BedrockVersion::V1_20_0,
        BedrockVersion::V1_20_80,
        BedrockVersion::V1_21_0,
        BedrockVersion::V1_21_50,
        BedrockVersion::V26_30,
        BedrockVersion::V26_50,
    ];

    /// wasm `mcVersion` 整数。
    #[inline]
    pub fn mc(self) -> i32 {
        self as i32
    }

    /// 从 wasm `mcVersion` 整数构造；不支持的整数返回 `None`。
    pub fn from_mc(mc: i32) -> Option<BedrockVersion> {
        Some(match mc {
            14 => BedrockVersion::V1_16_0,
            15 => BedrockVersion::V1_16_220,
            17 => BedrockVersion::V1_17_40,
            18 => BedrockVersion::V1_18_0,
            19 => BedrockVersion::V1_18_30,
            21 => BedrockVersion::V1_19_0,
            22 => BedrockVersion::V1_19_80,
            23 => BedrockVersion::V1_20_0,
            24 => BedrockVersion::V1_20_80,
            25 => BedrockVersion::V1_21_0,
            26 => BedrockVersion::V1_21_50,
            27 => BedrockVersion::V26_30,
            28 => BedrockVersion::V26_50,
            _ => return None,
        })
    }

    /// 人类可读的版本字符串，如 `"1.21.50"`。
    pub fn name(self) -> &'static str {
        match self {
            BedrockVersion::V1_16_0 => "1.16.0",
            BedrockVersion::V1_16_220 => "1.16.220",
            BedrockVersion::V1_17_40 => "1.17.40",
            BedrockVersion::V1_18_0 => "1.18.0",
            BedrockVersion::V1_18_30 => "1.18.30",
            BedrockVersion::V1_19_0 => "1.19.0",
            BedrockVersion::V1_19_80 => "1.19.80",
            BedrockVersion::V1_20_0 => "1.20.0",
            BedrockVersion::V1_20_80 => "1.20.80",
            BedrockVersion::V1_21_0 => "1.21.0",
            BedrockVersion::V1_21_50 => "1.21.50",
            BedrockVersion::V26_30 => "26.30",
            BedrockVersion::V26_50 => "26.50",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mc_roundtrip() {
        for &v in BedrockVersion::ALL {
            assert_eq!(BedrockVersion::from_mc(v.mc()), Some(v));
        }
        assert_eq!(BedrockVersion::from_mc(16), None);
    }
}
