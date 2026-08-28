//! `java.util.Random` 的逐位精确移植（48 位 LCG）。
//! 对齐 cubiomes `rng.h` 中的 `setSeed/next/nextInt/...`。

const MULTIPLIER: u64 = 0x5deece66d;
const ADDEND: u64 = 0xb;
const MASK: u64 = (1 << 48) - 1;

/// Java 线性同余随机数发生器（48 位状态）。
///
/// 与 Java 版 Minecraft 1.17 及之前的世界生成、以及所有版本的结构
/// 候选计算中使用的 `java.util.Random` 完全一致。
#[derive(Clone, Copy, Debug)]
pub struct JavaRandom {
    seed: u64,
}

impl JavaRandom {
    /// 以世界种子创建（内部做 `seed ^ 0x5DEECE66D` 混淆）。
    #[inline]
    pub fn new(seed: i64) -> Self {
        let mut r = JavaRandom { seed: 0 };
        r.set_seed(seed);
        r
    }

    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ MULTIPLIER) & MASK;
    }

    /// 取内部 48 位状态（用于调试与测试）。
    #[inline]
    pub fn raw_state(&self) -> u64 {
        self.seed
    }

    /// 生成 `bits` 位随机数（1..=32）。
    #[inline]
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        ((self.seed as i64) >> (48 - bits)) as i32
    }

    /// `Random.nextInt()`
    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// `Random.nextInt(n)`，要求 `n > 0`。
    #[inline]
    pub fn next_int_bound(&mut self, n: i32) -> i32 {
        debug_assert!(n > 0);
        let m = n - 1;
        if (m & n) == 0 {
            // n 为 2 的幂
            return (((n as i64) * (self.next(31) as i64)) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % n;
            if (bits as u32).wrapping_sub(val as u32).wrapping_add(m as u32) as i32 >= 0 {
                return val;
            }
        }
    }

    /// `Random.nextLong()`
    #[inline]
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// `Random.nextFloat()`
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1u32 << 24) as f32
    }

    /// `Random.nextDouble()`
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        let x = ((self.next(26) as u64) << 27) + self.next(27) as u64;
        x as f64 / (1u64 << 53) as f64
    }

    /// `Random.nextBoolean()`
    #[inline]
    pub fn next_bool(&mut self) -> bool {
        self.next(1) != 0
    }

    /// 前跳 `n` 次 `next` 调用（O(log n)，对应 cubiomes `skipNextN`）。
    pub fn skip(&mut self, n: u64) {
        let mut m: u64 = 1;
        let mut a: u64 = 0;
        let mut im: u64 = MULTIPLIER;
        let mut ia: u64 = ADDEND;
        let mut k = n;
        while k > 0 {
            if k & 1 != 0 {
                m = m.wrapping_mul(im);
                a = im.wrapping_mul(a).wrapping_add(ia);
            }
            ia = im.wrapping_add(1).wrapping_mul(ia);
            im = im.wrapping_mul(im);
            k >>= 1;
        }
        self.seed = self.seed.wrapping_mul(m).wrapping_add(a) & MASK;
    }
}

/// `mulInv`（cubiomes `rng.h`）：求模逆元 `(1/x) mod m`，扩展欧几里得。
///
/// 假设 `x`、`m` 为正（小于 2⁶³）且互素；无解时返回 0。四连底座搜索
/// （`structure::quadbase`）用它求 region 平移常数 `132897987541` 对
/// 2^n 的模逆。
pub fn mul_inv(x: u64, m: u64) -> u64 {
    if m as i64 <= 1 {
        return 0; // 无解
    }
    let n = m;
    let mut a = 0u64;
    let mut b = 1u64;
    let mut x = x;
    let mut m = m;

    while x as i64 > 1 {
        if m == 0 {
            return 0; // x 与 m 不互素
        }
        let q = x / m;
        let t = m;
        m = x % m;
        x = t;
        let t = a;
        a = b.wrapping_sub(q.wrapping_mul(a));
        b = t;
    }

    if (b as i64) < 0 {
        b = b.wrapping_add(n);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    // 以下向量来自 java.util.Random 官方行为（广泛引用且经 cubiomes C 实现复核）。
    #[test]
    fn matches_java_seed0() {
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int(), -1155484576);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_long(), -4962768465676381896);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_double(), 0.730967787376657);
    }

    #[test]
    fn matches_java_seed42() {
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(), -1170105035);
        assert_eq!(r.next_int(), 234785527);
        assert_eq!(r.next_int(), -1360544799);
    }

    #[test]
    fn next_int_bound_power_of_two() {
        // cubiomes C 参考实现实测：setSeed(0); nextInt(16) == 11, nextInt(10) == 0
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int_bound(16), 11);
        let mut r = JavaRandom::new(0);
        assert_eq!(r.next_int_bound(10), 0);
    }

    #[test]
    fn next_int_bound_range() {
        let mut r = JavaRandom::new(123);
        for _ in 0..1000 {
            let v = r.next_int_bound(7);
            assert!((0..7).contains(&v));
        }
    }

    #[test]
    fn skip_matches_sequential() {
        let mut a = JavaRandom::new(7);
        let mut b = JavaRandom::new(7);
        for _ in 0..100 {
            a.next(31);
        }
        b.skip(100);
        assert_eq!(a.raw_state(), b.raw_state());
    }
}
