//! 1.18+ 主世界群系气候噪声（multi-noise biome source）。
//!
//! 逐函数移植 cubiomes `biomenoise.c` 的 `initBiomeNoise` / `setBiomeSeed` /
//! `sampleBiomeNoise`（噪声采样部分）与 spline 相关静态函数
//! （`getOffsetValue` / `createSpline_38219` / `createFlatOffsetSpline` /
//! `createLandSpline` / `getSpline`）。
//!
//! ## 结构
//!
//! - [`BiomeNoise`]：6 个气候参数的 [`DoublePerlinNoise`]（shift / temperature
//!   / humidity / continentalness / erosion / weirdness），各自用
//!   `md5("minecraft:<name>")` 盐从世界种子派生（`init_climate_seed`）。
//! - 深度（depth）不是独立噪声，而是由 continentalness/erosion/weirdness
//!   经 spline 表（`initBiomeNoise` 构建的三维样条树）加高度项算出。
//! - 采样出口是 6 个 ×10000 定点化的气候值（`np[6]`），群系判定
//!   （biome tree 查表，对应 `climateToBiome`）在
//!   [`crate::generator::v1_18`] 中完成。
//!
//! ## 未覆盖
//!
//! - `setClimateParaSeed` / `sampleClimatePara`（单气候参数调试初始化，
//!   cubiomes viewer 用）未移植；本库只支持 `setBiomeSeed` 的完整初始化。
//! - Beta 1.7 的 `BiomeNoiseBeta` 系列不属于本模块（1.18+ 范围之外）。
//!
//! ## 浮点精确性
//!
//! spline 构建与求值全部使用 `f32`（C 的 `float`），噪声采样使用 `f64`，
//! 定点化 `(int64_t)(10000.0F * v)` 用 `f32` 乘法后向零截断，均与 C 逐位一致。

use crate::rng::Xoroshiro;

use super::DoublePerlinNoise;

/// 气候参数下标（对应 cubiomes 的 `NP_*` 枚举）。
pub const NP_TEMPERATURE: usize = 0;
pub const NP_HUMIDITY: usize = 1;
pub const NP_CONTINENTALNESS: usize = 2;
pub const NP_EROSION: usize = 3;
/// shift 与 depth 共用下标（depth 不是真实气候）。
pub const NP_SHIFT: usize = 4;
pub const NP_DEPTH: usize = NP_SHIFT;
pub const NP_WEIRDNESS: usize = 5;
/// 气候参数总数。
pub const NP_MAX: usize = 6;

/// `sampleBiomeNoise` 的采样标志（对应 cubiomes 的 `SAMPLE_*`）。
/// 跳过局部扰动（shift 偏移）。
pub const SAMPLE_NO_SHIFT: u32 = 0x1;
/// 跳过 depth 采样（无垂直群系变化）。
pub const SAMPLE_NO_DEPTH: u32 = 0x2;

// spline 树的输入维度（对应 cubiomes 的 `SP_*` 枚举）。
const SP_CONTINENTALNESS: usize = 0;
const SP_EROSION: usize = 1;
const SP_RIDGES: usize = 2;
#[allow(dead_code)] // 文档用途：eval 的 vals[3] 即 weirdness
const SP_WEIRDNESS: usize = 3;

/// spline 子值：固定值（对应 C 的 `FixSpline`）或子样条节点下标。
#[derive(Clone, Copy, Debug)]
enum SplineVal {
    Fix(f32),
    Node(u32),
}

/// spline 节点（对应 cubiomes `Spline`，定长 12 槽位 + `len`）。
#[derive(Clone, Copy, Debug)]
struct SplineNode {
    len: usize,
    typ: usize,
    loc: [f32; 12],
    der: [f32; 12],
    val: [SplineVal; 12],
}

impl SplineNode {
    fn new(typ: usize) -> Self {
        SplineNode {
            len: 0,
            typ,
            loc: [0.0; 12],
            der: [0.0; 12],
            val: [SplineVal::Fix(0.0); 12],
        }
    }
}

/// spline 树的构造器（对应 C 的 `SplineStack` 内存池，改为索引竞技场）。
#[derive(Clone, Debug)]
struct SplineBuilder {
    nodes: Vec<SplineNode>,
}

impl SplineBuilder {
    /// `createFixSpline`：返回一个单点固定值样条。
    fn fix(&mut self, _val: f32) -> SplineVal {
        // C 里 FixSpline 伪装成 len==1 的 Spline；这里直接用值叶节点。
        SplineVal::Fix(_val)
    }

    fn push_node(&mut self, typ: usize) -> u32 {
        self.nodes.push(SplineNode::new(typ));
        (self.nodes.len() - 1) as u32
    }

    /// `addSplineVal`
    fn add(&mut self, idx: u32, loc: f32, val: SplineVal, der: f32) {
        let sp = &mut self.nodes[idx as usize];
        sp.loc[sp.len] = loc;
        sp.val[sp.len] = val;
        sp.der[sp.len] = der;
        sp.len += 1;
    }

    /// `getOffsetValue`
    fn offset_value(weirdness: f32, continentalness: f32) -> f32 {
        let f0 = 1.0 - (1.0 - continentalness) * 0.5;
        let f1 = 0.5 * (1.0 - continentalness);
        let f2 = (weirdness + 1.17) * 0.46082947;
        let off = f2 * f0 - f1;
        if weirdness < -0.7 {
            if off > -0.2222 {
                off
            } else {
                -0.2222
            }
        } else if off > 0.0 {
            off
        } else {
            0.0
        }
    }

    /// `createSpline_38219`
    fn create_spline_38219(&mut self, f: f32, bl: bool) -> u32 {
        let sp = self.push_node(SP_RIDGES);

        let i = Self::offset_value(-1.0, f);
        let k = Self::offset_value(1.0, f);
        let mut l = 1.0 - (1.0 - f) * 0.5;
        let u0 = 0.5 * (1.0 - f);
        l = u0 / (0.46082947 * l) - 1.17;

        if -0.65 < l && l < 1.0 {
            let u = Self::offset_value(-0.65, f);
            let p = Self::offset_value(-0.75, f);
            let q = (p - i) * 4.0;
            let r = Self::offset_value(l, f);
            let s = (k - r) / (1.0 - l);

            let (fi, fp, fu, fr1, fr2, fk) = (
                self.fix(i),
                self.fix(p),
                self.fix(u),
                self.fix(r),
                self.fix(r),
                self.fix(k),
            );
            self.add(sp, -1.0, fi, q);
            self.add(sp, -0.75, fp, 0.0);
            self.add(sp, -0.65, fu, 0.0);
            self.add(sp, l - 0.01, fr1, 0.0);
            self.add(sp, l, fr2, s);
            self.add(sp, 1.0, fk, s);
        } else {
            let u = (k - i) * 0.5;
            if bl {
                let fi = self.fix(if i > 0.2 { i } else { 0.2 });
                let fk = self.fix(k);
                let fl = self.fix(lerp_spline(0.5, i, k));
                self.add(sp, -1.0, fi, 0.0);
                self.add(sp, 0.0, fl, u);
                self.add(sp, 1.0, fk, u);
            } else {
                let fi = self.fix(i);
                let fk = self.fix(k);
                self.add(sp, -1.0, fi, u);
                self.add(sp, 1.0, fk, u);
            }
        }
        sp
    }

    /// `createFlatOffsetSpline`
    fn create_flat_offset_spline(
        &mut self,
        f: f32,
        g: f32,
        h: f32,
        i: f32,
        j: f32,
        k: f32,
    ) -> u32 {
        let sp = self.push_node(SP_RIDGES);

        let mut l = 0.5 * (g - f);
        if l < k {
            l = k;
        }
        let m = 5.0 * (h - g);

        let (ff, fg, fh, fi, fj) = (
            self.fix(f),
            self.fix(g),
            self.fix(h),
            self.fix(i),
            self.fix(j),
        );
        self.add(sp, -1.0, ff, l);
        self.add(sp, -0.4, fg, if l < m { l } else { m });
        self.add(sp, 0.0, fh, m);
        self.add(sp, 0.4, fi, 2.0 * (i - h));
        self.add(sp, 1.0, fj, 0.7 * (j - i));
        sp
    }

    /// `createLandSpline`
    #[allow(clippy::too_many_arguments)]
    fn create_land_spline(
        &mut self,
        f: f32,
        g: f32,
        h: f32,
        i: f32,
        j: f32,
        k: f32,
        bl: bool,
    ) -> u32 {
        let sp1 = self.create_spline_38219(lerp_spline(i, 0.6, 1.5), bl);
        let sp2 = self.create_spline_38219(lerp_spline(i, 0.6, 1.0), bl);
        let sp3 = self.create_spline_38219(i, bl);
        let ih = 0.5 * i;
        let sp4 = self.create_flat_offset_spline(f - 0.15, ih, ih, ih, i * 0.6, 0.5);
        let sp5 = self.create_flat_offset_spline(f, j * i, g * i, ih, i * 0.6, 0.5);
        let sp6 = self.create_flat_offset_spline(f, j, j, g, h, 0.5);
        let sp7 = self.create_flat_offset_spline(f, j, j, g, h, 0.5);

        let sp8 = self.push_node(SP_RIDGES);
        let ff = self.fix(f);
        let fh = self.fix(h + 0.07);
        self.add(sp8, -1.0, ff, 0.0);
        self.add(sp8, -0.4, SplineVal::Node(sp6), 0.0);
        self.add(sp8, 0.0, fh, 0.0);

        let sp9 = self.create_flat_offset_spline(-0.02, k, k, g, h, 0.0);
        let sp = self.push_node(SP_EROSION);
        self.add(sp, -0.85, SplineVal::Node(sp1), 0.0);
        self.add(sp, -0.7, SplineVal::Node(sp2), 0.0);
        self.add(sp, -0.4, SplineVal::Node(sp3), 0.0);
        self.add(sp, -0.35, SplineVal::Node(sp4), 0.0);
        self.add(sp, -0.1, SplineVal::Node(sp5), 0.0);
        self.add(sp, 0.2, SplineVal::Node(sp6), 0.0);
        if bl {
            self.add(sp, 0.4, SplineVal::Node(sp7), 0.0);
            self.add(sp, 0.45, SplineVal::Node(sp8), 0.0);
            self.add(sp, 0.55, SplineVal::Node(sp8), 0.0);
            self.add(sp, 0.58, SplineVal::Node(sp7), 0.0);
        }
        self.add(sp, 0.7, SplineVal::Node(sp9), 0.0);
        sp
    }

    /// `getSpline`：样条求值。`vals` 按 `[continentalness, erosion, ridges,
    /// weirdness]` 顺序给出。
    fn eval(&self, idx: u32, vals: &[f32; 4]) -> f32 {
        let sp = &self.nodes[idx as usize];
        debug_assert!(sp.len > 0 && sp.len < 12);

        let f = vals[sp.typ];
        let mut i = 0;
        while i < sp.len {
            if sp.loc[i] >= f {
                break;
            }
            i += 1;
        }
        if i == 0 || i == sp.len {
            i = i.saturating_sub(1);
            let v = self.eval_val(sp.val[i], vals);
            return v + sp.der[i] * (f - sp.loc[i]);
        }
        let g = sp.loc[i - 1];
        let h = sp.loc[i];
        let k = (f - g) / (h - g);
        let l = sp.der[i - 1];
        let m = sp.der[i];
        let n = self.eval_val(sp.val[i - 1], vals);
        let o = self.eval_val(sp.val[i], vals);
        let p = l * (h - g) - (o - n);
        let q = -m * (h - g) + (o - n);
        // C: float r = lerp(k, n, o) + k * (1.0F - k) * lerp(k, p, q);
        // lerp 为 double 函数，k*(1-k) 是 float 乘法，整体 double 求值后窄化。
        (lerp64(k, n, o) + (k * (1.0 - k)) as f64 * lerp64(k, p, q)) as f32
    }

    fn eval_val(&self, val: SplineVal, vals: &[f32; 4]) -> f32 {
        match val {
            SplineVal::Fix(v) => v,
            SplineVal::Node(idx) => self.eval(idx, vals),
        }
    }
}

/// `rng.h` 的 `lerp` 只有 **double** 版本；spline 代码中的 float 实参先提升
/// 为 double 计算。该 helper 保持这一语义（返回 double），以逐位对齐 C。
#[inline(always)]
fn lerp64(part: f32, from: f32, to: f32) -> f64 {
    from as f64 + part as f64 * (to as f64 - from as f64)
}

/// 同上，但按 C 中赋值回 float 变量的语义窄化为 `f32`。
#[inline(always)]
fn lerp_spline(part: f32, from: f32, to: f32) -> f32 {
    lerp64(part, from, to) as f32
}

/// `init_climate_seed` 的各气候参数配置：
/// `(lo 盐, hi 盐, 振幅表, omin)`。盐为 `md5("minecraft:<name>")` 前 16 字节。
struct ClimateSpec {
    salt_lo: u64,
    salt_hi: u64,
    amps: &'static [f64],
    omin: i32,
}

/// `large` 为 normal / large biomes 两种盐各一份。
fn climate_spec(nptype: usize, large: bool) -> ClimateSpec {
    match nptype {
        NP_SHIFT => ClimateSpec {
            // md5 "minecraft:offset"
            salt_lo: 0x080518cf6af25384,
            salt_hi: 0x3f3dfb40a54febd5,
            amps: &[1.0, 1.0, 1.0, 0.0],
            omin: -3,
        },
        NP_TEMPERATURE => ClimateSpec {
            // md5 "minecraft:temperature" / "minecraft:temperature_large"
            salt_lo: if large { 0x944b0073edf549db } else { 0x5c7e6b29735f0d7f },
            salt_hi: if large { 0x4ff44347e9d22b96 } else { 0xf7d86f1bbc734988 },
            amps: &[1.5, 0.0, 1.0, 0.0, 0.0, 0.0],
            omin: if large { -12 } else { -10 },
        },
        NP_HUMIDITY => ClimateSpec {
            // md5 "minecraft:vegetation" / "minecraft:vegetation_large"
            salt_lo: if large { 0x71b8ab943dbd5301 } else { 0x81bb4d22e8dc168e },
            salt_hi: if large { 0xbb63ddcf39ff7a2b } else { 0xf1c8b4bea16303cd },
            amps: &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            omin: if large { -10 } else { -8 },
        },
        NP_CONTINENTALNESS => ClimateSpec {
            // md5 "minecraft:continentalness" / "minecraft:continentalness_large"
            salt_lo: if large { 0x9a3f51a113fce8dc } else { 0x83886c9d0ae3a662 },
            salt_hi: if large { 0xee2dbd157e5dcdad } else { 0xafa638a61b42e8ad },
            amps: &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0],
            omin: if large { -11 } else { -9 },
        },
        NP_EROSION => ClimateSpec {
            // md5 "minecraft:erosion" / "minecraft:erosion_large"
            salt_lo: if large { 0x8c984b1f8702a951 } else { 0xd02491e6058f6fd8 },
            salt_hi: if large { 0xead7b1f92bae535f } else { 0x4792512c94c17a80 },
            amps: &[1.0, 1.0, 0.0, 1.0, 1.0],
            omin: if large { -11 } else { -9 },
        },
        NP_WEIRDNESS => ClimateSpec {
            // md5 "minecraft:ridge"
            salt_lo: 0xefc8ef4d36102b34,
            salt_hi: 0x1beeeb324a0f24ea,
            amps: &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0],
            omin: -7,
        },
        _ => unreachable!("unsupported climate parameter"),
    }
}

/// 1.18+ 主世界群系气候噪声（对应 cubiomes `BiomeNoise`）。
///
/// 用法：`BiomeNoise::new(mc)`（构建 spline 表，对应 `initBiomeNoise`）
/// 后调用 [`BiomeNoise::set_biome_seed`]（对应 `setBiomeSeed`）注入种子，
/// 再用 [`BiomeNoise::sample_np`] 采样。`set_biome_seed` 可重复调用以更换种子。
#[derive(Clone, Debug)]
pub struct BiomeNoise {
    /// 6 个气候参数的 DoublePerlin 噪声（按 `NP_*` 下标）。
    /// 未调用 [`BiomeNoise::set_biome_seed`] 前为空噪声（采样无意义）。
    pub climate: [DoublePerlinNoise; NP_MAX],
    splines: SplineBuilder,
    /// spline 根节点下标（`initBiomeNoise` 的 `bn->sp`）。
    sp: u32,
    mc: crate::version::McVersion,
}

impl BiomeNoise {
    /// `initBiomeNoise`：构建 depth 偏移的 spline 表并记录版本。
    ///
    /// spline 表本身与版本无关；版本差异体现在群系树查表
    /// （[`crate::generator::v1_18::climate_to_biome`]）。
    pub fn new(mc: crate::version::McVersion) -> Self {
        let mut ss = SplineBuilder { nodes: Vec::new() };
        let sp = ss.push_node(SP_CONTINENTALNESS);

        let sp1 = ss.create_land_spline(-0.15, 0.00, 0.0, 0.1, 0.00, -0.03, false);
        let sp2 = ss.create_land_spline(-0.10, 0.03, 0.1, 0.1, 0.01, -0.03, false);
        let sp3 = ss.create_land_spline(-0.10, 0.03, 0.1, 0.7, 0.01, -0.03, true);
        let sp4 = ss.create_land_spline(-0.05, 0.03, 0.1, 1.0, 0.01, 0.01, true);

        let f044 = ss.fix(0.044);
        let fn2222a = ss.fix(-0.2222);
        let fn2222b = ss.fix(-0.2222);
        let fn12a = ss.fix(-0.12);
        let fn12b = ss.fix(-0.12);
        ss.add(sp, -1.10, f044, 0.0);
        ss.add(sp, -1.02, fn2222a, 0.0);
        ss.add(sp, -0.51, fn2222b, 0.0);
        ss.add(sp, -0.44, fn12a, 0.0);
        ss.add(sp, -0.18, fn12b, 0.0);
        ss.add(sp, -0.16, SplineVal::Node(sp1), 0.0);
        ss.add(sp, -0.15, SplineVal::Node(sp1), 0.0);
        ss.add(sp, -0.10, SplineVal::Node(sp2), 0.0);
        ss.add(sp, 0.25, SplineVal::Node(sp3), 0.0);
        ss.add(sp, 1.00, SplineVal::Node(sp4), 0.0);

        // 空的占位噪声，set_biome_seed 时替换。
        let empty = || DoublePerlinNoise {
            amplitude: 0.0,
            oct_a: super::OctaveNoise { octaves: Vec::new() },
            oct_b: super::OctaveNoise { octaves: Vec::new() },
        };
        BiomeNoise {
            climate: [empty(), empty(), empty(), empty(), empty(), empty()],
            splines: ss,
            sp,
            mc,
        }
    }

    /// 版本（`initBiomeNoise` 记录的 `bn->mc`）。
    pub fn mc(&self) -> crate::version::McVersion {
        self.mc
    }

    /// `setBiomeSeed`：注入世界种子，派生全部 6 个气候噪声。
    ///
    /// `large` 对应 cubiomes `LARGE_BIOMES` 标志（大型生物群系世界类型）。
    pub fn set_biome_seed(&mut self, seed: u64, large: bool) {
        let mut pxr = Xoroshiro::new(seed);
        let xlo = pxr.next_long();
        let xhi = pxr.next_long();

        for i in 0..NP_MAX {
            let spec = climate_spec(i, large);
            let mut pxr = Xoroshiro::from_state(xlo ^ spec.salt_lo, xhi ^ spec.salt_hi);
            self.climate[i] =
                DoublePerlinNoise::new_xoroshiro(&mut pxr, spec.amps, spec.omin, -1);
        }
    }

    /// `sampleBiomeNoise` 的噪声采样部分：返回 6 个 ×10000 定点气候值
    /// `[temperature, humidity, continentalness, erosion, depth, weirdness]`。
    ///
    /// 坐标为 1:4 群系比例。`flags` 见 [`SAMPLE_NO_SHIFT`] /
    /// [`SAMPLE_NO_DEPTH`]。群系判定由调用方用返回值进一步完成
    /// （[`crate::generator::v1_18::climate_to_biome`]）。
    pub fn sample_np(&self, x: i32, y: i32, z: i32, flags: u32) -> [i64; NP_MAX] {
        let mut px = x as f64;
        let mut pz = z as f64;
        if flags & SAMPLE_NO_SHIFT == 0 {
            px += self.climate[NP_SHIFT].sample(x as f64, 0.0, z as f64) * 4.0;
            pz += self.climate[NP_SHIFT].sample(z as f64, x as f64, 0.0) * 4.0;
        }

        let c = self.climate[NP_CONTINENTALNESS].sample(px, 0.0, pz) as f32;
        let e = self.climate[NP_EROSION].sample(px, 0.0, pz) as f32;
        let w = self.climate[NP_WEIRDNESS].sample(px, 0.0, pz) as f32;

        let mut d = 0.0f32;
        if flags & SAMPLE_NO_DEPTH == 0 {
            let np_param = [
                c,
                e,
                -3.0 * ((w.abs() - 0.6666667).abs() - 0.33333334),
                w,
            ];
            let off = self.splines.eval(self.sp, &np_param) + 0.015;
            // C: d = 1.0 - (y * 4) / 128.0 - 83.0/160.0 + off（double 计算，
            // off 为 float 提升），最后赋给 float d。
            d = (1.0 - (y * 4) as f64 / 128.0 - 83.0 / 160.0 + off as f64) as f32;
        }

        let t = self.climate[NP_TEMPERATURE].sample(px, 0.0, pz) as f32;
        let h = self.climate[NP_HUMIDITY].sample(px, 0.0, pz) as f32;

        [
            (10000.0f32 * t) as i64,
            (10000.0f32 * h) as i64,
            (10000.0f32 * c) as i64,
            (10000.0f32 * e) as i64,
            (10000.0f32 * d) as i64,
            (10000.0f32 * w) as i64,
        ]
    }

    /// 直接对 spline 表求值（测试用，对应 `getSpline`）。
    #[cfg(test)]
    pub(crate) fn eval_spline(&self, vals: &[f32; 4]) -> f32 {
        self.splines.eval(self.sp, vals)
    }
}

impl Default for BiomeNoise {
    fn default() -> Self {
        Self::new(crate::version::McVersion::NEWEST)
    }
}

#[cfg(test)]
mod tests;
