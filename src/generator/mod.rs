//! 生物群系生成器，按版本模块化：
//! - [`v1_18`]：多噪声（multi-noise）群系源（主世界 1.18+）
//! - [`layers`]：分层（LayerStack）群系源（主世界 B1.8–1.17）
//! - [`nether`]：下界多噪声群系源（1.16+）
//! - [`end`]：末地 simplex 高地噪声群系源（1.9+）
//! - [`tables`]：1.18+ 群系参数搜索树数据（自动生成）
//! - [`voronoi`]：1:1 比例的 voronoi 缩放助手
//! - [`surface`]：地表高度近似 `map_approx_height`（`ApproxHeight`，
//!   1:4 比例；对应 `mapApproxHeight`/`mapEndSurfaceHeight`）
//!
//! 统一入口为 [`Generator`]：`Generator::new(version)`
//! → `with_seed(dim, seed)` → `get_biome(x, y, z)` / `gen_biomes(range)`。
//!
//! ## 覆盖范围
//!
//! - 主世界：**Beta 1.7–1.21.x**（B1.7- 气候噪声，B1.8–1.17 分层
//!   LayerStack，1.18+ 多噪声 + 群系树）。分层路径仅支持 scale
//!   1/4/16/64/256（`getLayerForScale` 的入口层）；beta 路径支持任意
//!   2 的幂 scale（`genBiomeNoiseBetaScaled`）。
//! - 下界：**1.16.1+**（多噪声）；更早版本按 cubiomes 行为填充
//!   `nether_wastes`。
//! - 末地：**1.9+**，scale 1/4/16/64+（scale 1 为 voronoi 缩放：1.14- 平面
//!   旧算法，1.15+ 逐 y 层的 SHA 变体）。

pub mod end;
pub mod layers;
pub mod nether;
pub mod surface;
pub mod tables;
pub mod v1_18;
pub mod voronoi;

pub use end::EndNoise;
pub use nether::NetherNoise;
pub use surface::ApproxHeight;

#[cfg(test)]
mod tests;

use std::cell::OnceCell;

use crate::biome::BiomeId;
use crate::noise::beta::{gen_biome_noise_beta_scaled, BiomeNoiseBeta, SurfaceNoiseBeta};
use crate::noise::biome_noise::BiomeNoise;
use crate::noise::surface::SurfaceNoise;
use crate::version::{Dimension, McVersion};

/// 群系生成区域（对应 cubiomes `Range`）。
///
/// - `scale`：水平比例因子，支持 1、4、16、64、256（1:1 为方块级，
///   需要 voronoi；4 为默认群系比例）。
/// - `x, z`：西北角坐标（按 `scale` 比例）。
/// - `sx, sz`：水平尺寸（应为正数）。
/// - `y, sy`：垂直位置与尺寸；`scale != 1` 时垂直比例为 1:4，`sy <= 0`
///   视为 1。
///
/// 输出索引为 `out[i_y*sx*sz + i_z*sx + i_x]`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub scale: i32,
    pub x: i32,
    pub z: i32,
    pub sx: i32,
    pub sz: i32,
    pub y: i32,
    pub sy: i32,
}

impl Range {
    /// 2D 区域（`y = 0, sy = 1`）。
    pub fn new(scale: i32, x: i32, z: i32, sx: i32, sz: i32) -> Self {
        Range {
            scale,
            x,
            z,
            sx,
            sz,
            y: 0,
            sy: 1,
        }
    }

    /// 设置垂直范围（体积生成）。
    pub fn with_y(mut self, y: i32, sy: i32) -> Self {
        self.y = y;
        self.sy = sy;
        self
    }
}

/// 生物群系生成器（对应 cubiomes `Generator`：分层路径为
/// `setupLayerStack` + `setLayerSeed` + `genArea`；1.18+ 路径为
/// `setupGenerator` + `applySeed` + `getBiomeAt` + `genBiomes`）。
///
/// 用法：
/// ```
/// use minecraft_seed_core::generator::{Generator, Range};
/// use minecraft_seed_core::{Dimension, McVersion};
///
/// let g = Generator::new(McVersion::V1_20).with_seed(Dimension::Overworld, 12345);
/// let biome = g.get_biome(0, 0, 0); // 1:4 比例坐标
/// let area = g.gen_biomes(Range::new(4, -4, -4, 8, 8));
///
/// // 1.7–1.17 分层群系源
/// let g = Generator::new(McVersion::V1_12).with_seed(Dimension::Overworld, 12345);
/// let biome = g.get_biome(0, 0, 0);
/// ```
#[derive(Clone, Debug)]
pub struct Generator {
    mc: McVersion,
    dim: Option<Dimension>,
    seed: u64,
    sha: u64,
    large: bool,
    bn: Option<BiomeNoise>,
    nn: Option<NetherNoise>,
    en: Option<EndNoise>,
    ls: Option<layers::LayerStack>,
    /// Beta 1.7 及更早的气候噪声群系源（`BiomeNoiseBeta`）。
    bnb: Option<BiomeNoiseBeta>,
    /// 地表噪声（`initSurfaceNoise`），首次使用时惰性初始化。
    sn: OnceCell<SurfaceNoise>,
}

impl Generator {
    /// `setupGenerator`：按版本初始化生成器（不含种子）。
    ///
    /// 主世界 1.18+ 会构建群系噪声的 spline 表（`initBiomeNoise`）；
    /// 1.7–1.17 的分层层栈推迟到 [`Generator::with_seed`] 构建
    /// （需要 `large` 标志，对应 C 中 `setupGenerator` 的 flags 参数）。
    pub fn new(mc: McVersion) -> Self {
        Generator {
            mc,
            dim: None,
            seed: 0,
            sha: 0,
            large: false,
            bn: if mc.has_multi_noise_biomes() {
                Some(BiomeNoise::new(mc))
            } else {
                None
            },
            nn: None,
            en: None,
            ls: None,
            bnb: if mc <= McVersion::B1_7 {
                Some(BiomeNoiseBeta::new_uninit())
            } else {
                None
            },
            sn: OnceCell::new(),
        }
    }

    /// 设置 `LARGE_BIOMES` 标志（大型生物群系世界类型）。
    /// 须在 [`Generator::with_seed`] 之前调用才生效。
    pub fn with_large_biomes(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// `applySeed`：注入维度与世界种子。可重复调用以更换种子/维度。
    pub fn with_seed(mut self, dim: Dimension, seed: u64) -> Self {
        self.dim = Some(dim);
        self.seed = seed;
        // 地表噪声缓存随种子/维度失效
        self.sn = OnceCell::new();

        match dim {
            Dimension::Overworld => {
                if self.mc.has_multi_noise_biomes() {
                    let bn = self.bn.as_mut().unwrap();
                    bn.set_biome_seed(seed, self.large);
                } else if self.mc <= McVersion::B1_7 {
                    // B1.7-：气候噪声群系源（setBetaBiomeSeed）
                    self.bnb.as_mut().unwrap().set_beta_biome_seed(seed);
                } else {
                    // B1.8–1.17：分层群系源（setLayerSeed）
                    let mut ls = layers::LayerStack::new(self.mc, self.large);
                    ls.set_world_seed(seed);
                    self.ls = Some(ls);
                }
            }
            Dimension::Nether => {
                if self.mc >= McVersion::V1_16_1 {
                    self.nn = Some(NetherNoise::new(seed));
                }
            }
            Dimension::End => {
                if self.mc >= McVersion::V1_9 {
                    self.en = Some(EndNoise::new(self.mc, seed));
                }
            }
        }
        if self.mc >= McVersion::V1_15 {
            // 1.15–1.17 主世界的 sha 即 L_VORONOI_1 的 startSalt
            // （同为 getVoronoiSHA(seed)，见 C `applySeed`）。
            self.sha = voronoi::get_voronoi_sha(seed);
        }
        self
    }

    /// 维度（未调用 [`Generator::with_seed`] 前为 `None`）。
    pub fn dim(&self) -> Option<Dimension> {
        self.dim
    }

    /// 世界种子。
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// 版本。
    pub fn version(&self) -> McVersion {
        self.mc
    }

    /// 1.18+ 主世界的群系噪声（测试与调试用）。
    pub fn biome_noise(&self) -> Option<&BiomeNoise> {
        self.bn.as_ref()
    }

    /// 末地群系/高度噪声（1.9+ 且维度为末地时存在）。
    pub fn end_noise(&self) -> Option<&EndNoise> {
        self.en.as_ref()
    }

    /// （crate 内部）设置/恢复结构可行性过滤器（对应 C
    /// `isViableStructurePos` 里保存-替换-恢复 `L_BIOME_256`/`L_SHORE_16`
    /// 的 getMap）。返回旧值以便嵌套调用恢复。
    pub(crate) fn set_viable_filter(
        &self,
        f: Option<(crate::structure::StructureType, McVersion)>,
    ) -> Option<(crate::structure::StructureType, McVersion)> {
        let ls = self.ls.as_ref().expect("Generator: 当前版本/维度无分层群系源");
        ls.viable_filter.replace(f)
    }

    /// （crate 内部）带可行性剪枝的 [`Generator::gen_biomes`]：
    /// 过滤器触发了粗层剪枝（C 的 err 传播）时返回 `None`。
    /// 未设置过滤器（或非 1.7–1.17 主世界）时与 `gen_biomes` 完全一致。
    pub(crate) fn viable_gen_biomes(&self, r: Range) -> Option<Vec<BiomeId>> {
        if let Some(ls) = &self.ls {
            ls.viable_failed.set(false);
        }
        let out = self.gen_biomes(r);
        match &self.ls {
            Some(ls) if ls.viable_failed.get() => None,
            _ => Some(out),
        }
    }

    /// （crate 内部）带可行性剪枝的旧版主世界单点查询（1.7–1.17 分层
    /// 群系源；C `getBiomeAt` 在 `isViableStructurePos` 中的行为）：
    /// 被剪枝时返回 -1（C 的 `none`）。
    pub(crate) fn viable_layered_biome_at(&self, layer: layers::LayerId, x: i32, z: i32) -> i32 {
        let ls = self.ls.as_ref().expect("Generator: 当前版本/维度无分层群系源");
        ls.viable_failed.set(false);
        let out = ls.gen_area(layer as usize, x, z, 1, 1);
        if ls.viable_failed.get() {
            -1
        } else {
            out[0]
        }
    }

    /// `getBiomeAt` 的常用形式：1:4 群系比例下单点查询。
    ///
    /// 坐标 `(x, y, z)` 均为 1:4 比例（即方块坐标除以 4）。
    pub fn get_biome(&self, x: i32, y: i32, z: i32) -> BiomeId {
        self.gen_biomes(Range::new(4, x, z, 1, 1).with_y(y, 1))[0]
    }

    /// `genBiomes`：区域批量生成，返回 `sx*sy*sz` 个群系。
    ///
    /// # Panics
    ///
    /// - 未调用 [`Generator::with_seed`]；
    /// - 主世界 B1.8–1.17 时 `scale` 不是 1/4/16/64/256；B1.7- 时
    ///   `scale` 非正或非 2 的幂。
    pub fn gen_biomes(&self, r: Range) -> Vec<BiomeId> {
        match self.dim.expect("Generator: call with_seed() first") {
            Dimension::Overworld => {
                if self.mc.has_multi_noise_biomes() {
                    let bn = self.bn.as_ref().unwrap();
                    v1_18::gen_biome_noise_scaled(bn, r, self.sha)
                } else if self.mc <= McVersion::B1_7 {
                    // B1.7-：气候噪声群系源；每次调用初始化地表噪声
                    // （C 的 genBiomes 对 beta 同样现初始化；行为等价于
                    // C 默认 flags=0，即启用海洋映射）
                    let bnb = self.bnb.as_ref().expect("Generator: call with_seed() first");
                    let snb = SurfaceNoiseBeta::new(self.seed);
                    let mut plane = vec![0i32; (r.sx * r.sz) as usize];
                    gen_biome_noise_beta_scaled(bnb, Some(&snb), &mut plane, Range { sy: 1, ..r });
                    // 主世界无垂直噪声，2D 平面沿 y 复制
                    let sy = if r.sy == 0 { 1 } else { r.sy };
                    let mut out = Vec::with_capacity((r.sx * r.sz * sy) as usize);
                    for _ in 0..sy {
                        out.extend(plane.iter().map(|&v| {
                            BiomeId::from_i32(v)
                                .unwrap_or_else(|| panic!("Generator: beta 群系源产生未知群系 ID {v}"))
                        }));
                    }
                    out
                } else {
                    // B1.8–1.17：分层群系源；主世界无垂直噪声，2D 平面沿 y 复制
                    let ls = self.ls.as_ref().expect("Generator: call with_seed() first");
                    layers::gen_biomes(ls, r)
                }
            }
            Dimension::Nether => match &self.nn {
                Some(nn) => nn.gen_scaled(r, self.mc, self.sha),
                None => {
                    // mc <= 1.15：下界恒为 nether_wastes
                    let sy = if r.sy == 0 { 1 } else { r.sy };
                    vec![BiomeId::NetherWastes; (r.sx * sy * r.sz) as usize]
                }
            },
            Dimension::End => match &self.en {
                Some(en) => en.gen_scaled(r, self.mc),
                None => {
                    // mc <= 1.8：末地恒为 the_end
                    let sy = if r.sy == 0 { 1 } else { r.sy };
                    vec![BiomeId::TheEnd; (r.sx * sy * r.sz) as usize]
                }
            },
        }
    }
}
