//! Voronoi 缩放（1:1 比例）助手。
//!
//! 移植 cubiomes `layers.c` 的 `getVoronoiSHA` / `getVoronoiCell` /
//! `voronoiAccess3D` / `mapVoronoi114`（核心部分）与 `biomenoise.c` 的
//! `getVoronoiSrcRange`。
//! 1.15+ 的 1:1 群系边界由这些函数把 1:4 噪声单元扰动成 voronoi 细胞；
//! 1.14-（含末地 1.9–1.14 的 scale 1）使用旧版平面 voronoi
//!（[`map_voronoi_114_plane`]，种子流水线而非 SHA）。

use crate::rng::seed::{chunk_seed, first_int, step_seed};

use super::Range;

/// `getVoronoiSHA`：世界种子的 SHA-256 前 64 位（voronoi 散列盐）。
///
/// 按 C 的单块 SHA-256 压缩逐位移植（输入固定为 8 字节种子 + 填充）。
pub fn get_voronoi_sha(seed: u64) -> u64 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    const B: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let mut m = [0u32; 64];
    m[0] = (seed as u32).swap_bytes();
    m[1] = ((seed >> 32) as u32).swap_bytes();
    m[2] = 0x80000000;
    m[15] = 0x00000040;

    for i in 16..64 {
        m[i] = m[i - 7].wrapping_add(m[i - 16]);
        let mut x = m[i - 15];
        m[i] = m[i].wrapping_add(x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3));
        x = m[i - 2];
        m[i] = m[i].wrapping_add(x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10));
    }

    let [mut a0, mut a1, mut a2, mut a3, mut a4, mut a5, mut a6, mut a7] = B;

    for i in 0..64 {
        let mut x = a7.wrapping_add(K[i]).wrapping_add(m[i]);
        x = x.wrapping_add(a4.rotate_right(6) ^ a4.rotate_right(11) ^ a4.rotate_right(25));
        x = x.wrapping_add((a4 & a5) ^ (!a4 & a6));

        let mut y = a0.rotate_right(2) ^ a0.rotate_right(13) ^ a0.rotate_right(22);
        y = y.wrapping_add((a0 & a1) ^ (a0 & a2) ^ (a1 & a2));

        a7 = a6;
        a6 = a5;
        a5 = a4;
        a4 = a3.wrapping_add(x);
        a3 = a2;
        a2 = a1;
        a1 = a0;
        a0 = x.wrapping_add(y);
    }

    a0 = a0.wrapping_add(B[0]);
    a1 = a1.wrapping_add(B[1]);

    a0.swap_bytes() as u64 | ((a1.swap_bytes() as u64) << 32)
}

/// `getVoronoiCell`：voronoi 细胞内的扰动偏移（输出单位为 1/10240 格）。
fn get_voronoi_cell(sha: u64, a: i32, b: i32, c: i32) -> (i32, i32, i32) {
    let mut s = sha;
    s = step_seed(s, a as i64 as u64);
    s = step_seed(s, b as i64 as u64);
    s = step_seed(s, c as i64 as u64);
    s = step_seed(s, a as i64 as u64);
    s = step_seed(s, b as i64 as u64);
    s = step_seed(s, c as i64 as u64);

    let x = (((s >> 24) & 1023) as i32 - 512) * 36;
    s = step_seed(s, sha);
    let y = (((s >> 24) & 1023) as i32 - 512) * 36;
    s = step_seed(s, sha);
    let z = (((s >> 24) & 1023) as i32 - 512) * 36;
    (x, y, z)
}

/// `voronoiAccess3D`：把 1:1 坐标映射到所属 1:4 voronoi 细胞坐标。
pub fn voronoi_access_3d(sha: u64, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    let x = x - 2;
    let y = y - 2;
    let z = z - 2;
    let px = x >> 2;
    let py = y >> 2;
    let pz = z >> 2;
    let dx = (x & 3) * 10240;
    let dy = (y & 3) * 10240;
    let dz = (z & 3) * 10240;
    let (mut ax, mut ay, mut az) = (0, 0, 0);
    let mut dmin = u64::MAX;

    for i in 0..8 {
        let bx = (i & 4) != 0;
        let by = (i & 2) != 0;
        let bz = (i & 1) != 0;
        let cx = px + bx as i32;
        let cy = py + by as i32;
        let cz = pz + bz as i32;

        let (mut rx, mut ry, mut rz) = get_voronoi_cell(sha, cx, cy, cz);

        rx += dx - 40 * 1024 * bx as i32;
        ry += dy - 40 * 1024 * by as i32;
        rz += dz - 40 * 1024 * bz as i32;

        let d = (rx as i64 as u64)
            .wrapping_mul(rx as i64 as u64)
            .wrapping_add((ry as i64 as u64).wrapping_mul(ry as i64 as u64))
            .wrapping_add((rz as i64 as u64).wrapping_mul(rz as i64 as u64));
        if d < dmin {
            dmin = d;
            ax = cx;
            ay = cy;
            az = cz;
        }
    }
    (ax, ay, az)
}

/// `mapVoronoiPlane`：把 1:4 源平面按 voronoi 扰动放大到 1:1 输出平面。
///
/// 1.15+ 主世界分层群系源的 scale-1 层（`mapVoronoi`）使用；1.18+ 的
/// 等价路径是逐点 [`voronoi_access_3d`]。
///
/// 参数对应 C：`(x, z, w, h)` 为 1:1 输出区域（调用前会被 -2 偏移），
/// `(px, pz, pw, ph)` 为 1:4 源区域，`y` 为高度（旧版主世界恒为 0）。
/// `src` 为 `pw*ph` 的源数据，`out` 为 `w*h` 的输出。
#[allow(clippy::too_many_arguments)]
pub fn map_voronoi_plane(
    sha: u64,
    out: &mut [i32],
    src: &[i32],
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    y: i32,
    px: i32,
    pz: i32,
    pw: i32,
    ph: i32,
) {
    let x = x - 2;
    let y = y - 2;
    let z = z - 2;

    // 相邻两行的 4 个 voronoi 细胞角点（各含 xyz 扰动，单位 1/10240 格）
    let mut c00 = (0, 0, 0);
    let mut c01 = (0, 0, 0);
    let mut c10 = (0, 0, 0);
    let mut c11 = (0, 0, 0);

    for pj in 0..ph - 1 {
        let mut v00 = src[(pj * pw) as usize];
        let mut v10 = src[((pj + 1) * pw) as usize];
        let pjz = pz + pj;
        let j4 = pjz * 4 - z;
        // 每行重置（对应 C 循环体内的 `prev_skip = 1`）
        let mut prev_skip = true;

        for pi in 0..pw - 1 {
            let v01 = src[(pj * pw + pi + 1) as usize];
            let v11 = src[((pj + 1) * pw + pi + 1) as usize];
            let pix = px + pi;
            let i4 = pix * 4 - x;

            if v00 == v01 && v00 == v10 && v00 == v11 {
                for jj in 0..4 {
                    let j = j4 + jj;
                    if j < 0 || j >= h {
                        continue;
                    }
                    for ii in 0..4 {
                        let i = i4 + ii;
                        if i < 0 || i >= w {
                            continue;
                        }
                        out[(j * w + i) as usize] = v00;
                    }
                }
                prev_skip = true;
                v00 = v01;
                v10 = v11;
                continue;
            }
            if prev_skip {
                c00 = get_voronoi_cell(sha, pix, y - 1, pjz);
                c01 = get_voronoi_cell(sha, pix, y, pjz);
                c10 = get_voronoi_cell(sha, pix, y - 1, pjz + 1);
                c11 = get_voronoi_cell(sha, pix, y, pjz + 1);
                prev_skip = false;
            }
            let c00r = c00;
            let c01r = c01;
            let c10r = c10;
            let c11r = c11;
            c00 = get_voronoi_cell(sha, pix + 1, y - 1, pjz);
            c01 = get_voronoi_cell(sha, pix + 1, y, pjz);
            c10 = get_voronoi_cell(sha, pix + 1, y - 1, pjz + 1);
            c11 = get_voronoi_cell(sha, pix + 1, y, pjz + 1);

            const A: i64 = 40 * 1024;
            const B: i64 = 20 * 1024;

            for jj in 0..4 {
                let j = j4 + jj;
                if j < 0 || j >= h {
                    continue;
                }
                for ii in 0..4 {
                    let i = i4 + ii;
                    if i < 0 || i >= w {
                        continue;
                    }
                    let dx = (ii * 10 * 1024) as i64;
                    let dz = (jj * 10 * 1024) as i64;
                    let mut dmin = u64::MAX;
                    let mut v = v00;

                    // 8 个候选点：(角点) × (y-1, y)，最近者胜出；
                    // 与前两者打平时保持 v00（对应 C 中仅严格更小才更新）。
                    let cand = [
                        (c00r, 0, B, 0, v00),
                        (c01r, 0, -B, 0, v00),
                        (c00, A, B, 0, v01),
                        (c01, A, -B, 0, v01),
                        (c10r, 0, B, A, v10),
                        (c11r, 0, -B, A, v10),
                        (c10, A, B, A, v11),
                        (c11, A, -B, A, v11),
                    ];
                    for &(c, ox, oy, oz, cv) in &cand {
                        let rx = c.0 as i64 - ox + dx;
                        let ry = c.1 as i64 + oy;
                        let rz = c.2 as i64 - oz + dz;
                        let d = ((rx * rx) as u64)
                            .wrapping_add((ry * ry) as u64)
                            .wrapping_add((rz * rz) as u64);
                        if d < dmin {
                            dmin = d;
                            v = cv;
                        }
                    }
                    out[(j * w + i) as usize] = v;
                }
            }

            v00 = v01;
            v10 = v11;
        }
    }
}

/// `mapVoronoi114` 的核心（1.14- 旧版平面 voronoi 缩放）：给定 1:4 源平面
/// `src`，输出 1:1 平面到 `out`。
///
/// 对应 C 中 `l->p == NULL`（源数据已就位）的调用形式：源区域
/// `(px, pz, pw, ph)` 由输出区域 `(x, z, w, h)` 推出（与函数体内的
/// `x -= 2; z -= 2` 之后一致），`src` 须有 `pw*ph` 个元素。
/// `st`/`ss` 为层的 `startSalt`/`startSeed`（末地路径：零初始化层 +
/// `startSalt = getLayerSalt(10)`，即 `st = layer_salt(10), ss = 0`）。
///
/// C 把结果写进 `out` 之后的暂存区再 `memmove` 回来；循环覆盖全部输出格，
/// 故这里直接写 `out`。
#[allow(clippy::too_many_arguments)]
pub fn map_voronoi_114_plane(
    st: u64,
    ss: u64,
    src: &[i32],
    out: &mut [i32],
    x: i32,
    z: i32,
    w: i32,
    h: i32,
) {
    let x = x - 2;
    let z = z - 2;
    let px = x >> 2;
    let pz = z >> 2;
    let pw = ((x + w) >> 2) - px + 2;
    let ph = ((z + h) >> 2) - pz + 2;
    let pwu = pw as usize;
    debug_assert!(src.len() >= pwu * ph as usize);
    debug_assert!(out.len() >= (w * h) as usize);

    for pj in 0..ph - 1 {
        let mut v00 = src[pj as usize * pwu];
        let mut v01 = src[(pj as usize + 1) * pwu];
        let pjz = pz + pj;
        let j4 = pjz * 4 - z;

        for pi in 0..pw - 1 {
            let pix = px + pi;
            let i4 = pix * 4 - x;
            let v10 = src[(pi + 1 + pj * pw) as usize];
            let v11 = src[(pi + 1 + (pj + 1) * pw) as usize];

            if v00 == v01 && v00 == v10 && v00 == v11 {
                for jj in 0..4 {
                    let j = j4 + jj;
                    if j < 0 || j >= h {
                        continue;
                    }
                    for ii in 0..4 {
                        let i = i4 + ii;
                        if i < 0 || i >= w {
                            continue;
                        }
                        out[(j * w + i) as usize] = v00;
                    }
                }
            } else {
                let mut cs = chunk_seed(ss, (pi + px) * 4, (pj + pz) * 4);
                let da1 = ((first_int(cs, 1024) - 512) * 36) as i64;
                cs = step_seed(cs, st);
                let da2 = ((first_int(cs, 1024) - 512) * 36) as i64;

                cs = chunk_seed(ss, (pi + px + 1) * 4, (pj + pz) * 4);
                let db1 = ((first_int(cs, 1024) - 512) * 36) as i64 + 40 * 1024;
                cs = step_seed(cs, st);
                let db2 = ((first_int(cs, 1024) - 512) * 36) as i64;

                cs = chunk_seed(ss, (pi + px) * 4, (pj + pz + 1) * 4);
                let dc1 = ((first_int(cs, 1024) - 512) * 36) as i64;
                cs = step_seed(cs, st);
                let dc2 = ((first_int(cs, 1024) - 512) * 36) as i64 + 40 * 1024;

                cs = chunk_seed(ss, (pi + px + 1) * 4, (pj + pz + 1) * 4);
                let dd1 = ((first_int(cs, 1024) - 512) * 36) as i64 + 40 * 1024;
                cs = step_seed(cs, st);
                let dd2 = ((first_int(cs, 1024) - 512) * 36) as i64 + 40 * 1024;

                for jj in 0..4 {
                    let j = j4 + jj;
                    if j < 0 || j >= h {
                        continue;
                    }
                    let mj = (10240 * jj) as i64;
                    let sja = (mj - da2) * (mj - da2);
                    let sjb = (mj - db2) * (mj - db2);
                    let sjc = (mj - dc2) * (mj - dc2);
                    let sjd = (mj - dd2) * (mj - dd2);

                    for ii in 0..4 {
                        let i = i4 + ii;
                        if i < 0 || i >= w {
                            continue;
                        }
                        let mi = (10240 * ii) as i64;
                        let da = (mi - da1) * (mi - da1) + sja;
                        let db = (mi - db1) * (mi - db1) + sjb;
                        let dc = (mi - dc1) * (mi - dc1) + sjc;
                        let dd = (mi - dd1) * (mi - dd1) + sjd;

                        let v = if da < db && da < dc && da < dd {
                            v00
                        } else if db < da && db < dc && db < dd {
                            v10
                        } else if dc < da && dc < db && dc < dd {
                            v01
                        } else {
                            v11
                        };
                        out[(j * w + i) as usize] = v;
                    }
                }
            }
            v00 = v10;
            v01 = v11;
        }
    }
}

/// `getVoronoiSrcRange`：1:1 区域所需的 1:4 源区域（含边界余量）。
///
/// 要求 `r.scale == 1`，返回区域的 `scale` 恒为 4。
pub fn get_voronoi_src_range(r: Range) -> Range {
    assert!(r.scale == 1, "getVoronoiSrcRange() expects input range with scale 1:1");

    let x = r.x - 2;
    let z = r.z - 2;
    let mut s = Range {
        scale: 4,
        x: x >> 2,
        z: z >> 2,
        sx: ((x + r.sx) >> 2) - (x >> 2) + 2,
        sz: ((z + r.sz) >> 2) - (z >> 2) + 2,
        y: 0,
        sy: 0,
    };
    if r.sy >= 1 {
        let ty = r.y - 2;
        s.y = ty >> 2;
        s.sy = ((ty + r.sy) >> 2) - s.y + 2;
    }
    s
}
