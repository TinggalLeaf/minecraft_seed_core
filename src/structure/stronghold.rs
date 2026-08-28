//! 要塞定位：移植 cubiomes `finders.c` 的 `initFirstStronghold` /
//! `nextStronghold` / `isStrongholdBiome`。
//!
//! 1.9+ 生成 128 座要塞（8 环带），1.8 生成 3 座。首座要塞的近似位置
//! （±112 方块）只取决于世界种子低 48 位；精确位置需要主世界群系检查
//! （[`StrongholdIter::next`]）。

use crate::biome::is_oceanic;
use crate::biome::BiomeId;
use crate::generator::Generator;
use crate::rng::JavaRandom;
use crate::version::McVersion;

use super::region::Pos;
use super::spawn::locate_biome;
use super::viability::is_overworld;

/// `PI`（与 C 相同）。
const PI: f64 = std::f64::consts::PI;

/// `isStrongholdBiome`：要塞能否生成于该群系。
pub fn is_stronghold_biome(mc: McVersion, id: i32) -> bool {
    use BiomeId::*;
    if !is_overworld(mc, id) {
        return false;
    }
    if is_oceanic(id) {
        return false;
    }
    match BiomeId::from_i32(id) {
        Some(Plains | MushroomFields | TaigaHills) => mc >= McVersion::V1_7,
        Some(Swamp) => mc <= McVersion::V1_6,
        Some(River | FrozenRiver | Beach | SnowyBeach | SwampHills) => false,
        Some(MushroomFieldShore) => mc >= McVersion::V1_13,
        Some(StoneShore) => mc <= McVersion::V1_17,
        // 模拟 MC-199298
        Some(BambooJungle | BambooJungleHills) => {
            mc <= McVersion::V1_15 || mc >= McVersion::V1_18
        }
        Some(MangroveSwamp | DeepDark) => false,
        _ => true,
    }
}

/// 要塞迭代器（对应 C `StrongholdIter`）。
///
/// 字段语义与 C 一致：`pos` 为当前要塞精确位置（[`StrongholdIter::next`]
/// 调用后有效），`nextapprox` 为下一座的近似位置（±112 方块）。
#[derive(Clone, Debug)]
pub struct StrongholdIter {
    /// 当前要塞的精确位置。
    pub pos: Pos,
    /// 下一座要塞的近似位置（±112 方块）。
    pub nextapprox: Pos,
    /// 要塞序号计数器。
    pub index: i32,
    /// 当前环带编号。
    pub ringnum: i32,
    /// 当前环带的最大序号。
    pub ringmax: i32,
    /// 环带内序号。
    pub ringidx: i32,
    /// 环带内的下一个角度。
    pub angle: f64,
    /// 下一个距原点距离（单位：区块）。
    pub dist: f64,
    /// LCG 状态（C 的 `rnds`，48 位）。
    pub rnds: JavaRandom,
    /// 版本。
    pub mc: McVersion,
}

/// `initFirstStronghold`：首座要塞的近似位置（±112 方块，无需群系检查，
/// 只看种子低 48 位）。
///
/// 只需要位置时调用本函数；需要精确位置与后续要塞时用
/// [`StrongholdIter::new`] + [`StrongholdIter::next`]。
pub fn init_first_stronghold(mc: McVersion, s48: u64) -> Pos {
    let mut rnds = JavaRandom::new(s48 as i64);
    let angle = 2.0 * PI * rnds.next_double();
    let dist = if mc >= McVersion::V1_9 {
        (4.0 * 32.0) + (rnds.next_double() - 0.5) * 32.0 * 2.5
    } else {
        (1.25 + rnds.next_double()) * 32.0
    };
    Pos {
        x: ((angle.cos() * dist).round() as i32) * 16 + 8,
        z: ((angle.sin() * dist).round() as i32) * 16 + 8,
    }
}

impl StrongholdIter {
    /// `initFirstStronghold`（`sh != NULL`）：初始化要塞迭代器。
    pub fn new(mc: McVersion, s48: u64) -> Self {
        let mut rnds = JavaRandom::new(s48 as i64);
        let angle = 2.0 * PI * rnds.next_double();
        let dist = if mc >= McVersion::V1_9 {
            (4.0 * 32.0) + (rnds.next_double() - 0.5) * 32.0 * 2.5
        } else {
            (1.25 + rnds.next_double()) * 32.0
        };
        let nextapprox = Pos {
            x: ((angle.cos() * dist).round() as i32) * 16 + 8,
            z: ((angle.sin() * dist).round() as i32) * 16 + 8,
        };
        StrongholdIter {
            pos: Pos::default(),
            nextapprox,
            index: 0,
            ringnum: 0,
            ringmax: 3,
            ringidx: 0,
            angle,
            dist,
            rnds,
            mc,
        }
    }

    /// `nextStronghold`：做群系检查、求当前要塞精确位置与下一座近似位置。
    ///
    /// `g` 应为已按主世界初始化的生成器；1.19.3+（`mc > 1.19.2`）可传
    /// `None`，跳过群系检查只迭代近似位置。
    ///
    /// 返回这座之后还剩多少座要塞（C 的返回值）。Beta 1.7 及更早没有
    /// 要塞（C 的 `else return 0` 分支）：直接返回 0，不消耗随机数。
    pub fn next(&mut self, g: Option<&Generator>) -> i32 {
        if self.mc <= McVersion::B1_7 {
            return 0;
        }
        // 要塞可行群系集合（每次重建，与 C 一致；成本可忽略）
        let mut valid_b = 0u64;
        let mut valid_m = 0u64;
        for i in 0..64i32 {
            if is_stronghold_biome(self.mc, i) {
                valid_b |= 1u64 << i;
            }
            if is_stronghold_biome(self.mc, i + 128) {
                valid_m |= 1u64 << i;
            }
        }

        if self.mc > McVersion::V1_19_2 {
            match g {
                Some(g) => {
                    let mut lbr = JavaRandom::new(self.rnds.next_long());
                    self.pos = locate_biome(
                        g,
                        self.nextapprox.x,
                        0,
                        self.nextapprox.z,
                        112,
                        valid_b,
                        valid_m,
                        &mut lbr,
                        None,
                    );
                }
                None => {
                    self.rnds.next_long();
                    self.pos = self.nextapprox;
                }
            }
        } else {
            let g = g.expect("StrongholdIter::next: B1.8–1.19.2 需要主世界生成器");
            self.pos = locate_biome(
                g,
                self.nextapprox.x,
                0,
                self.nextapprox.z,
                112,
                valid_b,
                valid_m,
                &mut self.rnds,
                None,
            );
        }
        // 楼梯间位于区块内 (4, 4)
        self.pos.x = (self.pos.x & !15) + 4;
        self.pos.z = (self.pos.z & !15) + 4;

        self.ringidx += 1;
        self.angle += 2.0 * PI / self.ringmax as f64;

        if self.ringidx == self.ringmax {
            self.ringnum += 1;
            self.ringidx = 0;
            self.ringmax += 2 * self.ringmax / (self.ringnum + 1);
            if self.ringmax > 128 - self.index {
                self.ringmax = 128 - self.index;
            }
            self.angle += self.rnds.next_double() * PI * 2.0;
        }

        if self.mc >= McVersion::V1_9 {
            self.dist = (4.0 * 32.0)
                + (6.0 * self.ringnum as f64 * 32.0)
                + (self.rnds.next_double() - 0.5) * 32.0 * 2.5;
        } else {
            self.dist = (1.25 + self.rnds.next_double()) * 32.0;
        }

        self.nextapprox.x = ((self.angle.cos() * self.dist).round() as i32) * 16 + 8;
        self.nextapprox.z = ((self.angle.sin() * self.dist).round() as i32) * 16 + 8;
        self.index += 1;

        (if self.mc >= McVersion::V1_9 { 128 } else { 3 }) - (self.index - 1)
    }
}
