//! 种子搜索 API：逐一对齐 mcseedmap.com 的 `api.wasm` 中
//! `find_biomes` / `find_structures` / `find_biomes_with_structure`
//! （func130/105/104）的搜索语义与返回值。
//!
//! 语义要点（均由网站 wasm 反编译确认并 golden 对拍）：
//! - 群系匹配用 64 位掩码，`1 << (id & 63)`（id ≥ 64 的群系按 mod 64 回绕，
//!   与 wasm `i64.shl` 一致——这是网站搜索不支持高 id 群系的固有怪癖，
//!   为保持结果一致原样复刻）；
//! - 结构搜索是「48 位基值 + 高 16 位后缀」两段式：候选位置只由种子低 48 位
//!   决定，群系可行性需要完整 64 位种子，返回值把所需高 16 位打包进
//!   bit 48..64（`full = (k << 48) | h`）。

use crate::generator::{Generator, Range};
use crate::structure::{get_structure_pos, is_viable_structure_pos};
use crate::{Dimension, McVersion, StructureType};

mod masks;
pub use masks::BiomeSet;

/// 计算区域的群系位掩码双字：`m1` 为 id < 128 的位 OR（位 id & 63），
/// `m2` 为 id ≥ 128 的位 OR（位 (id - 128) & 63），与网站掩码模型一致。
fn area_mask(g: &Generator, range: Range) -> (u64, u64) {
    let mut m1 = 0u64;
    let mut m2 = 0u64;
    for id in g.gen_biomes(range) {
        let id = id as i32;
        if id < 128 {
            m1 |= 1u64 << ((id as u32) & 63);
        } else {
            m2 |= 1u64 << (((id - 128) as u32) & 63);
        }
    }
    (m1, m2)
}

/// `find_biomes`（api.wasm func130）：从 `start_seed` 起向上扫描，
/// 返回第一个使 `area` 内出现 `biomes` 中**全部**群系的种子。
///
/// `x, z, sx, sz` 为 scale-4 群系坐标；`y_height` 为方块高度（内部 `/4`，
/// 与网站传参一致）。`biomes` 中支持网站同样的类别扩展（见 [`BiomeSet`]）。
#[allow(clippy::too_many_arguments)]
pub fn find_biomes(
    mc: McVersion,
    dim: Dimension,
    biomes: &[i32],
    x: i32,
    z: i32,
    sx: i32,
    sz: i32,
    y_height: i32,
    start_seed: i64,
) -> i64 {
    let set = BiomeSet::parse(biomes);
    let range = Range::new(4, x, z, sx, sz).with_y(y_height / 4, 1);
    let proto = Generator::new(mc);
    let mut seed = start_seed;
    loop {
        let g = proto.clone().with_seed(dim, seed as u64);
        let (m1, m2) = area_mask(&g, range);
        if set.matches(m1, m2) {
            return seed;
        }
        seed = seed.wrapping_add(1);
    }
}

/// `find_structures`（api.wasm func105）：48 位基值 + 高 16 位两段式结构搜索。
///
/// 从 `start_seed`（48 位基值 `h`）起向上扫描：
/// 1. 计算 region (0,0) 的结构候选位置（只依赖种子低 48 位）；该 region 不生成
///    （稀有度等）时跳过；
/// 2. 候选位置必须落在 `[x-range, x+range] × [z-range, z+range]`（方块坐标，
///    含边界，`range` 单位是方块）；
/// 3. 在高 16 位 `k ∈ [0, 65536)` 中返回首个使 `full = (k << 48) | h` 通过
///    群系可行性检查的完整种子。
///
/// 返回值的低 48 位是位置基值，bit 48..64 是可行性后缀——这正是网站
/// `findStructures` 返回大数值的原因。
pub fn find_structures(
    mc: McVersion,
    dim: Dimension,
    stype: StructureType,
    x: i32,
    z: i32,
    range: i32,
    start_seed: i64,
) -> i64 {
    let proto = Generator::new(mc);
    let mut h = start_seed;
    loop {
        // 外层：region (0,0) 候选（f_sa）
        let pos = get_structure_pos(stype, mc, h as u64, 0, 0);
        if let Some(pos) = pos
            && pos.x >= x - range
            && pos.x <= x + range
            && pos.z >= z - range
            && pos.z <= z + range
        {
                // 内层：高 16 位后缀（f_ra 需要完整种子）
                for k in 0..65536u64 {
                    let full = (k << 48) | (h as u64);
                    let g = proto.clone().with_seed(dim, full);
                    if is_viable_structure_pos(stype, &g, pos.x, pos.z, 0) != 0 {
                        return full as i64;
                    }
                }
        }
        h = h.wrapping_add(1);
    }
}

/// `find_biomes_with_structure`（api.wasm func104）：
/// 结构搜索与 [`find_structures`] 相同（`range` 为方块半径，结构候选须在
/// ±range 方块内），但额外要求：以 `(x, z)` 为中心、`range` 方块为半宽的
/// scale-4 正方形区域内包含 `biomes` 的全部群系（y = `y_height / 4`）。
///
/// 结构可行性与群系条件在同一完整种子上判定，返回打包种子（同
/// [`find_structures`]）。
#[allow(clippy::too_many_arguments)]
pub fn find_biomes_with_structure(
    mc: McVersion,
    dim: Dimension,
    stype: StructureType,
    biomes: &[i32],
    x: i32,
    z: i32,
    range: i32,
    y_height: i32,
    start_seed: i64,
) -> i64 {
    let set = BiomeSet::parse(biomes);
    let c = range / 4;
    let area = Range::new(4, x - c, z - c, 2 * c, 2 * c).with_y(y_height / 4, 1);
    let proto = Generator::new(mc);
    let mut h = start_seed;
    loop {
        let pos = get_structure_pos(stype, mc, h as u64, 0, 0);
        if let Some(pos) = pos
            && pos.x >= x - range
            && pos.x <= x + range
            && pos.z >= z - range
            && pos.z <= z + range
        {
                for k in 0..65536u64 {
                    let full = (k << 48) | (h as u64);
                    let g = proto.clone().with_seed(dim, full);
                    if is_viable_structure_pos(stype, &g, pos.x, pos.z, 0) != 0 {
                        let (m1, m2) = area_mask(&g, area);
                        if set.matches(m1, m2) {
                            return full as i64;
                        }
                    }
                }
        }
        h = h.wrapping_add(1);
    }
}
