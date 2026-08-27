//! Xoroshiro128++，1.18+ 气候噪声与新版随机源使用。
//! 对齐 cubiomes `rng.h` 的 `xSetSeed/xNextLong/xNextInt/...`。

const SILVER_RATIO: u64 = 0x6a09e667f3bcc909;
const GOLDEN_RATIO: u64 = 0x9e3779b97f4a7c15;
const MIX_A: u64 = 0xbf58476d1ce4e5b9;
const MIX_B: u64 = 0x94d049bb133111eb;

#[inline]
fn mix_stafford13(mut s: u64) -> u64 {
    s = (s ^ (s >> 30)).wrapping_mul(MIX_A);
    s = (s ^ (s >> 27)).wrapping_mul(MIX_B);
    s ^ (s >> 31)
}

/// Xoroshiro128++ 随机数发生器（MC 1.18+ `XoroshiroRandomSource` 内核）。
#[derive(Clone, Copy, Debug)]
pub struct Xoroshiro {
    pub lo: u64,
    pub hi: u64,
}

impl Xoroshiro {
    /// MC 种子扩散：`xSetSeed`。
    #[inline]
    pub fn new(seed: u64) -> Self {
        let l = seed ^ SILVER_RATIO;
        let h = l.wrapping_add(GOLDEN_RATIO);
        Xoroshiro {
            lo: mix_stafford13(l),
            hi: mix_stafford13(h),
        }
    }

    /// 直接以内部状态创建。
    #[inline]
    pub fn from_state(lo: u64, hi: u64) -> Self {
        Xoroshiro { lo, hi }
    }

    /// `xNextLong`：xoroshiro128++ 输出函数。
    #[inline]
    pub fn next_long(&mut self) -> u64 {
        let l = self.lo;
        let h = self.hi;
        let n = l.wrapping_add(h).rotate_left(17).wrapping_add(l);
        let h2 = h ^ l;
        self.lo = l.rotate_left(49) ^ h2 ^ (h2 << 21);
        self.hi = h2.rotate_left(28);
        n
    }

    /// `xNextInt(n)`：取 `next_long` 低 32 位做 Lemire 拒绝采样。
    #[inline]
    pub fn next_int(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0);
        let mut r = (self.next_long() & 0xFFFF_FFFF).wrapping_mul(n as u64);
        if (r as u32) < n {
            let threshold = (!n).wrapping_add(1) % n;
            while (r as u32) < threshold {
                r = (self.next_long() & 0xFFFF_FFFF).wrapping_mul(n as u64);
            }
        }
        (r >> 32) as u32
    }

    /// `xNextDouble`
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        (self.next_long() >> 11) as f64 * 1.1102230246251565e-16
    }

    /// `xNextFloat`
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        (self.next_long() >> 40) as f32 * 5.9604645e-8
    }

    /// `xSkipN`
    #[inline]
    pub fn skip(&mut self, count: u32) {
        for _ in 0..count {
            self.next_long();
        }
    }

    /// `xNextLongJ`：Java 风格（两次高 32 位拼接）。
    ///
    /// 注意 C 里是 `((uint64_t)a << 32) + b`，`a`/`b` 均为 int32_t，按符号
    /// 扩展转换到 uint64（`b` 为负时等价于向高 32 位借位），这里显式复刻。
    #[inline]
    pub fn next_long_j(&mut self) -> u64 {
        let a = (self.next_long() >> 32) as u32 as i32;
        let b = (self.next_long() >> 32) as u32 as i32;
        ((a as i64 as u64) << 32).wrapping_add(b as i64 as u64)
    }

    /// `xNextIntJ(n)`：Java 风格拒绝采样（高 31 位）。
    #[inline]
    pub fn next_int_j(&mut self, n: u32) -> i32 {
        debug_assert!(n > 0);
        let m = n.wrapping_sub(1);
        if (m & n) == 0 {
            let x = (n as u64) * (self.next_long() >> 33);
            return (x >> 31) as i32;
        }
        loop {
            let bits = (self.next_long() >> 33) as u32;
            let val = bits % n;
            if bits.wrapping_sub(val).wrapping_add(m) as i32 >= 0 {
                return val as i32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试向量由 cubiomes 的 C 参考实现（reference/cubiomes/rng.h）生成。
    #[test]
    fn seed_expansion_matches_cubiomes() {
        let cases: [(u64, u64, u64); 5] = [
            (0, 3847398142028685078, 7192185014346937746),
            (1, 5272463233947570727, 1927618558350093866),
            (42, 6720814022939733433, 15595420190114929605),
            (123456789, 11168089023507084530, 14777087788034466897),
            (0xdeadbeefcafe, 8372022640849968230, 9150774609728861018),
        ];
        for (seed, lo, hi) in cases {
            let xr = Xoroshiro::new(seed);
            assert_eq!((xr.lo, xr.hi), (lo, hi), "seed {seed}");
        }
    }

    #[test]
    fn next_long_matches_cubiomes() {
        let cases: [(u64, [u64; 4]); 5] = [
            (0, [3038984756725240190, 14752704786953913202, 4633751808701151732, 2160572957309072155]),
            (1, [17413076366490032638, 6451672561743293322, 16624853809821157986, 890086654470169703]),
            (42, [13750795694971935007, 7341713790291473579, 10904010558988233405, 4888889476139319686]),
            (123456789, [3219654894476264721, 8777179534091651608, 6434035011650043425, 9959798251472937234]),
            (0xdeadbeefcafe, [7688865154014233280, 8691279023884721055, 1988288189129561487, 14644851104931901477]),
        ];
        for (seed, expect) in cases {
            let mut xr = Xoroshiro::new(seed);
            for e in expect {
                assert_eq!(xr.next_long(), e, "seed {seed}");
            }
        }
    }

    #[test]
    fn next_int_matches_cubiomes() {
        let mut xr = Xoroshiro::new(1);
        let expect = [4u32, 1, 1, 6, 1, 5];
        for e in expect {
            assert_eq!(xr.next_int(10), e);
        }
    }

    #[test]
    fn next_int_j_matches_cubiomes() {
        let mut xr = Xoroshiro::new(1);
        let expect = [9, 7, 6, 4, 5, 7];
        for e in expect {
            assert_eq!(xr.next_int_j(10), e);
        }
    }

    #[test]
    fn next_double_matches_cubiomes() {
        let mut xr = Xoroshiro::new(1);
        // golden 十进制字面量按"最近 f64"舍入即 C 的位模式，保留全精度位数
        #[allow(clippy::excessive_precision)]
        let expect = [0.94396476131022433, 0.34974587038035987, 0.9012351308931007, 0.048251694223845565];
        for e in expect {
            assert_eq!(xr.next_double(), e);
        }
    }

    #[test]
    fn next_long_j_matches_cubiomes() {
        let mut xr = Xoroshiro::new(1);
        let expect = [17413076366257615363u64, 16624853809203132696, 8094835630624897715];
        for e in expect {
            assert_eq!(xr.next_long_j(), e);
        }
    }

    #[test]
    fn next_int_bounds() {
        let mut xr = Xoroshiro::new(99);
        for _ in 0..1000 {
            assert!(xr.next_int(6) < 6);
        }
    }
}

