//! Bedrock 群系层栈（`bedrock.wasm` 导出 `p` = func21 的群系过滤部分）。
//!
//! 网站 wasm 在执行带过滤的结构定位（导出 `p`）时，会在栈上静态构造一棵 **54 层**
//! 的群系层链（每层 56 字节：函数表索引 / scale 乘数 b5 / 边距 b6 / scale / salt /
//! 起始种子 s1 / 起始盐 s2 / 父层指针 p1、p2），然后：
//!
//! 1. `f_1`（func19）：自顶向下按 `scale *= b5` 设定各层 scale；
//! 2. `f_p`（func15）：自顶向下按 `s1 = step³(seed, salt)`、`s2 = step0(s1)`
//!    设定各层 RNG 状态（salt == 0 的层 s1 = s2 = 0）；
//! 3. 对每个结构候选位置调用 `f_f`（func5）：在 scale-4 坐标下取
//!    `(x±r, z±r)` 的矩形区域，求值**第 52 层**（`f_ha` 洋温混合层），
//!    要求区域内**所有**格子的群系 id 都在给定列表中。
//!
//! 与 Java/cubiomes 层链的关键差异（逐指令核对，见 `docs/BEDROCK_ANALYSIS.md`）：
//! - 单元 RNG 为 `s=s2+x; step z; step x; step z` 四步模式，
//!   `step(s,t)=(s·6364136223846793005+1442695040888963407)·s+t`（全 wrapping i64）；
//! - zoom 层用独立的 i32 LCG：`step2(s,t)=(s·1284865837-144211633)·s+t`，
//!   且只取 s1/s2 的低 32 位；
//! - 坐标扩展方式逐函数不同（`f_cb`/`f_na` 用零扩展，其余多数用符号扩展）；
//! - 层 53（`f_ga`，4 倍 voronoi zoom）只参与建栈与 scale/seed 传播，
//!   过滤路径只求值到层 52，因此 `f_ga` 的 apply 未移植（调用即 panic）。
//!
//! 全部函数逐指令移植自 `reference/site/bedrock.dcmp`（func32..=58 层函数 +
//! func5/15/18/19 辅助），由 `tests/bedrock_layers.rs` 的逐层输出向量对拍。

/// LCG 乘数（wasm 内层栈通用）。
const LCG_C: i64 = 6364136223846793005;
/// LCG 增量。
const LCG_A: i64 = 1442695040888963407;

/// `step(s, t) = (s·C + A)·s + t`（全 wrapping i64）。
#[inline]
fn step(s: i64, t: i64) -> i64 {
    s.wrapping_mul(LCG_C).wrapping_add(LCG_A).wrapping_mul(s).wrapping_add(t)
}

/// 标准单元 RNG：`s2+x` → step z → step x → step z。
#[inline]
fn cell_seed4(s2: i64, x: i64, z: i64) -> i64 {
    let s = s2.wrapping_add(x);
    let s = step(s, z);
    let s = step(s, x);
    step(s, z)
}

/// zoom 层 i32 RNG：`step2(s, t) = (s·1284865837 - 144211633)·s + t`（wrapping i32）。
#[inline]
fn step2(s: i32, t: i32) -> i32 {
    s.wrapping_mul(1284865837)
        .wrapping_sub(144211633)
        .wrapping_mul(s)
        .wrapping_add(t)
}

/// 掩码 M1 = {0, 40, 42, 44, 46}（海洋/洋温集合）。
const M1: i64 = 93458488360961;
/// 掩码 M2 = {0, 24, 40..=47}。
const M2: i64 = 280375481860097;
/// 掩码 M3 = {24, 41, 43, 45, 47}。
const M3: i64 = 186916993499136;
/// 掩码 M4：f_qa/f_va 中 table2520 二次映射的门控位集。
const M4: i64 = 548620208191;

/// wasm `1L << i64_extend_i32_u(v) & mask`：移位计数对 64 取模（i64.shl 语义）。
#[inline]
fn mask_contains(mask: i64, v: i32) -> bool {
    (1i64 << ((v as u32) & 63)) & mask != 0
}

#[inline]
fn m1(v: i32) -> bool {
    mask_contains(M1, v)
}
#[inline]
fn m2(v: i32) -> bool {
    mask_contains(M2, v)
}
#[inline]
fn m3(v: i32) -> bool {
    mask_contains(M3, v)
}

/// 带 `v <= 63` 前置守卫的 M2 测试（dcmp 中的常见写法）。
#[inline]
fn m2_guarded(v: i32) -> bool {
    v <= 63 && m2(v)
}

/// 群系分类表（dcmp 中 `(id<<2)[455]:int`，即数据段 1820 起的 168 项；id 越界为 -1）。
#[rustfmt::skip]
const CATEGORY: [i32; 168] = [
    0, 1, 2, 3, 4, 5, 6, 7, -1, -1, -1, 7,
    12, 12, 14, 14, 16, 2, 4, 5, 3, 21, 21, 21,
    0, 25, 16, 4, 4, 4, 5, 5, 5, 5, 3, 35,
    35, 37, 37, 37, 0, 0, 0, 0, 0, 0, 0, 0,
    21, 21, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, 1, 2, 3,
    4, 5, 6, -1, -1, -1, -1, -1, 12, -1, -1, -1,
    -1, -1, -1, -1, -1, 21, -1, 21, -1, -1, -1, 4,
    4, 4, 5, -1, 5, 5, 3, 35, 35, 37, 37, 37,
];

/// wasm 的 `id <= 167 ? 表[id] : -1`（本层链中 id 恒非负，负数按 -1 处理）。
#[inline]
fn category(id: i32) -> i32 {
    if (0..=167).contains(&id) {
        CATEGORY[id as usize]
    } else {
        -1
    }
}

/// f_ua（群系选择层）的四个变体表（数据段 1552/1584/1616/1632）。
const T_UA_PLAINS: [i32; 6] = [2, 2, 2, 35, 35, 1];
const T_UA_DESERT: [i32; 8] = [4, 29, 3, 1, 1, 1, 27, 6];
const T_UA_HILLS: [i32; 4] = [4, 3, 5, 1];
const T_UA_FOREST: [i32; 4] = [12, 12, 12, 30];

/// f_va 的洋温深化表（数据段 2492，按 `center - 40` 索引）。
const T_VA: [i32; 7] = [41, 24, 43, 24, 45, 24, 47];

/// f_qa 的群系变异表（数据段 2520，按 `id - 1` 索引，0 项由 M4 门控挡掉）。
#[rustfmt::skip]
const T_QA: [i32; 40] = [
    129, 130, 131, 132, 133, 134, 0, 0, 0, 0, 0, 140, 0, 0, 0, 0, 0, 0, 0, 0,
    149, 0, 151, 0, 0, 0, 155, 156, 157, 158, 0, 160, 161, 162, 163, 164, 165,
    166, 167, 0,
];

/// 单个层节点（对应 wasm 56 字节层结构体；字段语义见模块文档）。
#[derive(Clone, Copy, Debug)]
pub struct Layer {
    /// 层函数分派索引（wasm 函数表索引 1..=25）。
    pub func: i32,
    /// scale 乘数（f_1 用；1/2/4）。
    pub b5: i32,
    /// 边距（f_s 尺寸传播用）。
    pub b6: i32,
    /// 本层 scale（f_1 填充）。
    pub scale: i32,
    /// 层盐（f_p 用；为 0 时 s1 = s2 = 0）。
    pub salt: i64,
    /// 起始种子（f_p 填充）。
    pub s1: i64,
    /// 起始盐（f_p 填充）。
    pub s2: i64,
    /// 父层 1 索引（-1 = 无）。
    pub p1: i32,
    /// 父层 2 索引（-1 = 无）。
    pub p2: i32,
}

/// 层栈静态构造参数（func21 内的字面量，经运行时内存快照核对）。
#[derive(Clone, Copy)]
struct LayerDef {
    func: i32,
    b5: i32,
    b6: i32,
    salt: i64,
    p1: i32,
    p2: i32,
}

#[rustfmt::skip]
const LAYER_DEFS: [LayerDef; 54] = [
    LayerDef { func:  1, b5: 1, b6:  0, salt: 3107951898966440229, p1: -1, p2: -1 }, //  0 f_gb 岛根
    LayerDef { func:  2, b5: 2, b6:  3, salt: -8774101820360152064, p1:  0, p2: -1 }, //  1 f_fb 模糊 zoom
    LayerDef { func:  3, b5: 1, b6:  2, salt: 3107951898966440229, p1:  1, p2: -1 }, //  2 f_db 岛生长
    LayerDef { func:  4, b5: 2, b6:  3, salt: 229918546094678885, p1:  2, p2: -1 }, //  3 f_eb 平滑 zoom
    LayerDef { func:  3, b5: 1, b6:  2, salt: -5014677998924433960, p1:  3, p2: -1 }, //  4
    LayerDef { func:  3, b5: 1, b6:  2, salt: -1473395045552829736, p1:  4, p2: -1 }, //  5
    LayerDef { func:  3, b5: 1, b6:  2, salt: 7231908362866731896, p1:  5, p2: -1 }, //  6
    LayerDef { func:  5, b5: 1, b6:  2, salt: -5014677998924433960, p1:  6, p2: -1 }, //  7 f_cb 添岛
    LayerDef { func:  6, b5: 1, b6:  2, salt: -5014677998924433960, p1:  7, p2: -1 }, //  8 f_bb 去岛/补岛
    LayerDef { func:  3, b5: 1, b6:  2, salt: 7590731853067264053, p1:  8, p2: -1 }, //  9
    LayerDef { func:  7, b5: 1, b6:  2, salt: -5014677998924433960, p1:  9, p2: -1 }, // 10 f_za 暖海→
    LayerDef { func:  8, b5: 1, b6:  2, salt: -5014677998924433960, p1: 10, p2: -1 }, // 11 f_ya 暖海→
    LayerDef { func:  9, b5: 1, b6:  2, salt: 7590731853067264053, p1: 11, p2: -1 }, // 12 f_xa 变异位
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 12, p2: -1 }, // 13
    LayerDef { func:  4, b5: 2, b6:  3, salt: 3006835321906069877, p1: 13, p2: -1 }, // 14
    LayerDef { func:  3, b5: 1, b6:  2, salt: 5360640171528462240, p1: 14, p2: -1 }, // 15
    LayerDef { func: 10, b5: 1, b6:  0, salt: 3038466749335869312, p1: 15, p2: -1 }, // 16 f_ua 群系选择
    LayerDef { func: 11, b5: 1, b6:  2, salt: -7479281634960481323, p1: 16, p2: -1 }, // 17 f_wa 深海
    LayerDef { func: 12, b5: 1, b6:  2, salt: 5360640171528462240, p1: 17, p2: -1 }, // 18 f_va 洋温边缘
    LayerDef { func: 13, b5: 1, b6:  0, salt: 5852781679691581125, p1: 18, p2: -1 }, // 19 f_sa 竹林
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 19, p2: -1 }, // 20
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 20, p2: -1 }, // 21
    LayerDef { func: 14, b5: 1, b6:  2, salt: 5692911206796425088, p1: 21, p2: -1 }, // 22 f_ra 岸
    LayerDef { func: 15, b5: 1, b6:  0, salt: 5723240131506253216, p1: 18, p2: -1 }, // 23 f_ta 河链根
    LayerDef { func:  4, b5: 2, b6:  3, salt: 0, p1: 23, p2: -1 }, // 24（salt=0）
    LayerDef { func:  4, b5: 2, b6:  3, salt: 0, p1: 24, p2: -1 }, // 25（salt=0）
    LayerDef { func: 16, b5: 1, b6:  2, salt: 5692911206796425088, p1: 22, p2: 25 }, // 26 f_qa 山丘/变异（双父）
    LayerDef { func: 17, b5: 1, b6:  0, salt: 5852781679691581125, p1: 26, p2: -1 }, // 27 f_ma 变异平原
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5692911206796425088, p1: 27, p2: -1 }, // 28
    LayerDef { func:  3, b5: 1, b6:  2, salt: 7590731853067264053, p1: 28, p2: -1 }, // 29
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 29, p2: -1 }, // 30
    LayerDef { func: 18, b5: 1, b6:  2, salt: 5692911206796425088, p1: 30, p2: -1 }, // 31 f_la 岸/边缘
    LayerDef { func:  4, b5: 2, b6:  3, salt: 1827289100522298840, p1: 31, p2: -1 }, // 32
    LayerDef { func:  4, b5: 2, b6:  3, salt: -4039966243449460139, p1: 32, p2: -1 }, // 33
    LayerDef { func: 19, b5: 1, b6:  2, salt: 5692911206796425088, p1: 33, p2: -1 }, // 34 f_na 平滑
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 23, p2: -1 }, // 35 河 zoom 链
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 35, p2: -1 }, // 36
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 36, p2: -1 }, // 37
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 37, p2: -1 }, // 38
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 38, p2: -1 }, // 39
    LayerDef { func:  4, b5: 2, b6:  3, salt: 5852781679691581125, p1: 39, p2: -1 }, // 40
    LayerDef { func: 20, b5: 1, b6:  2, salt: 3107951898966440229, p1: 40, p2: -1 }, // 41 f_oa 河成形
    LayerDef { func: 19, b5: 1, b6:  2, salt: 5692911206796425088, p1: 41, p2: -1 }, // 42 f_na 平滑
    LayerDef { func: 21, b5: 1, b6:  0, salt: 5723240131506253216, p1: 34, p2: 42 }, // 43 f_ka 河混（双父）
    LayerDef { func: 22, b5: 1, b6:  0, salt: -5014677998924433960, p1: -1, p2: -1 }, // 44 f_ia 洋温根
    LayerDef { func: 23, b5: 1, b6:  2, salt: -5014677998924433960, p1: 44, p2: -1 }, // 45 f_ja 洋温边缘
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 45, p2: -1 }, // 46 洋温 zoom 链
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 46, p2: -1 }, // 47
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 47, p2: -1 }, // 48
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 48, p2: -1 }, // 49
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 49, p2: -1 }, // 50
    LayerDef { func:  4, b5: 2, b6:  3, salt: 837738509879401688, p1: 50, p2: -1 }, // 51
    LayerDef { func: 24, b5: 1, b6: 17, salt: 5723240131506253216, p1: 43, p2: 51 }, // 52 f_ha 洋混（双父）
    LayerDef { func: 25, b5: 4, b6:  7, salt: -8738471090773341224, p1: 52, p2: -1 }, // 53 f_ga 4x zoom（从不求值）
];

/// Bedrock 群系层栈（54 层；构造一次可对多个候选位置做群系检查）。
pub struct LayerStack {
    /// 54 层节点。
    pub layers: Vec<Layer>,
}

impl LayerStack {
    /// 建栈并对全栈执行 f_1（scale 传播）与 f_p（种子传播）。
    ///
    /// `seed` 为世界种子（wasm 中由两个 u32 零扩展拼成的 64 位值，与 i64 位模式一致）。
    pub fn new(seed: i64) -> Self {
        let mut stack = LayerStack {
            layers: LAYER_DEFS
                .iter()
                .map(|d| Layer {
                    func: d.func,
                    b5: d.b5,
                    b6: d.b6,
                    scale: 0,
                    salt: d.salt,
                    s1: 0,
                    s2: 0,
                    p1: d.p1,
                    p2: d.p2,
                })
                .collect(),
        };
        stack.set_scales(53, 1);
        stack.set_seed(53, seed);
        stack
    }

    /// f_1（func19）：`scale = b`，递归 p1 用 `b*b5`，p2 链迭代同值。
    fn set_scales(&mut self, idx: usize, scale: i32) {
        let mut idx = idx;
        let mut scale = scale;
        loop {
            let (b5, p1, p2) = {
                let l = &self.layers[idx];
                (l.b5, l.p1, l.p2)
            };
            self.layers[idx].scale = scale;
            if p1 >= 0 {
                self.set_scales(p1 as usize, scale * b5);
            }
            if p2 >= 0 {
                scale *= b5;
                idx = p2 as usize;
            } else {
                break;
            }
        }
    }

    /// f_p（func15）：先递归 p2 再 p1；salt≠0 时 `s1 = step³(seed, salt)`、
    /// `s2 = (s1·C + A)·s1`，salt==0 时两者为 0。
    fn set_seed(&mut self, idx: usize, seed: i64) {
        let (salt, p1, p2) = {
            let l = &self.layers[idx];
            (l.salt, l.p1, l.p2)
        };
        if p2 >= 0 {
            self.set_seed(p2 as usize, seed);
        }
        if p1 >= 0 {
            self.set_seed(p1 as usize, seed);
        }
        let (s1, s2) = if salt == 0 {
            (0, 0)
        } else {
            let mut s = seed;
            s = step(s, salt);
            s = step(s, salt);
            s = step(s, salt);
            (s, s.wrapping_mul(LCG_C).wrapping_add(LCG_A).wrapping_mul(s))
        };
        self.layers[idx].s1 = s1;
        self.layers[idx].s2 = s2;
    }

    /// f_s（func18）：沿祖先链传播 `w/2^shift + b6`，返回沿途最大缓冲区尺寸。
    fn propagate_size(&self, idx: usize, w: i32, h: i32, mw: &mut i32, mh: &mut i32) {
        let mut idx = idx;
        let (mut w, mut h) = (w, h);
        loop {
            let l = &self.layers[idx];
            match l.b5 {
                2 => {
                    w >>= 1;
                    h >>= 1;
                }
                4 => {
                    w >>= 2;
                    h >>= 2;
                }
                _ => {}
            }
            w += l.b6;
            h += l.b6;
            if w > *mw {
                *mw = w;
            }
            if h > *mh {
                *mh = h;
            }
            if l.p1 >= 0 {
                self.propagate_size(l.p1 as usize, w, h, mw, mh);
            }
            if l.p2 >= 0 {
                idx = l.p2 as usize;
            } else {
                break;
            }
        }
    }

    /// 求值层 `idx` 在 `(x, z)` 起 `w×h` 区域的输出（供测试逐层对拍）。
    ///
    /// `buf` 至少要有 [`Self::required_size`] 个元素；只前 `w*h` 项是有效输出。
    pub fn apply(&self, idx: usize, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        let l = &self.layers[idx];
        match l.func {
            1 => Self::apply_gb(l, buf, x, z, w, h),
            2 => self.apply_zoom(l, buf, x, z, w, h, ZoomMode::Fuzzy),
            3 => self.apply_db(l, buf, x, z, w, h),
            4 => self.apply_zoom(l, buf, x, z, w, h, ZoomMode::Smooth),
            5 => self.apply_cb(l, buf, x, z, w, h),
            6 => self.apply_bb(l, buf, x, z, w, h),
            7 => self.apply_neighbor_pair(l, buf, x, z, w, h, 1, [3, 4], 2),
            8 => self.apply_neighbor_pair(l, buf, x, z, w, h, 4, [1, 2], 3),
            9 => self.apply_xa(l, buf, x, z, w, h),
            10 => self.apply_ua(l, buf, x, z, w, h),
            11 => self.apply_wa(l, buf, x, z, w, h),
            12 => self.apply_va(l, buf, x, z, w, h),
            13 => self.apply_sa(l, buf, x, z, w, h),
            14 => self.apply_ra(l, buf, x, z, w, h),
            15 => self.apply_ta(l, buf, x, z, w, h),
            16 => self.apply_qa(l, buf, x, z, w, h),
            17 => self.apply_ma(l, buf, x, z, w, h),
            18 => self.apply_la(l, buf, x, z, w, h),
            19 => self.apply_na(l, buf, x, z, w, h),
            20 => self.apply_oa(l, buf, x, z, w, h),
            21 => self.apply_ka(l, buf, x, z, w, h),
            22 => Self::apply_ia(l, buf, x, z, w, h),
            23 => self.apply_ja(l, buf, x, z, w, h),
            24 => self.apply_ha(l, buf, x, z, w, h),
            25 => unreachable!("f_ga（层 53）在过滤路径中从不被求值"),
            _ => unreachable!("未知层函数 {}", l.func),
        }
    }

    /// 求值层 `idx` 所需的最大缓冲区尺寸（f_s 的最大传播）。
    pub fn required_size(&self, idx: usize, w: i32, h: i32) -> (i32, i32) {
        let (mut mw, mut mh) = (w, h);
        self.propagate_size(idx, w, h, &mut mw, &mut mh);
        (mw, mh)
    }

    /// f_f（func5）：检查 scale-4 坐标 `(x±r, z±r)` 区域内所有格子的群系
    /// 是否都在 `list` 中（对层 52 求值）。
    pub fn check(&self, x: i32, z: i32, r: i32, list: &[i32]) -> bool {
        let mut marks = [false; 256];
        for &v in list {
            if (0..=255).contains(&v) {
                marks[v as usize] = true;
            }
        }
        let x0 = (x - r) >> 2;
        let w = ((x + r) >> 2) - x0 + 1;
        let z0 = (z - r) >> 2;
        let h = ((z + r) >> 2) - z0 + 1;
        let (mw, mh) = self.required_size(52, w, h);
        // wasm 只零初始化前 w*h 格；父层写入总会先于读取覆盖其余区域
        let mut buf = vec![0i32; mw as usize * mh as usize];
        self.apply(52, &mut buf, x0, z0, w, h);
        for &v in &buf[..(w * h) as usize] {
            if !(0..=255).contains(&v) || !marks[v as usize] {
                return false;
            }
        }
        true
    }

    // ---- 根层（无父层） ----

    /// f_gb（fn1，func58）：岛根。`(s>>24)%10==0 → 1`；原点在区域内时强制为 1。
    fn apply_gb(l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        if h > 0 && w > 0 {
            for row in 0..h {
                let zr = row as i64 + z as i64;
                for col in 0..w {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    buf[(row * w + col) as usize] = i32::from((s >> 24) % 10 == 0);
                }
            }
        }
        if x <= 0 && x > -w && z <= 0 && z > -h {
            buf[(-(x.wrapping_add(z.wrapping_mul(w)))) as usize] = 1;
        }
    }

    /// f_ia（fn22，func34）：洋温根。修正取模后按阈值分 40/42/0/44/46。
    fn apply_ia(l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                let mut v = (s >> 24) % 100;
                if v < 0 {
                    v += 100;
                }
                buf[(row * w + col) as usize] = if v < 8 {
                    40
                } else if v < 40 {
                    42
                } else if v < 58 {
                    0
                } else if v < 95 {
                    44
                } else {
                    46
                };
            }
        }
    }

    // ---- 同尺寸变换层 ----

    /// f_ta（fn15，func45）：河链根。`center > 0 → 修正((s>>24)%299999) + 2`。
    fn apply_ta(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let cell = (row * w + col) as usize;
                let v = buf[cell];
                buf[cell] = if v > 0 {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    let mut m = (s >> 24) % 299999;
                    if m < 0 {
                        m += 299999;
                    }
                    (m + 2) as i32
                } else {
                    0
                };
            }
        }
    }

    /// f_sa（fn13，func44）：`center == 21 && (s>>24)%10==0 → 48`。
    fn apply_sa(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let cell = (row * w + col) as usize;
                if buf[cell] == 21 {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    if (s >> 24) % 10 == 0 {
                        buf[cell] = 48;
                    }
                }
            }
        }
    }

    /// f_ma（fn17，func38）：`center == 1 && (s>>24)%57==0 → 129`。
    fn apply_ma(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let cell = (row * w + col) as usize;
                if buf[cell] == 1 {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    if (s >> 24) % 57 == 0 {
                        buf[cell] = 129;
                    }
                }
            }
        }
    }

    /// f_xa（fn9，func49）：变异位。`center!=0 && (s>>24)%13==0`
    /// 时按 `%15` 结果叠加 0xF00 掩码位。
    fn apply_xa(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let cell = (row * w + col) as usize;
                let d = buf[cell];
                if d != 0 {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    if (s >> 24) % 13 == 0 {
                        let g = (step(s, l.s1) >> 24) % 15;
                        let bits = ((g >> 63) as i32)
                            .wrapping_add((g as i32) << 8)
                            .wrapping_add(256)
                            & 3840;
                        buf[cell] = d | bits;
                    }
                }
            }
        }
    }

    /// f_ua（fn10，func46）：群系选择层。剥离变异位后 14/M2 直通，
    /// 1/2/3/4 查变体表（平原/沙漠/山丘/森林），其余落 14。
    fn apply_ua(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for row in 0..h {
            let zr = row as i64 + z as i64;
            for col in 0..w {
                let cell = (row * w + col) as usize;
                let a = buf[cell];
                let f = a & !3840;
                if f == 14 || (f < 64 && m2(f)) {
                    buf[cell] = f;
                    continue;
                }
                let mutbits = a & 3840;
                let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                buf[cell] = match f - 1 {
                    0 => {
                        // 平原
                        let g = s >> 24;
                        if mutbits != 0 {
                            if g % 3 == 0 {
                                39
                            } else {
                                38
                            }
                        } else {
                            T_UA_PLAINS[(g % 6).rem_euclid(6) as usize]
                        }
                    }
                    1 => {
                        // 沙漠
                        if mutbits != 0 {
                            21
                        } else {
                            T_UA_DESERT[((s >> 24) % 8).rem_euclid(8) as usize]
                        }
                    }
                    2 => {
                        // 峭壁
                        if mutbits != 0 {
                            32
                        } else {
                            T_UA_HILLS[((s >> 24) % 4).rem_euclid(4) as usize]
                        }
                    }
                    3 => T_UA_FOREST[((s >> 24) % 4).rem_euclid(4) as usize], // 森林
                    _ => 14,
                };
            }
        }
    }

    // ---- ±1 边距邻域层 ----

    /// f_cb（fn5，func54）：添岛。中心与四边邻居全 0 时按 bit24==0 → 1。
    /// 注意本层坐标用**零扩展**。
    fn apply_cb(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            let zr = row as i64 + (z as u32) as i64;
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let mut out = center;
                if center == 0
                    && buf[row * stride + col + 1] == 0
                    && buf[(row + 1) * stride + col + 2] == 0
                    && buf[(row + 1) * stride + col] == 0
                    && buf[(row + 2) * stride + col + 1] == 0
                {
                    let xc = col as i64 + (x as u32) as i64;
                    let mut g = l.s2.wrapping_add(xc);
                    g = step(g, zr);
                    g = step(g, xc);
                    let bit = g
                        .wrapping_mul(9797421)
                        .wrapping_add(23560527)
                        .wrapping_mul(g)
                        .wrapping_add(zr)
                        & 16777216;
                    if bit == 0 {
                        out = 1;
                    }
                }
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_bb（fn6，func53）：`center ∈ M1` 保持；否则按修正 `%6` → 4/3/1。
    /// 注意 dcmp 的 select_if 语义是 `cond ? a : b`：d>1 → 1，d==1 → 3。
    fn apply_bb(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            let zr = row as i64 + z as i64;
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let out = if center <= 63 && m1(center) {
                    center
                } else {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    let v = (s >> 24) % 6;
                    let d = v + if v < 0 { 6 } else { 0 };
                    if d == 0 {
                        4
                    } else if d > 1 {
                        1
                    } else {
                        3
                    }
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_za（fn7，func51）/ f_ya（fn8，func50）：中心为 `from` 且任一四边邻居
    /// 属于 `neighbors` 时改为 `to`（无 RNG）。
    #[allow(clippy::too_many_arguments)]
    fn apply_neighbor_pair(
        &self,
        l: &Layer,
        buf: &mut [i32],
        x: i32,
        z: i32,
        w: i32,
        h: i32,
        from: i32,
        neighbors: [i32; 2],
        to: i32,
    ) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let mut out = center;
                if center == from {
                    let n = buf[row * stride + col + 1];
                    let e = buf[(row + 1) * stride + col + 2];
                    let wv = buf[(row + 1) * stride + col];
                    let s = buf[(row + 2) * stride + col + 1];
                    if neighbors.contains(&n)
                        || neighbors.contains(&e)
                        || neighbors.contains(&wv)
                        || neighbors.contains(&s)
                    {
                        out = to;
                    }
                }
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_wa（fn11，func48）：深海。中心与**四角**邻居全 0 时 `(s>>24)%100==0 → 14`。
    fn apply_wa(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            let zr = row as i64 + z as i64;
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let mut out = center;
                if center == 0
                    && buf[row * stride + col] == 0
                    && buf[row * stride + col + 2] == 0
                    && buf[(row + 2) * stride + col] == 0
                    && buf[(row + 2) * stride + col + 2] == 0
                {
                    let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                    out = if (s >> 24) % 100 == 0 { 14 } else { 0 };
                }
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_va（fn12，func47）：洋温边缘。中心与四邻居全在 M1 时按表深化
    /// （注意 `center - 40 > 6` 是**无符号**比较，center==0 → 24）。
    fn apply_va(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let out_cell = row * w as usize + col;
                if center > 63 || !m1(center) {
                    buf[out_cell] = center;
                    continue;
                }
                let n = buf[row * stride + col + 1];
                let e = buf[(row + 1) * stride + col + 2];
                let wv = buf[(row + 1) * stride + col];
                let s = buf[(row + 2) * stride + col + 1];
                let n_in = n < 64 && m1(n);
                // 注意：E 的 M1 测试没有 ≤63 守卫（wasm 移位取模语义）
                #[allow(clippy::if_same_then_else)]
                let term = if n > 63 {
                    i32::from(n_in)
                } else if !m1(e) {
                    i32::from(n_in)
                } else if n_in {
                    2
                } else {
                    1
                };
                let count =
                    term + i32::from(wv < 64 && m1(wv)) + i32::from(s < 64 && m1(s));
                if count < 4 {
                    buf[out_cell] = center;
                    continue;
                }
                let idx = center.wrapping_sub(40);
                buf[out_cell] = if (idx as u32) > 6 {
                    24
                } else {
                    T_VA[idx as usize]
                };
            }
        }
    }

    /// f_ra（fn14，func43）：岸层。
    fn apply_ra(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let wv = buf[(row + 1) * stride + col];
                let ev = buf[(row + 1) * stride + col + 2];
                let sv = buf[(row + 2) * stride + col + 1];
                let nv = buf[row * stride + col + 1];
                let k = buf[(row + 1) * stride + col + 1];
                let ns = [nv, ev, wv, sv];
                let out = match k {
                    2 => {
                        if ns.contains(&12) {
                            34
                        } else {
                            2
                        }
                    }
                    6 => {
                        if ns.iter().any(|&v| v == 2 || v == 30 || v == 12) {
                            1
                        } else if ns.iter().any(|&v| v == 21 || v == 48) {
                            23
                        } else {
                            6
                        }
                    }
                    32 => {
                        if ns.iter().all(|&v| v == 32 || category(v) == 21) {
                            32
                        } else {
                            5
                        }
                    }
                    38 => {
                        if ns.iter().all(|&v| v == 38 || category(v) == -1) {
                            38
                        } else {
                            37
                        }
                    }
                    39 => {
                        if ns.iter().all(|&v| v == 39 || category(v) == -1) {
                            39
                        } else {
                            37
                        }
                    }
                    _ => k,
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_la（fn18，func37）：岸/边缘层（大分派）。
    ///
    /// 注意 wasm 中 `(a-37) >= 2`、`a > 24`、`(v-37) > 2`、`(v-165) >= 3`
    /// 均为**无符号**比较（已经 wat 确认，dcmp 文本省略了 `u` 后缀）。
    fn apply_la(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let out_cell = row * w as usize + col;
                let c = buf[(row + 1) * stride + col]; // W
                let g = buf[(row + 1) * stride + col + 2]; // E
                let h4 = buf[(row + 2) * stride + col + 1]; // S
                let i = buf[row * stride + col + 1]; // N
                let a = buf[(row + 1) * stride + col + 1]; // 中心

                if a == 14 {
                    // 冻洋→15（邻有 0）否则保持 14
                    buf[out_cell] = if i == 0 || g == 0 || c == 0 || h4 == 0 {
                        15
                    } else {
                        14
                    };
                    continue;
                }
                if category(a) != 21 {
                    if a == 3 || a == 34 {
                        // B_i → 25 逻辑
                        if m2(a) {
                            continue; // wasm 此处跳过写入（保持父层遗留值）
                        }
                        let mut j = 25;
                        if !(m2_guarded(i) || m2_guarded(g) || m2_guarded(c)) {
                            // WAT select 语义：S∉M2（或 >63）→ 保持 a；S∈M2 → 25
                            j = if h4 > 63 || !m2(h4) { a } else { 25 };
                        }
                        buf[out_cell] = j;
                    } else if matches!(a, 11 | 12 | 13 | 26 | 30 | 31 | 46 | 140 | 158) {
                        // B_j → 26 逻辑
                        if a <= 63 && m2(a) {
                            continue; // 同上跳过写入
                        }
                        let mut j = 26;
                        if !(m2_guarded(i) || m2_guarded(g) || m2_guarded(c)) {
                            j = if h4 > 63 || !m2(h4) { a } else { 26 };
                        }
                        buf[out_cell] = j;
                    } else if (a.wrapping_sub(37) as u32) >= 2 {
                        // B_h 主分支（a ∉ {37, 38}）
                        if (a as u32) > 24 || (1i32 << (a & 31)) & 16777409 == 0 {
                            // B_v：邻有 M2 → 16（沙滩）
                            if m2_guarded(i) || m2_guarded(g) || m2_guarded(c) {
                                buf[out_cell] = 16;
                            } else if h4 > 63 || !m2(h4) {
                                buf[out_cell] = a;
                            } else {
                                buf[out_cell] = 16;
                            }
                        } else {
                            buf[out_cell] = a; // a ∈ {0, 7, 8, 24}
                        }
                    } else {
                        // a ∈ {37, 38}（红土边缘）
                        if m2_guarded(i) || m2_guarded(g) || m2_guarded(c) || m2_guarded(h4)
                        {
                            buf[out_cell] = a;
                        } else {
                            // 邻居非 {37,38,39,165,166,167} → 2
                            let bad = |v: i32| {
                                (v.wrapping_sub(37) as u32) > 2
                                    && (v.wrapping_sub(165) as u32) >= 3
                            };
                            buf[out_cell] =
                                if bad(i) || bad(g) || bad(c) || bad(h4) { 2 } else { a };
                        }
                    }
                    continue;
                }
                // category(a) == 21（丛林类边缘）：邻居"坏"→ 23；全好且有 M2 → 16
                let bad = |v: i32| {
                    if (v & -2) == 4 || category(v) == 21 {
                        return false;
                    }
                    v > 63 || !m2(v)
                };
                if bad(i) || bad(g) || bad(c) || bad(h4) {
                    buf[out_cell] = 23;
                } else if m2_guarded(i) || m2_guarded(g) || m2_guarded(c) {
                    buf[out_cell] = 16;
                } else if h4 > 63 || !m2(h4) {
                    buf[out_cell] = a;
                } else {
                    buf[out_cell] = 16;
                }
            }
        }
    }

    /// f_na（fn19，func39）：平滑层。注意本层坐标用**零扩展**。
    fn apply_na(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            let zr = row as i64 + (z as u32) as i64;
            for col in 0..w as usize {
                let n = buf[row * stride + col + 1];
                let center = buf[(row + 1) * stride + col + 1];
                let wv = buf[(row + 1) * stride + col];
                let s = buf[(row + 2) * stride + col + 1];
                let e = buf[(row + 1) * stride + col + 2];
                let out = if center == wv && n == center {
                    center
                } else {
                    let r = s == n;
                    if !(r && e == wv) {
                        if r {
                            n
                        } else if wv == e {
                            wv
                        } else {
                            center
                        }
                    } else {
                        // S==N 且 E==W：按 RNG bit24 选 W/N
                        let xc = col as i64 + (x as u32) as i64;
                        let mut g = l.s2.wrapping_add(xc);
                        g = step(g, zr);
                        g = step(g, xc);
                        let bit = g
                            .wrapping_mul(9797421)
                            .wrapping_add(23560527)
                            .wrapping_mul(g)
                            .wrapping_add(zr)
                            & 16777216;
                        if bit == 0 {
                            wv
                        } else {
                            n
                        }
                    }
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_oa（fn20，func40）：河成形。`t(v) = v>1 ? (v&1)|2 : v`；
    /// 四邻居变换值全等 → -1，否则 7。
    fn apply_oa(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        let t = |v: i32| if v > 1 { (v & 1) | 2 } else { v };
        for row in 0..h as usize {
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let n = buf[row * stride + col + 1];
                let e = buf[(row + 1) * stride + col + 2];
                let wv = buf[(row + 1) * stride + col];
                let s = buf[(row + 2) * stride + col + 1];
                let tc = t(center);
                let out = if tc == t(e) && tc == t(s) && tc == t(n) && tc == t(wv) {
                    -1
                } else {
                    7
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_ja（fn23，func35）：洋温边缘。中心 40 邻有 40 → 0；中心 46 邻有 46 → 0。
    fn apply_ja(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            for col in 0..w as usize {
                let center = buf[(row + 1) * stride + col + 1];
                let wv = buf[(row + 1) * stride + col];
                let e = buf[(row + 1) * stride + col + 2];
                let n = buf[row * stride + col + 1];
                let s = buf[(row + 2) * stride + col + 1];
                // dcmp 的 br_table 把 center==40 映射到检查「邻居 ==46」的
                // 分支、center==46 映射到「邻居 ==40」（标签物理顺序与逻辑相反），
                // 即：暖洋与冻洋相邻时互相侵蚀为普通海洋。
                let out = match center {
                    40 if wv == 46 || e == 46 || n == 46 || s == 46 => 0,
                    46 if wv == 40 || e == 40 || n == 40 || s == 40 => 0,
                    _ => center,
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    /// f_db（fn3，func55）：岛生长。只读四角邻居与中心；各路径的
    /// `step(g, s1)` 推进次数不同（0/1/2 次），须严格对齐。
    fn apply_db(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x - 1, z - 1, w + 2, h + 2);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = (w + 2) as usize;
        for row in 0..h as usize {
            let zr = row as i64 + z as i64;
            for col in 0..w as usize {
                let nw = buf[row * stride + col];
                let ne = buf[row * stride + col + 2];
                let sw = buf[(row + 2) * stride + col];
                let se = buf[(row + 2) * stride + col + 2];
                let a = buf[(row + 1) * stride + col + 1];
                let out = match a {
                    0 => {
                        if nw == 0 && ne == 0 && sw == 0 && se == 0 {
                            0
                        } else {
                            let mut g = cell_seed4(l.s2, col as i64 + x as i64, zr);
                            // NW/NE 选择（a2 = 已推进次数计数）
                            let (mut d, mut cnt);
                            if nw == 0 {
                                d = 1;
                                if ne != 0 {
                                    d = ne;
                                    cnt = 1;
                                    g = step(g, l.s1);
                                } else {
                                    cnt = 0;
                                }
                            } else {
                                g = step(g, l.s1);
                                if ne == 0 {
                                    d = nw;
                                    cnt = 1;
                                } else {
                                    d = if (g & 16777216) == 0 { ne } else { nw };
                                    cnt = 2;
                                    g = step(g, l.s1);
                                }
                            }
                            // SW
                            let mut c = sw;
                            if sw == 0 {
                                c = d;
                            } else {
                                match cnt {
                                    0 => {}
                                    1 => {
                                        if (g & 16777216) != 0 {
                                            c = d;
                                        }
                                    }
                                    _ => {
                                        if (g >> 24) % 3 != 0 {
                                            c = d;
                                        }
                                    }
                                }
                                cnt += 1;
                                g = step(g, l.s1);
                            }
                            // SE
                            let mut d2 = se;
                            if se == 0 {
                                d2 = c;
                            } else {
                                match cnt {
                                    // wasm br_table a==0 → B_t：恒保持 SE（不做 RNG 选择），
                                    // 此前误并入 bit25 分支，NW/NE/SW 全 0 且 SE 非 0 时出错
                                    0 => {}
                                    1 => {
                                        if (g & 16777216) != 0 {
                                            d2 = c;
                                        }
                                    }
                                    2 => {
                                        if (g >> 24) % 3 != 0 {
                                            d2 = c;
                                        }
                                    }
                                    _ => {
                                        if (g & 50331648) != 0 {
                                            d2 = c;
                                        }
                                    }
                                }
                                g = step(g, l.s1);
                            }
                            if d2 == 4 {
                                4
                            } else if (g >> 24) % 3 == 0 {
                                d2
                            } else {
                                0
                            }
                        }
                    }
                    4 => 4,
                    _ => {
                        // center ∈ {1, 2, 3, ≥5}：四角全非零保持，否则 1/5 → 0
                        if nw != 0 && ne != 0 && sw != 0 && se != 0 {
                            a
                        } else {
                            let s = cell_seed4(l.s2, col as i64 + x as i64, zr);
                            if (s >> 24) % 5 == 0 {
                                0
                            } else {
                                a
                            }
                        }
                    }
                };
                buf[row * w as usize + col] = out;
            }
        }
    }

    // ---- zoom 层 ----

    /// f_fb（fn2，func57，模糊）/ f_eb（fn4，func56，平滑）：2 倍 zoom。
    ///
    /// 父层区域 `(x>>1, z>>1, ((x+w)>>1)-(x>>1)+1, ((z+h)>>1)-(z>>1)+1)`；
    /// 临时网格行距 2·pw（wasm 按 (2pw+1)(2ph+1) 分配）；裁剪回写时
    /// 行号加 `(z&1)`、列偏移 `(x&1)`。
    #[allow(clippy::too_many_arguments)]
    fn apply_zoom(
        &self,
        l: &Layer,
        buf: &mut [i32],
        x: i32,
        z: i32,
        w: i32,
        h: i32,
        mode: ZoomMode,
    ) {
        let px = x >> 1;
        let pz = z >> 1;
        let pw = ((x + w) >> 1) - px + 1;
        let ph = ((z + h) >> 1) - pz + 1;
        self.apply(l.p1 as usize, buf, px, pz, pw, ph);
        let stride = (2 * pw) as usize;
        let mut tmp = vec![0i32; (2 * pw + 1) as usize * (2 * ph + 1) as usize];
        if pw >= 1 && ph >= 1 {
            // wasm 守卫是 pw-1/ph-1 >= 0，w/h ≥ 1 时恒真
            let s1lo = l.s1 as i32;
            let s2lo = l.s2 as i32;
            for k in 0..ph {
                let mut g = buf[(k * pw) as usize]; // W
                let mut south = buf[((k + 1) * pw) as usize]; // S
                // 每个父行写临时网格的第 2k、2k+1 行（wat：偏移 l*pw*16 字节 = 2k 行）
                let row2 = (2 * k) as usize * stride;
                for p in 0..pw {
                    let east = buf[(k * pw + p + 1) as usize]; // E
                    let se = buf[((k + 1) * pw + p + 1) as usize]; // SE
                    let c0 = row2 + 2 * p as usize;
                    if g == south && g == east && g == se {
                        tmp[c0] = g;
                        tmp[c0 + 1] = g;
                        tmp[c0 + stride] = g;
                        tmp[c0 + stride + 1] = g;
                    } else {
                        // 4 步 i32 RNG：2px+s2lo → 2pz → 2px → 2pz
                        let gx = (p + px) << 1;
                        let gz = (k + pz) << 1;
                        let mut a = gx.wrapping_add(s2lo);
                        a = step2(a, gz);
                        a = step2(a, gx);
                        a = step2(a, gz);
                        tmp[c0] = g; // NW
                        tmp[c0 + stride] = if a & 16777216 != 0 { south } else { g }; // SW
                        let a = step2(a, s1lo);
                        tmp[c0 + 1] = if a & 16777216 != 0 { east } else { g }; // NE
                        let v = a
                            .wrapping_mul(9797421)
                            .wrapping_add(57114959)
                            .wrapping_mul(a)
                            .wrapping_add(s1lo);
                        let pick = (v >> 24) & 3;
                        tmp[c0 + stride + 1] = match mode {
                            // f_fb：1/2/3 → E/S/SE，0 → W
                            ZoomMode::Fuzzy => match pick {
                                1 => east,
                                2 => south,
                                3 => se,
                                _ => g,
                            },
                            // f_eb：多数表决，平票时按同一 RNG 选
                            ZoomMode::Smooth => {
                                let aa = i32::from(south == se);
                                let s_cnt = i32::from(g == east)
                                    + i32::from(g == south)
                                    + i32::from(g == se);
                                let r2 = i32::from(se == east) + i32::from(south == east);
                                if s_cnt > r2 && s_cnt > aa {
                                    g
                                } else if r2 > s_cnt {
                                    east
                                } else if s_cnt < aa {
                                    south
                                } else {
                                    match pick {
                                        1 => east,
                                        2 => south,
                                        3 => se,
                                        _ => g,
                                    }
                                }
                            }
                        };
                    }
                    g = east;
                    south = se;
                }
            }
        }
        if h <= 0 {
            return;
        }
        // 裁剪：out[row][col] = tmp[row + (z&1)][col + (x&1)]
        let xoff = (x & 1) as usize;
        for row in 0..h as usize {
            let src = (row + (z & 1) as usize) * stride + xoff;
            let dst = row * w as usize;
            buf[dst..dst + w as usize].copy_from_slice(&tmp[src..src + w as usize]);
        }
    }

    // ---- 双父层 ----

    /// f_ka（fn21，func36）：河混（同尺寸双父）。P2==7 时按 P1 改写。
    fn apply_ka(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p1 as usize, buf, x, z, w, h);
        let p1g = buf[..(w * h) as usize].to_vec();
        self.apply(l.p2 as usize, buf, x, z, w, h);
        if h <= 0 || w <= 0 {
            return;
        }
        for cell in 0..(w * h) as usize {
            let a = p1g[cell];
            let out = if buf[cell] != 7 {
                a
            } else if a == 0 {
                0
            } else if a > 63 || !m2(a) {
                if a == 12 {
                    11
                } else if (a & -2) == 14 {
                    15
                } else {
                    7
                }
            } else {
                a
            };
            buf[cell] = out;
        }
    }

    /// f_qa（fn16，func42）：山丘/变异层（±1 双父）。
    ///
    /// P2 为河链（zoom 后的 f_ta 值），`(d-2)%29==1` 且 P1 不在 M1 时
    /// 查 T_QA 变异表；否则走 48 项分派 + 四邻分类多数表决的山丘逻辑。
    fn apply_qa(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        let pw = w + 2;
        let ph = h + 2;
        self.apply(l.p1 as usize, buf, x - 1, z - 1, pw, ph);
        let p1g = buf[..(pw * ph) as usize].to_vec();
        self.apply(l.p2 as usize, buf, x - 1, z - 1, pw, ph);
        if h <= 0 || w <= 0 {
            return;
        }
        let stride = pw as usize;
        for row in 0..h as usize {
            let zr = row as i64 + z as i64;
            for col in 0..w as usize {
                let ci = (row + 1) * stride + col + 1;
                let d = buf[ci]; // P2 中心
                let c = (d - 2) % 29;
                let mut a = p1g[ci]; // P1 中心
                if d >= 2 && c == 1 && (a > 63 || !m1(a)) {
                    // B_h：变异表映射（M4 门控；M4 置位保证 c2 ∈ 0..=38）
                    let c2 = a - 1;
                    if c2 <= 38 && (M4 >> ((c2 as u32) & 63)) & 1 != 0 {
                        a = T_QA[c2 as usize];
                    }
                } else {
                    let mut out = a; // B_g：保持 P1
                    'main: {
                        let mut g = cell_seed4(l.s2, col as i64 + x as i64, zr);
                        if c != 0 && (g >> 24) % 3 != 0 {
                            break 'main;
                        }
                        let mut e = 17;
                        let mut reach_k = true;
                        match a {
                            0 => e = 24,
                            1 => {
                                // B_s：注意这里不推进 g
                                e = if (step(g, l.s1) >> 24) % 3 == 0 { 18 } else { 4 };
                            }
                            2 => {}
                            3 => e = 34,
                            4 => e = 18,
                            5 => e = 19,
                            12 => e = 13,
                            21 => e = 22,
                            27 => e = 28,
                            29 => e = 1,
                            30 => e = 31,
                            32 => e = 33,
                            35 => e = 36,
                            38 | 39 => e = 37,
                            47 => e = 49,
                            _ => {
                                // B_l：M3 门控 + RNG
                                if a > 63 || !m3(a) {
                                    reach_k = false;
                                } else {
                                    g = step(g, l.s1);
                                    if (g >> 24) % 3 != 0 {
                                        reach_k = false;
                                    } else {
                                        e = if g
                                            .wrapping_mul(9797421)
                                            .wrapping_add(23560527)
                                            .wrapping_mul(g)
                                            .wrapping_add(l.s1)
                                            & 16777216
                                            == 0
                                        {
                                            1
                                        } else {
                                            4
                                        };
                                    }
                                }
                            }
                        }
                        if !reach_k {
                            break 'main;
                        }
                        // B_k：c==0 且 a!=e 时按 M4 门控做二次映射
                        if c == 0 && a != e {
                            if e > 39 {
                                break 'main;
                            }
                            let c2 = e - 1;
                            if (M4 >> ((c2 as u32) & 63)) & 1 == 0 {
                                break 'main;
                            }
                            e = T_QA[c2 as usize];
                        }
                        // B_aa：a != e 时四邻分类多数表决
                        if a == e {
                            break 'main;
                        }
                        let n = p1g[row * stride + col + 1];
                        let s = p1g[(row + 2) * stride + col + 1];
                        let wv = p1g[(row + 1) * stride + col];
                        let ev = p1g[(row + 1) * stride + col + 2];
                        let cat_a = category(a);
                        let score = |v: i32| {
                            if a != v {
                                i32::from(category(v) == cat_a)
                            } else {
                                1
                            }
                        };
                        if score(wv) + score(n) + score(ev) + score(s) > 2 {
                            out = e;
                        }
                    }
                    a = out;
                }
                buf[row * w as usize + col] = a;
            }
        }
    }

    /// f_ha（fn24，func33）：洋温混合层（双父）。
    ///
    /// 先求值 P2（层 51 洋温链，同尺寸），扫描其中 40/46 格子扩大
    /// 包围盒（各方向 8/9 格），再以扩大的区域求值 P1（层 43），
    /// 最后逐格合并：P1 非 M2 直通；P1==24 时按 P2 洋温换 43/45/47。
    fn apply_ha(&self, l: &Layer, buf: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
        self.apply(l.p2 as usize, buf, x, z, w, h);
        let p2g = buf[..(w * h) as usize].to_vec();
        let (mut k, mut j, mut i2, mut g2) = (0, 0, h, w);
        if h > 0 && w > 0 {
            for row in 0..h {
                let o = row + 9;
                let interior = o < h && row > 8;
                for col in 0..w {
                    let m = col + 9;
                    // 内部行只检查边缘 9 列
                    if interior && m < w && col >= 9 {
                        continue;
                    }
                    let v = p2g[(row * w + col) as usize];
                    if v == 40 || v == 46 {
                        if k > col - 8 {
                            k = col - 8;
                        }
                        if i2 < o {
                            i2 = o;
                        }
                        if j > row - 8 {
                            j = row - 8;
                        }
                        if g2 < m {
                            g2 = m;
                        }
                    }
                }
            }
        }
        let pw = g2 - k;
        let ph = i2 - j;
        self.apply(l.p1 as usize, buf, x + k, z + j, pw, ph);
        if h <= 0 || w <= 0 {
            return;
        }
        let p1g = buf[..(pw * ph) as usize].to_vec();
        let stride = pw as usize;
        for row in 0..h {
            for col in 0..w {
                let d = p1g[((row - j) as usize) * stride + (col - k) as usize];
                let cell = (row * w + col) as usize;
                if d > 63 || !m2(d) {
                    buf[cell] = d;
                    continue;
                }
                let mut g = p2g[cell];
                if d == 24 {
                    match g - 42 {
                        0 => g = 43,
                        2 => g = 45,
                        4 => g = 47,
                        _ => {
                            if g == 0 {
                                g = 24;
                            }
                        }
                    }
                }
                buf[cell] = g;
            }
        }
    }
}

/// zoom 层模式（f_fb 模糊 / f_eb 平滑）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ZoomMode {
    Fuzzy,
    Smooth,
}
