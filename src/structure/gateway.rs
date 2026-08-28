//! 末地折跃门落点：移植 cubiomes `finders.c` 的 `applyEndIslandHeight` /
//! `mapEndIslandHeight` / `getLinkedGatewayChunk` / `getLinkedGatewayPos`。
//!
//! 折跃门把玩家传送到以原点为圆心、半径 1024 方块方向上的最近非空末地
//! chunk；`get_linked_gateway_pos` 再在该 chunk 附近选取最高点作为落点。
//! 依赖 [`crate::noise::SurfaceNoise`]（末地地形密度噪声）与
//! [`crate::generator::EndNoise`]。

use crate::generator::end::floordiv;
use crate::generator::EndNoise;
use crate::noise::SurfaceNoise;
use crate::version::McVersion;

use super::region::{get_end_islands, EndIsland, Pos};
use super::viability::is_end_chunk_empty;

/// `applyEndIslandHeight`：把单个末地小岛的高度抬升到高度图 `y` 中。
///
/// `y` 索引为 `y[(j - z) * w + (i - x)]`（`(x, z)` 为西北角，`scale`
/// 缩放）；小岛圆盘（半径 `r + 1` 的平方判定）内的格子高度取
/// `max(当前值, 小岛 y)`。
fn apply_end_island_height(
    y: &mut [f32],
    island: &EndIsland,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
) {
    let r = island.r;
    let r2 = (r + 1) * (r + 1);
    let x0 = floordiv(island.x - r, scale);
    let z0 = floordiv(island.z - r, scale);
    let x1 = floordiv(island.x + r, scale);
    let z1 = floordiv(island.z + r, scale);
    let ds = 0; // C 中 ds 恒为 0（保留以便对照）
    for j in z0..=z1 {
        if j < z || j >= z + h {
            continue;
        }
        let dz = j * scale - island.z + ds;
        for i in x0..=x1 {
            if i < x || i >= x + w {
                continue;
            }
            let dx = i * scale - island.x + ds;
            if dx * dx + dz * dz > r2 {
                continue;
            }
            let idx = ((j - z) * w + (i - x)) as usize;
            if y[idx] < island.y as f32 {
                y[idx] = island.y as f32;
            }
        }
    }
}

/// `mapEndIslandHeight`：把小末地岛（`small_end_islands` 群系 chunk 中的
/// 浮空岛）高度叠加到高度图 `y`（区域 `(x, z, w, h)`，`scale` 缩放）。
///
/// 只会抬高已有值：小岛圆盘内的格子取 `max(当前值, 岛 y)`。
#[allow(clippy::too_many_arguments)] // 与 C 参数列表一一对应
pub fn map_end_island_height(
    y: &mut [f32],
    en: &EndNoise,
    seed: u64,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
) {
    let rmax = (6 + scale - 1) / scale;
    let cx = floordiv(x - rmax, 16 / scale);
    let cz = floordiv(z - rmax, 16 / scale);
    let cw = floordiv(x + w + rmax, 16 / scale) - cx + 1;
    let ch = floordiv(z + h + rmax, 16 / scale) - cz + 1;

    let ids = en.map_end_biome(cx, cz, cw, ch);
    for cj in 0..ch {
        for ci in 0..cw {
            if ids[(cj * cw + ci) as usize] != crate::biome::BiomeId::SmallEndIslands {
                continue;
            }
            for island in get_end_islands(en.mc(), seed, cx + ci, cz + cj) {
                apply_end_island_height(y, &island, x, z, w, h, scale);
            }
        }
    }
}

/// `getLinkedGatewayChunk`：源折跃门 `src`（方块坐标）链接的目标 chunk
/// 与目标参考点。
///
/// 返回 `(chunk, dst)`：`chunk` 为 1:16 chunk 坐标（该方向 1024 方块处
/// 起向前/向后找到的非空末地 chunk），`dst` 为 C 的 `dst` 输出参数
/// （搜索终点处的方块坐标）。
pub fn get_linked_gateway_chunk(
    en: &EndNoise,
    sn: &SurfaceNoise,
    seed: u64,
    src: Pos,
) -> (Pos, Pos) {
    let invr = 1.0 / ((src.x * src.x + src.z * src.z) as f64).sqrt();
    let dx = src.x as f64 * invr;
    let dz = src.z as f64 * invr;
    let mut px = dx * 1024.0;
    let mut pz = dz * 1024.0;
    let dx = dx * 16.0;
    let dz = dz * 16.0;

    let mut c = Pos {
        x: (px.floor() as i32) >> 4,
        z: (pz.floor() as i32) >> 4,
    };

    if is_end_chunk_empty(en, sn, seed, c.x, c.z) {
        // 向前找第一个非空 chunk
        for _ in 0..15 {
            px += dx;
            pz += dz;
            let qx = (px.floor() as i32) >> 4;
            let qz = (pz.floor() as i32) >> 4;
            if qx == c.x && qz == c.z {
                continue;
            }
            c.x = qx;
            c.z = qz;
            if !is_end_chunk_empty(en, sn, seed, c.x, c.z) {
                break;
            }
        }
    } else {
        // 向后找最后一个非空 chunk
        for _ in 0..15 {
            px -= dx;
            pz -= dz;
            let qx = (px.floor() as i32) >> 4;
            let qz = (pz.floor() as i32) >> 4;
            if is_end_chunk_empty(en, sn, seed, qx, qz) {
                break;
            }
            c.x = qx;
            c.z = qz;
        }
    }
    let dst = Pos {
        x: px.floor() as i32,
        z: pz.floor() as i32,
    };
    (c, dst)
}

/// `getLinkedGatewayPos`：源折跃门 `src` 链接的目标方块位置。
///
/// 第一阶段确定参考点 `dst`：1.17+ 复刻了 MC 原版的 bug——区块内搜索结果
/// 变量被迭代器引用覆盖，参考点恒为目标 chunk 的 `(+15, +15)` 角（译自 C
/// 注释）；1.16- 则在目标 chunk 内选取地表/小岛最远（`dr` 最大）点。第二
/// 阶段对两个分支都执行：在 `(dst-16, dst-16)` 起 33×33 范围内取地表/小岛
/// 最高点作为最终落点。
pub fn get_linked_gateway_pos(en: &EndNoise, sn: &SurfaceNoise, seed: u64, src: Pos) -> Pos {
    let mut ymin = 0;
    let (c, mut dst) = get_linked_gateway_chunk(en, sn, seed, src);

    if en.mc() > McVersion::V1_16 {
        // MC 原版 bug（译注见上文）
        dst.x = c.x * 16 + 15;
        dst.z = c.z * 16 + 15;
    } else {
        let mut y = en.map_end_surface_height(sn, c.x * 16, c.z * 16, 16, 16, 1, 30);
        map_end_island_height(&mut y, en, seed, c.x * 16, c.z * 16, 16, 16, 1);

        let mut d = 0u64;
        for j in 0..16 {
            for i in 0..16 {
                let v = y[(j * 16 + i) as usize] as i32;
                if v < 30 {
                    continue;
                }
                // C: uint64_t 回绕乘法（坐标为负时符号扩展后平方）
                let dx = (16 * c.x + i) as i64 as u64;
                let dz = (16 * c.z + j) as i64 as u64;
                let dr = dx
                    .wrapping_mul(dx)
                    .wrapping_add(dz.wrapping_mul(dz))
                    .wrapping_add((v as i64 as u64).wrapping_mul(v as i64 as u64));
                if dr > d {
                    d = dr;
                    dst.x = dx as i32;
                    dst.z = dz as i32;
                }
            }
        }
        // 用已知最小 y 剪枝低处的地表生成
        for &v in &y {
            if v > ymin as f32 {
                ymin = v.floor() as i32;
            }
        }
    }

    let sp = Pos {
        x: dst.x - 16,
        z: dst.z - 16,
    };
    // 小岛检查比地表生成便宜，先用最高小岛抬升下界
    let mut y = vec![0.0f32; 33 * 33];
    map_end_island_height(&mut y, en, seed, sp.x, sp.z, 33, 33, 1);
    for &v in &y {
        if v > ymin as f32 {
            ymin = v.floor() as i32;
        }
    }

    let mut y = en.map_end_surface_height(sn, sp.x, sp.z, 33, 33, 1, ymin);
    map_end_island_height(&mut y, en, seed, sp.x, sp.z, 33, 33, 1);

    let mut v = -1.0f32;
    for i in 0..33 {
        for j in 0..33 {
            if y[(j * 33 + i) as usize] <= v {
                continue;
            }
            v = y[(j * 33 + i) as usize];
            dst.x = sp.x + i;
            dst.z = sp.z + j;
        }
    }

    dst
}
