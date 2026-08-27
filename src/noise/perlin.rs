//! Perlin 噪声核心。
//!
//! 逐函数移植 cubiomes `noise.c` 的 `perlinInit` / `xPerlinInit` /
//! `samplePerlin` / `sampleSimplex2D`，以及 `static` 的
//! `samplePerlinBeta17Terrain`。
//!
//! ## 与 cubiomes 的接口差异
//!
//! cubiomes 的 `perlinInit(noise, uint64_t *seed)` 直接操作 48 位 LCG 原始
//! 状态指针；该状态机与本 crate 的 [`JavaRandom`] 完全一致（调用点总是先
//! `setSeed`），因此这里改用 `&mut JavaRandom`，语义不变且更符合 Rust 习惯。
//!
//! ## 浮点精确性
//!
//! 所有 `f64` 运算严格保持 C 源码中的运算顺序与结合方式，Rust 与 C 的
//! IEEE 754 double 运算逐位一致，测试中用位模式（`f64::to_bits`）精确比较。
//!
//! ## 已知偏差：`sample_beta17_terrain`
//!
//! cubiomes 的 `samplePerlinBeta17Terrain` 使用 `int a1 = idx[i1] + i2;`
//! （取值可达 510）索引仅 257 字节的置换表 `d[257]`，是**越界读**（实测
//! 与掩码版结果完全不同，读到相邻 octave 的置换表/结构体内存）。
//! MC Beta 原版使用 512 项的对折置换表（`perm[256+i] == perm[i]`），
//! 等价于对下标 `& 0xff`。本移植采用 `& 0xff` 掩码（即 MC 原版语义），
//! 因此该函数的测试向量由"加掩码的 C 参考"生成，而非由含未定义行为的
//! 原版生成。详见 `reference/gen/betacheck.c`。

use crate::rng::{JavaRandom, Xoroshiro};

/// 三次缓和曲线 `6t^5 - 15t^4 + 10t^3`，保持 C 的运算顺序。
#[inline(always)]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// `rng.h` 的 `lerp`：`from + part * (to - from)`。
#[inline(always)]
pub(crate) fn lerp(part: f64, from: f64, to: f64) -> f64 {
    from + part * (to - from)
}

/// 梯度点积（cubiomes `indexedLerp` 的 switch 形式）。
///
/// 注意 `idx & 0xf` 后 12/13/14 与 0/9/1 重复、且没有 `b - c` 之外的对称项，
/// 这是 MC 沿用至今的查表方式，原样保留。
#[inline(always)]
fn indexed_lerp(idx: u8, a: f64, b: f64, c: f64) -> f64 {
    match idx & 0xf {
        0 => a + b,
        1 => -a + b,
        2 => a - b,
        3 => -a - b,
        4 => a + c,
        5 => -a + c,
        6 => a - c,
        7 => -a - c,
        8 => b + c,
        9 => -b + c,
        10 => b - c,
        11 => -b - c,
        12 => a + b,
        13 => -b + c,
        14 => -a + b,
        // 15 与不可达分支（match 穷尽性要求）
        _ => -b - c,
    }
}

/// C 中 `uint8_t h = (int) floor(v)` 的两步转换：
/// 先向零截断为 `int`，再按模 256 存入 `uint8_t`。
///
/// Rust 的 `f64 as i32` 是饱和转换而非 C 的未定义行为；对 MC 实际输入范围
/// （|v| 远小于 2^31）两者完全一致。越界输入在 C 侧本就是 UB。
#[inline(always)]
fn floor_to_u8(v: f64) -> u8 {
    (v as i32) as u8
}

/// Perlin 噪声发生器（对应 cubiomes `PerlinNoise`）。
///
/// 字段含义与 C 结构体一一对应：`perm` 即 `d[256+1]`（末项重复首项），
/// `a/b/c` 是三个坐标轴的采样偏移，`h2/d2/t2` 是 `y == 0` 快路径的缓存。
#[derive(Clone, Debug)]
pub struct PerlinNoise {
    /// 256 项置换表 + 末尾重复的首项（供 `perm[i+1]` 在 `i == 255` 时使用）。
    pub perm: [u8; 257],
    /// `b` 的整数部分（模 256）。
    pub h2: u8,
    /// x/y/z 轴采样偏移（`nextDouble() * 256`）。
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// 振幅（octave 初始化时覆写）。
    pub amplitude: f64,
    /// 频隙（octave 初始化时覆写）。
    pub lacunarity: f64,
    /// `b` 的小数部分。
    pub d2: f64,
    /// `fade(d2)`。
    pub t2: f64,
}

impl PerlinNoise {
    /// `perlinInit`：用 Java LCG 初始化并 Fisher–Yates 洗牌置换表。
    ///
    /// 注意洗牌循环是 `j = nextInt(256 - i) + i`（即只与尚未固定的后缀交换），
    /// 且 `nextInt` 的实参在循环中逐次减小，与朴素洗牌不同。
    pub fn new_java(rng: &mut JavaRandom) -> Self {
        let a = rng.next_double() * 256.0;
        let b = rng.next_double() * 256.0;
        let c = rng.next_double() * 256.0;
        Self::finish_init(a, b, c, |bound| rng.next_int_bound(bound))
    }

    /// `xPerlinInit`：用 Xoroshiro128++ 初始化（MC 1.18+）。
    pub fn new_xoroshiro(xr: &mut Xoroshiro) -> Self {
        let a = xr.next_double() * 256.0;
        let b = xr.next_double() * 256.0;
        let c = xr.next_double() * 256.0;
        Self::finish_init(a, b, c, |bound| xr.next_int(bound as u32) as i32)
    }

    /// 两种初始化共享的填表/洗牌/缓存逻辑（`perlinInit` 与 `xPerlinInit`
    /// 除随机源外逐行相同）。
    fn finish_init(a: f64, b: f64, c: f64, mut next_int: impl FnMut(i32) -> i32) -> Self {
        let mut perm = [0u8; 257];
        for (i, p) in perm.iter_mut().enumerate().take(256) {
            *p = i as u8;
        }
        for i in 0..256 {
            let j = (next_int(256 - i as i32) + i as i32) as usize;
            perm.swap(i, j);
        }
        perm[256] = perm[0];
        let i2 = b.floor();
        let d2 = b - i2;
        PerlinNoise {
            perm,
            h2: floor_to_u8(i2),
            a,
            b,
            c,
            amplitude: 1.0,
            lacunarity: 1.0,
            d2,
            t2: fade(d2),
        }
    }

    /// `samplePerlin`：3D Perlin 采样。
    ///
    /// `yamp`/`ymin` 为 MC 1.15+ 的下界 y 截断（`yamp != 0.0` 时启用）。
    ///
    /// 注意 `y == 0.0` 是特殊快路径：直接使用初始化时缓存的 `h2/d2/t2`
    /// （因为此时 `y + b == b`），这也是 `sampleOctaveAmp` 用 `ydefault`
    /// 传 `-p.b` 来命中该路径的原因。
    pub fn sample(&self, x: f64, y: f64, z: f64, yamp: f64, ymin: f64) -> f64 {
        let mut d1 = x;
        let mut d2 = y;
        let mut d3 = z;

        let h2: u8;
        let t2: f64;
        if d2 == 0.0 {
            d2 = self.d2;
            h2 = self.h2;
            t2 = self.t2;
        } else {
            d2 += self.b;
            let i2 = d2.floor();
            d2 -= i2;
            h2 = floor_to_u8(i2);
            t2 = fade(d2);
        }

        d1 += self.a;
        d3 += self.c;

        let i1 = d1.floor();
        let i3 = d3.floor();
        d1 -= i1;
        d3 -= i3;

        let h1 = floor_to_u8(i1);
        let h3 = floor_to_u8(i3);

        let t1 = fade(d1);
        let t3 = fade(d3);

        if yamp != 0.0 {
            let yclamp = if ymin < d2 { ymin } else { d2 };
            d2 -= (yclamp / yamp).floor() * yamp;
        }

        let idx = &self.perm;

        // 与 C 的 vec2 版本逐行对应；所有下标加法都是 uint8_t 回绕加法，
        // `idx[x + 1]` 在 x == 255 时读取 idx[256]（== idx[0]）。
        let v1a = idx[h1 as usize].wrapping_add(h2);
        let v1b = idx[h1 as usize + 1].wrapping_add(h2);

        let v2a = idx[v1a as usize].wrapping_add(h3);
        let v2b = idx[v1a as usize + 1].wrapping_add(h3);
        let v3a = idx[v1b as usize].wrapping_add(h3);
        let v3b = idx[v1b as usize + 1].wrapping_add(h3);

        let v4a = idx[v2a as usize];
        let v4b = idx[v2a as usize + 1];
        let v5a = idx[v2b as usize];
        let v5b = idx[v2b as usize + 1];
        let v6a = idx[v3a as usize];
        let v6b = idx[v3a as usize + 1];
        let v7a = idx[v3b as usize];
        let v7b = idx[v3b as usize + 1];

        let mut l1 = indexed_lerp(v4a, d1, d2, d3);
        let mut l5 = indexed_lerp(v4b, d1, d2, d3 - 1.0);
        let l2 = indexed_lerp(v6a, d1 - 1.0, d2, d3);
        let l6 = indexed_lerp(v6b, d1 - 1.0, d2, d3 - 1.0);
        let mut l3 = indexed_lerp(v5a, d1, d2 - 1.0, d3);
        let mut l7 = indexed_lerp(v5b, d1, d2 - 1.0, d3 - 1.0);
        let l4 = indexed_lerp(v7a, d1 - 1.0, d2 - 1.0, d3);
        let l8 = indexed_lerp(v7b, d1 - 1.0, d2 - 1.0, d3 - 1.0);

        l1 = lerp(t1, l1, l2);
        l3 = lerp(t1, l3, l4);
        l5 = lerp(t1, l5, l6);
        l7 = lerp(t1, l7, l8);

        l1 = lerp(t2, l1, l3);
        l5 = lerp(t2, l5, l7);

        lerp(t3, l1, l5)
    }

    /// `sampleSimplex2D`：2D Simplex 采样（Beta 1.7 群系温度/湿度用）。
    ///
    /// 注意此函数只使用置换表，**不**加 `a/b/c` 偏移——偏移由调用方
    /// （`sampleOctaveBeta17Biome`）在外部完成。
    pub fn sample_simplex_2d(&self, x: f64, y: f64) -> f64 {
        // sqrt(3) 在 C 与 Rust 中都是正确舍入的 IEEE 开方，逐位一致。
        let skew = 0.5 * (3.0f64.sqrt() - 1.0);
        let unskew = (3.0 - 3.0f64.sqrt()) / 6.0;

        let hf = (x + y) * skew;
        let hx = (x + hf).floor() as i32;
        let hz = (y + hf).floor() as i32;
        let mhxz = (hx + hz) as f64 * unskew;
        let x0 = x - (hx as f64 - mhxz);
        let y0 = y - (hz as f64 - mhxz);
        // C: int offx = (x0 > y0); int offz = !offx;
        let offx = i32::from(x0 > y0);
        let offz = i32::from(offx == 0);
        let x1 = x0 - offx as f64 + unskew;
        let y1 = y0 - offz as f64 + unskew;
        let x2 = x0 - 1.0 + 2.0 * unskew;
        let y2 = y0 - 1.0 + 2.0 * unskew;
        let d = &self.perm;
        let gi0 = d[(0xff & hz) as usize] as i32;
        let gi1 = d[(0xff & (hz + offz)) as usize] as i32;
        let gi2 = d[(0xff & (hz + 1)) as usize] as i32;
        let gi0 = d[(0xff & (gi0 + hx)) as usize] as i32;
        let gi1 = d[(0xff & (gi1 + hx + offx)) as usize] as i32;
        let gi2 = d[(0xff & (gi2 + hx + 1)) as usize] as i32;
        let mut t = 0.0;
        t += simplex_grad(gi0 % 12, x0, y0, 0.0, 0.5);
        t += simplex_grad(gi1 % 12, x1, y1, 0.0, 0.5);
        t += simplex_grad(gi2 % 12, x2, y2, 0.0, 0.5);
        70.0 * t
    }

    /// `samplePerlinBeta17Terrain`（C 中为 `static`）：Beta 1.7 地形用的
    /// 垂直方向 9 层采样，结果累加进 `v[0]`/`v[1]`。
    ///
    /// **与 C 的刻意差异**：C 用 `int` 下标 `idx[i1] + i2`（可达 510）索引
    /// 257 字节的表，是越界读（见模块文档）。这里对下标加 `& 0xff`，
    /// 等价于 MC Beta 原版的 512 项对折置换表。
    ///
    /// `genFlag` 两趟循环是 cubiomes 的优化：y 轴上相邻 `yi` 的整数部分
    /// 往往相同，只在变化时重算 8 个梯度角点；第一趟找出 0..=7 中最后一次
    /// 变化的位置 `yic`，第二趟从 `yic` 开始复用已算的角点。
    pub(crate) fn sample_beta17_terrain(
        &self,
        v: &mut [f64; 2],
        x: f64,
        z: f64,
        y_lac_amp: f64,
    ) {
        let mut gen_flag: i32 = -1;
        let mut l1 = 0.0;
        let mut l3 = 0.0;
        let mut l5 = 0.0;
        let mut l7 = 0.0;

        let mut d1 = x + self.a;
        let mut d3 = z + self.c;
        let idx = &self.perm;
        let mut i1 = d1.floor() as i32;
        let mut i3 = d3.floor() as i32;
        d1 -= i1 as f64;
        d3 -= i3 as f64;
        let t1 = fade(d1);
        let t3 = fade(d3);

        i1 &= 0xff;
        i3 &= 0xff;

        let mut yic = 0;
        let mut gf_copy = 0;
        for yi in 0..=7 {
            let d2 = yi as f64 * self.lacunarity * y_lac_amp + self.b;
            let i2 = (d2.floor() as i32) & 0xff;
            if yi == 0 || i2 != gen_flag {
                yic = yi;
                gf_copy = gen_flag;
                gen_flag = i2;
            }
        }
        gen_flag = gf_copy;

        for yi in yic..=8 {
            let mut d2 = yi as f64 * self.lacunarity * y_lac_amp + self.b;
            let mut i2 = d2.floor() as i32;
            d2 -= i2 as f64;
            let t2 = fade(d2);

            i2 &= 0xff;

            if yi == 0 || i2 != gen_flag {
                gen_flag = i2;
                let a1 = (idx[i1 as usize] as i32 + i2) & 0xff;
                let b1 = (idx[(i1 + 1) as usize] as i32 + i2) & 0xff;

                let a2 = (idx[a1 as usize] as i32 + i3) & 0xff;
                let a3 = (idx[((a1 + 1) & 0xff) as usize] as i32 + i3) & 0xff;
                let b2 = (idx[b1 as usize] as i32 + i3) & 0xff;
                let b3 = (idx[((b1 + 1) & 0xff) as usize] as i32 + i3) & 0xff;

                let m1 = indexed_lerp(idx[a2 as usize], d1, d2, d3);
                let l2 = indexed_lerp(idx[b2 as usize], d1 - 1.0, d2, d3);
                let m3 = indexed_lerp(idx[a3 as usize], d1, d2 - 1.0, d3);
                let l4 = indexed_lerp(idx[b3 as usize], d1 - 1.0, d2 - 1.0, d3);
                let m5 = indexed_lerp(idx[((a2 + 1) & 0xff) as usize], d1, d2, d3 - 1.0);
                let l6 = indexed_lerp(idx[((b2 + 1) & 0xff) as usize], d1 - 1.0, d2, d3 - 1.0);
                let m7 = indexed_lerp(idx[((a3 + 1) & 0xff) as usize], d1, d2 - 1.0, d3 - 1.0);
                let l8 = indexed_lerp(idx[((b3 + 1) & 0xff) as usize], d1 - 1.0, d2 - 1.0, d3 - 1.0);

                l1 = lerp(t1, m1, l2);
                l3 = lerp(t1, m3, l4);
                l5 = lerp(t1, m5, l6);
                l7 = lerp(t1, m7, l8);
            }

            if yi >= 7 {
                let n1 = lerp(t2, l1, l3);
                let n5 = lerp(t2, l5, l7);

                v[(yi - 7) as usize] += lerp(t3, n1, n5) * self.amplitude;
            }
        }
    }
}

/// `simplexGrad`：Simplex 角点贡献（径向衰减四次方 × 梯度点积）。
fn simplex_grad(idx: i32, x: f64, y: f64, z: f64, d: f64) -> f64 {
    let mut con = d - x * x - y * y - z * z;
    if con < 0.0 {
        return 0.0;
    }
    con *= con;
    con * con * indexed_lerp(idx as u8, x, y, z)
}

#[cfg(test)]
mod tests;
