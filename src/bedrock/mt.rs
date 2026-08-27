//! MT19937 伪随机数发生器（Bedrock 版）。
//!
//! 与 Java 版使用的 LCG/Xoroshiro 不同，Bedrock 的结构散布与出生点/要塞
//! 计算全部基于标准 MT19937（mersenne twister）。本实现逐行为对齐
//! mcseedmap.com `bedrock.wasm` 的 `be_mt_n_get`（wasm 内为 `init_genrand`
//! + 标准 twist/temper）：种子只取**低 32 位**，状态 624 个 u32。

/// 标准 MT19937（32 位），与 wasm `be_mt_n_get` 逐输出一致。
pub struct Mt19937 {
    state: [u32; 624],
    /// 下一个待 temper 输出的下标；625 表示需要先做 twist。
    index: usize,
}

impl Mt19937 {
    /// 以 `seed` 初始化（等价于 `init_genrand`，只用种子的低 32 位）。
    pub(crate) fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = seed;
        for i in 1..624 {
            let prev = state[i - 1];
            state[i] = 1812433253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        Mt19937 { state, index: 625 }
    }

    fn twist(&mut self) {
        for i in 0..624 {
            let x = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
            let mut xa = x >> 1;
            if x & 1 != 0 {
                xa ^= 0x9908_b0df;
            }
            self.state[i] = self.state[(i + 397) % 624] ^ xa;
        }
        self.index = 0;
    }

    /// 生成下一个 u32 输出。
    pub(crate) fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }
}

/// 生成以 `seed`（低 32 位有效）为种子的前 `n` 个 MT19937 输出。
///
/// 对应 wasm 导出 `f`（`be_mt_n_get`）。
pub fn mt_outputs(seed: u32, n: usize) -> Vec<u32> {
    let mut mt = Mt19937::new(seed);
    (0..n).map(|_| mt.next_u32()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_first_outputs() {
        // 与网站 wasm（及标准 MT19937 参考实现）逐值一致。
        assert_eq!(mt_outputs(0, 2), [2357136044, 2546248239]);
        assert_eq!(mt_outputs(1, 1), [1791095845]);
        assert_eq!(mt_outputs(12345, 1), [3992670690]);
    }
}
