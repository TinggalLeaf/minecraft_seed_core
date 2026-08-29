//! Loot 模块专用的 RNG 薄封装，对齐 MC 1.20.1 的
//! `XoroshiroRandomSource` 与 `LegacyRandomSource` 行为。
//!
//! 设计原则：
//!
//! - 不修改 crate 根的 [`crate::rng`] 模块对外行为；任何 loot 模块特有的
//!   小工具函数都在本文件内实现。
//! - 整数 `next_int_between(a, b)` 等价于 MC `MathHelper.nextInt(a, b)`：
//!   `a + nextInt(b - a + 1)`，含两端。
//!
//! 关于 [`XoroshiroLootRng::next_long`] 与 [`crate::rng::Xoroshiro::next_long`]
//! 的细微差别：cubiomes / 实际 MC Java 端对 `lo + hi` 先做 `wrapping_add`
//! 再 rotate；本模块参考的 Python 项目
//! （`E:\Projects\Minecraft\宝箱内容生成\src\rng.py`）在 `_rotate_left` 内
//! 隐式把 `lo + hi` 当 Python 任意精度整数旋转，导致当 `lo + hi >= 2^64`
//! 时旋转结果会多带一个 `1 << 17` 的位。为逐位对拍 Python
//! `loot_predictor.py` 输出，本模块复刻这一行为；其它非 loot 场景不受影响。
//!
//! 同时暴露：`LootRng` trait（抽象底层实现）+ [`XoroshiroLootRng`] /
//! [`LegacyLootRng`] 两个具体实现，分别对应 1.18+ 与 1.17- 的随机源。

use crate::rng::JavaRandom;

/// Loot 引擎需要的随机源抽象。
///
/// 设计成 trait 是为了让 `loot::table` 不必关心底层是 xoroshiro 还是
/// legacy；目前 `table::generate` 只需要三个方法，故 trait 极简。
pub trait LootRng {
    /// `nextLong()`，对应 vanilla `XoroshiroRandomSource.nextLong` /
    /// `LegacyRandomSource.nextLong`。
    fn next_long(&mut self) -> i64;

    /// `nextInt(bound)`，返回 `[0, bound)`。
    fn next_int_bound(&mut self, bound: u32) -> u32;

    /// `nextDouble()`，返回 `[0, 1)`。
    fn next_double(&mut self) -> f64;

    /// `next_int_between(origin, bound)`，含两端，等价 `MathHelper.nextInt`。
    ///
    /// 提供默认实现：`origin + self.next_int_bound(bound - origin + 1)`。
    fn next_int_between(&mut self, origin: i32, bound: i32) -> i32 {
        debug_assert!(bound >= origin, "nextIntBetween: bound({bound}) < origin({origin})");
        origin + (self.next_int_bound((bound - origin + 1) as u32) as i32)
    }
}

/// 1.18+ `XoroshiroRandomSource` 的适配，与 Python `XoroshiroRandomSource`
/// 逐位一致。
#[derive(Clone, Copy, Debug)]
pub struct XoroshiroLootRng {
    /// 内部 `(lo, hi)` 状态。
    pub lo: u64,
    pub hi: u64,
}

impl XoroshiroLootRng {
    pub fn new(seed: u64) -> Self {
        // 与 `crate::rng::Xoroshiro::new` 同源（splitMix 扩散）。
        // 此处保留自有代码以避免依赖 crate 私有字段（lo/hi）。
        // cubiomes：XL = 0x9e3779b97f4a7c15, XH = 0x6a09e667f3bcc909
        const XL: u64 = 0x9e3779b97f4a7c15;
        const XH: u64 = 0x6a09e667f3bcc909;
        const A: u64 = 0xbf58476d1ce4e5b9;
        const B: u64 = 0x94d049bb133111eb;
        let l = seed ^ XH;
        let h = l.wrapping_add(XL);
        let mix = |mut s: u64| {
            s = (s ^ (s >> 30)).wrapping_mul(A);
            s = (s ^ (s >> 27)).wrapping_mul(B);
            s ^ (s >> 31)
        };
        XoroshiroLootRng { lo: mix(l), hi: mix(h) }
    }

    pub fn from_state(lo: u64, hi: u64) -> Self {
        XoroshiroLootRng { lo, hi }
    }

    /// Python 项目 `XoroshiroRandomSource.next_long()` 的逐位复刻。
    ///
    /// 与 cubiomes `xNextLong` 在 `lo + hi` 不溢出的情况下完全一致；
    /// 溢出时 Python 把 `lo + hi` 视作任意精度整数旋转后取低 64 位，
    /// 溢出位（位置 64）旋转 17 位后落到位置 17，需要把它 OR 回结果中。
    pub fn next_long_python(&mut self) -> i64 {
        let l = self.lo;
        let h = self.hi;
        // Python：result = mask64(rotate_left(lo+hi, 17) + lo)
        // 用 u128 检测溢出，溢出时给旋转结果补回溢出位的偏移。
        let sum = (l as u128) + (h as u128);
        let sum_low = sum as u64;
        // cubiomes 风格的 64 位旋转
        let rot = sum_low.rotate_left(17);
        // 如果 Python 的 65 位 sum 在第 64 位有 1（即溢出），旋转 17 位后该位
        // 落到位置 17，OR 回结果中。
        let rot = if sum > u64::MAX as u128 {
            rot | (1u64 << 17)
        } else {
            rot
        };
        let result = rot.wrapping_add(l);
        // hi ^= lo; new_lo = rotl(lo, 49) ^ hi ^ (hi << 21); new_hi = rotl(hi, 28)
        let h2 = h ^ l;
        self.lo = l.rotate_left(49) ^ h2 ^ (h2 << 21);
        self.hi = h2.rotate_left(28);
        result as i64
    }

    /// 暴露底层状态以便测试与高级使用方。
    pub fn state(&self) -> (u64, u64) {
        (self.lo, self.hi)
    }
}

impl LootRng for XoroshiroLootRng {
    #[inline]
    fn next_long(&mut self) -> i64 {
        // 见 [`XoroshiroLootRng::next_long_python`] 的注释。
        self.next_long_python()
    }

    #[inline]
    fn next_int_bound(&mut self, bound: u32) -> u32 {
        // Lemire 拒绝采样：复用 crate::rng::Xoroshiro::next_int 的位模式。
        let mut r = (self.next_long() as u64) & 0xFFFF_FFFF;
        r = r.wrapping_mul(bound as u64);
        if (r as u32) < bound {
            let threshold = (!bound).wrapping_add(1) % bound;
            while (r as u32) < threshold {
                r = (self.next_long() as u64) & 0xFFFF_FFFF;
                r = r.wrapping_mul(bound as u64);
            }
        }
        (r >> 32) as u32
    }

    #[inline]
    fn next_double(&mut self) -> f64 {
        ((self.next_long() as u64) >> 11) as f64 * 1.1102230246251565e-16
    }
}

/// 1.17 及更早版本 `LegacyRandomSource` 的适配。
#[derive(Clone, Copy, Debug)]
pub struct LegacyLootRng {
    inner: JavaRandom,
}

impl LegacyLootRng {
    pub fn new(seed: i64) -> Self {
        LegacyLootRng { inner: JavaRandom::new(seed) }
    }

    /// 暴露原始 48 位状态。
    pub fn raw_state(&self) -> u64 {
        self.inner.raw_state()
    }
}

impl LootRng for LegacyLootRng {
    #[inline]
    fn next_long(&mut self) -> i64 {
        self.inner.next_long()
    }

    #[inline]
    fn next_int_bound(&mut self, bound: u32) -> u32 {
        self.inner.next_int_bound(bound as i32) as u32
    }

    #[inline]
    fn next_double(&mut self) -> f64 {
        self.inner.next_double()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xoroshiro_int_between_inclusive() {
        let mut rng = XoroshiroLootRng::new(12345);
        // 在 [1, 4] 跑 5000 次，全部落在范围内
        for _ in 0..5000 {
            let v = rng.next_int_between(1, 4);
            assert!((1..=4).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn xoroshiro_seed_init_matches_crate() {
        // 状态初始化（splitMix 扩散）与 crate::rng::Xoroshiro::new 等价。
        use crate::rng::Xoroshiro;
        for seed in [0u64, 1, 42, 12345, 0xdead_beef_cafe] {
            let a = Xoroshiro::new(seed);
            let b = XoroshiroLootRng::new(seed);
            assert_eq!((a.lo, a.hi), (b.lo, b.hi), "seed init mismatch for {seed}");
        }
    }

    #[test]
    fn xoroshiro_long_matches_python_reference() {
        // 复刻 Python `XoroshiroRandomSource` 的 next_long，包括溢出分支。
        // 与 `loot_predictor.py` 输出对拍（见 `tests/loot_consistency.rs`）。
        // 这里只验证一些手算与 Python 一致的固定点：
        // chunk seed = 0x81b6804c3 → 第一个 long (Python 输出 0x3e5819276e3eb4c0)
        let mut r = XoroshiroLootRng::from_state(0xb3709369e01401fb, 0xa5f23209e2cac519);
        assert_eq!(r.next_long() as u64, 0x3e5819276e3eb4c0);
        // chunk seed = 0x136f28d7b4638fea → first long = 0x528bad96f3f1612c
        let mut r = XoroshiroLootRng::from_state(0xeeaa1775fde2b511, 0x67639a7acd2dc5f6);
        assert_eq!(r.next_long() as u64, 0x528bad96f3f1612c);
    }

    #[test]
    fn xoroshiro_double_in_unit_interval() {
        let mut rng = XoroshiroLootRng::new(1);
        for _ in 0..500 {
            let v = rng.next_double();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
