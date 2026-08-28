//! Bundle D golden 测试（由 `reference/gen/bundledvec.c` 的 cubiomes C
//! 参考实现输出生成，`reference/gen/gen_bundled_tests.py` 转换，请勿手工编辑）。
//!
//! 覆盖四连底座高速搜索（quadbase.c / quadbase.h）：
//! - `get_quad_hut_cst`：低 20 位星座分类（全部表条目 + 非条目边界值）；
//! - `is_quad_base`（dispatcher）窗口扫描：小屋 1.13+/1.12- radius=128
//!   （→ `is_quad_base_feature24`）、小屋 radius=124（→ `is_quad_base_feature`
//!   一般路径）、海底神殿 radius=128/200（→ `is_quad_base_large`），命中种子
//!   与返回半径的 f32 位模式逐一比较；
//! - `is_quad_base_feature24_classic`：经典星座窗口扫描；
//! - `scan_for_quads`：region 矩形扫描（小屋 lowBitN=20、海底神殿 lowBitN=48）；
//! - `get_optimal_afk`：真实四连小屋底座的最佳 AFK 站位；
//! - `search_all48`：全 48 位搜索（ideal 低 20 位 + 小屋判定，74474 个结果
//!   比对数量 + 校验和 + 首尾样本）。

use minecraft_seed_core::rng::mul_inv;
use minecraft_seed_core::structure::{
    LOW20_QUAD_CLASSIC, LOW20_QUAD_HUT_NORMAL, Pos, QuadHutCst, StructureType, get_config,
    get_optimal_afk, get_quad_hut_cst, get_structure_pos, is_quad_base,
    is_quad_base_feature24_classic, scan_for_quads, search_all48,
};
use minecraft_seed_core::McVersion;

/// 扫描命中记录：`(seed, 返回半径的 f32 位模式)`。
type SeedBits = (u64, u32);
/// 窗口扫描分组：`(低 20 位表条目, 命中列表)`。
type ScanGroup = (u64, &'static [SeedBits]);

fn hut_conf() -> minecraft_seed_core::structure::StructureConfig {
    get_config(StructureType::SwampHut, McVersion::V1_13).unwrap()
}

fn hut_conf_112() -> minecraft_seed_core::structure::StructureConfig {
    get_config(StructureType::SwampHut, McVersion::V1_12).unwrap()
}

fn mon_conf() -> minecraft_seed_core::structure::StructureConfig {
    get_config(StructureType::Monument, McVersion::V1_13).unwrap()
}

/// 窗口扫描：低 20 位固定为表条目（减 salt），枚举高 k ∈ [0, K) 位，
/// 收集 `(seed, 返回值 f32 位模式)`。
fn scan_window(
    table: &[u64],
    salt: u64,
    k_max: u64,
    check: impl Fn(u64) -> f32,
) -> Vec<(u64, Vec<SeedBits>)> {
    let mut groups = Vec::new();
    for &entry in table {
        let low = (entry.wrapping_sub(salt)) & 0xfffff;
        let mut hits = Vec::new();
        for k in 0..k_max {
            let s = (k << 20) | low;
            let r = check(s);
            if r != 0.0 {
                hits.push((s, r.to_bits()));
            }
        }
        if !hits.is_empty() {
            groups.push((entry, hits));
        }
    }
    groups
}


/// `(低 20 位取值, 期望分类)`：全部表条目（按 C 查表顺序的首个命中）+
/// 非条目边界值。
const CST_CASES: &[(u64, QuadHutCst)] = &[
    (0x43f18, QuadHutCst::Ideal),
    (0xc751a, QuadHutCst::Ideal),
    (0xf520a, QuadHutCst::Ideal),
    (0x43f18, QuadHutCst::Ideal),
    (0x79a0a, QuadHutCst::Classic),
    (0xc751a, QuadHutCst::Ideal),
    (0xf520a, QuadHutCst::Ideal),
    (0x43f18, QuadHutCst::Ideal),
    (0x65118, QuadHutCst::Normal),
    (0x75618, QuadHutCst::Normal),
    (0x79a0a, QuadHutCst::Classic),
    (0x89718, QuadHutCst::Normal),
    (0x9371a, QuadHutCst::Normal),
    (0xa5a08, QuadHutCst::Normal),
    (0xb5e18, QuadHutCst::Normal),
    (0xc751a, QuadHutCst::Ideal),
    (0xf520a, QuadHutCst::Ideal),
    (0x1272d, QuadHutCst::Barely),
    (0x17908, QuadHutCst::Barely),
    (0x367b9, QuadHutCst::Barely),
    (0x43f18, QuadHutCst::Ideal),
    (0x487c9, QuadHutCst::Barely),
    (0x487ce, QuadHutCst::Barely),
    (0x50aa7, QuadHutCst::Barely),
    (0x647b5, QuadHutCst::Barely),
    (0x65118, QuadHutCst::Normal),
    (0x75618, QuadHutCst::Normal),
    (0x79a0a, QuadHutCst::Classic),
    (0x89718, QuadHutCst::Normal),
    (0x9371a, QuadHutCst::Normal),
    (0x967ec, QuadHutCst::Barely),
    (0xa3d0a, QuadHutCst::Barely),
    (0xa5918, QuadHutCst::Barely),
    (0xa591d, QuadHutCst::Barely),
    (0xa5a08, QuadHutCst::Normal),
    (0xb5e18, QuadHutCst::Normal),
    (0xc6749, QuadHutCst::Barely),
    (0xc6d9a, QuadHutCst::Barely),
    (0xc751a, QuadHutCst::Ideal),
    (0xd7108, QuadHutCst::Barely),
    (0xd717a, QuadHutCst::Barely),
    (0xe2739, QuadHutCst::Barely),
    (0xe9918, QuadHutCst::Barely),
    (0xee1c4, QuadHutCst::Barely),
    (0xf520a, QuadHutCst::Ideal),
    (0x00000, QuadHutCst::None),
    (0x00001, QuadHutCst::None),
    (0x43f17, QuadHutCst::None),
    (0x43f19, QuadHutCst::None),
    (0xfffff, QuadHutCst::None),
    (0x12345, QuadHutCst::None),
];

#[test]
fn quad_hut_cst_matches_c() {
    for &(low20, want) in CST_CASES {
        assert_eq!(get_quad_hut_cst(low20), want, "cst of {low20:#05x}");
    }
    // 表常量自身与 C 静态表一致（条目数 + 内容已由上方分类断言覆盖）
    assert_eq!(LOW20_QUAD_CLASSIC.len(), 4);
    assert_eq!(LOW20_QUAD_HUT_NORMAL.len(), 10);
    // mul_inv：C `mulInv(132897987541, 1 << n)`，scan_for_quads 的一般路径
    assert_eq!(mul_inv(132897987541, 1 << 20), 132477);
    assert_eq!(mul_inv(132897987541, 1 << 48), 211541297333629);
    // 其他位宽与模逆定义自洽（x * inv ≡ 1 (mod m)）
    assert_eq!(mul_inv(132897987541, 1 << 16).wrapping_mul(132897987541) & 0xffff, 1);
}

/// 小屋 1.13+ radius=128（dispatcher → feature24）窗口扫描命中。
const QHUT_HITS_K: u64 = 65536;
/// 条目：`(低 20 位表条目, [(seed, 半径 f32 位模式)])`
const QHUT_HITS: &[ScanGroup] = &[
    (0x43f18, &[
        (26102803108, 0x42f05768),
        (27177593508, 0x42f05768),
        (46804839076, 0x42f05768),
        (62546061988, 0x42f05768),
        (63586249380, 0x42f05768),
        (64190229156, 0x42f05768),
    ]),
    (0x65118, &[
        (11740593316, 0x42f3a5b5),
        (17876860068, 0x42f3a5b5),
        (60325313700, 0x42f3a5b5),
        (62103698596, 0x42f3a5b5),
        (62338579620, 0x42f3a5b5),
        (64488160420, 0x42f3a5b5),
    ]),
    (0x75618, &[
        (11610636708, 0x42f3a5b5),
        (12182110628, 0x42f3a5b5),
        (26136560036, 0x42f3a5b5),
        (61973741988, 0x42f3a5b5),
        (64221888932, 0x42f3a5b5),
        (64223986084, 0x42f3a5b5),
    ]),
    (0x79a0a, &[
        (11906352534, 0x42faed21),
        (12375066006, 0x42faed21),
        (14526743958, 0x42faed21),
        (27039401366, 0x42faed21),
        (29052667286, 0x42faed21),
        (64419038614, 0x42faed21),
        (64421135766, 0x42faed21),
    ]),
    (0x89718, &[
        (11608621732, 0x42faed21),
        (11610718884, 0x42faed21),
        (13621887652, 0x42faed21),
        (27716846244, 0x42faed21),
        (29828678308, 0x42faed21),
        (36492378788, 0x42faed21),
        (38505644708, 0x42faed21),
        (46977090212, 0x42faed21),
        (48990356132, 0x42faed21),
        (61503013540, 0x42faed21),
        (63654691492, 0x42faed21),
    ]),
    (0x9371a, &[
        (27143316134, 0x42faed21),
        (27614126758, 0x42faed21),
        (29763707558, 0x42faed21),
        (29862273702, 0x42faed21),
        (31776973478, 0x42faed21),
        (37381612198, 0x42faed21),
        (47010685606, 0x42faed21),
    ]),
    (0xa5a08, &[
        (10800285076, 0x42faed21),
        (14492321172, 0x42faed21),
        (29018244500, 0x42faed21),
        (30093034900, 0x42faed21),
        (38566577556, 0x42faed21),
        (46265222548, 0x42faed21),
        (47340012948, 0x42faed21),
        (48885613972, 0x42faed21),
        (62613570964, 0x42faed21),
        (62842160532, 0x42faed21),
    ]),
    (0xb5e18, &[
        (2820688292, 0x42f3a5b5),
        (11508140452, 0x42f3a5b5),
        (27108854180, 0x42f3a5b5),
        (28185741732, 0x42f3a5b5),
        (28422719908, 0x42f3a5b5),
        (38285625764, 0x42f3a5b5),
        (44355832228, 0x42f3a5b5),
        (45432719780, 0x42f3a5b5),
        (46507510180, 0x42f3a5b5),
        (61637413284, 0x42f3a5b5),
        (62009657764, 0x42f3a5b5),
    ]),
    (0xc751a, &[
        (8855314598, 0x42f05768),
        (10030768294, 0x42f05768),
        (12651159718, 0x42f05768),
        (28251873446, 0x42f05768),
    ]),
    (0xf520a, &[
        (11840798102, 0x42f05768),
        (30058757526, 0x42f05768),
        (30665883030, 0x42f05768),
        (38751452566, 0x42f05768),
        (47305735574, 0x42f05768),
        (61502406038, 0x42f05768),
        (63882673558, 0x42f05768),
    ]),
];

#[test]
fn quad_hut_scan_matches_c() {
    let conf = hut_conf();
    let salt = conf.salt as u32 as u64;
    let got = scan_window(LOW20_QUAD_HUT_NORMAL, salt, QHUT_HITS_K, |s| {
        is_quad_base(&conf, s, 128)
    });
    let want: Vec<(u64, Vec<SeedBits>)> = QHUT_HITS
        .iter()
        .map(|&(e, h)| (e, h.to_vec()))
        .collect();
    assert_eq!(got, want);
}

/// 小屋 1.12-（旧 salt 14357617）radius=128 窗口扫描命中。
const QHUT112_HITS_K: u64 = 16384;
/// 条目：`(低 20 位表条目, [(seed, 半径 f32 位模式)])`
const QHUT112_HITS: &[ScanGroup] = &[
    (0x65118, &[
        (11740593319, 0x42f3a5b5),
    ]),
    (0x75618, &[
        (11610636711, 0x42f3a5b5),
        (12182110631, 0x42f3a5b5),
    ]),
    (0x79a0a, &[
        (11906352537, 0x42faed21),
        (12375066009, 0x42faed21),
        (14526743961, 0x42faed21),
    ]),
    (0x89718, &[
        (11608621735, 0x42faed21),
        (11610718887, 0x42faed21),
        (13621887655, 0x42faed21),
    ]),
    (0xa5a08, &[
        (10800285079, 0x42faed21),
        (14492321175, 0x42faed21),
    ]),
    (0xb5e18, &[
        (2820688295, 0x42f3a5b5),
        (11508140455, 0x42f3a5b5),
    ]),
    (0xc751a, &[
        (8855314601, 0x42f05768),
        (10030768297, 0x42f05768),
        (12651159721, 0x42f05768),
    ]),
    (0xf520a, &[
        (11840798105, 0x42f05768),
    ]),
];

#[test]
fn quad_hut_scan_112_matches_c() {
    let conf = hut_conf_112();
    let salt = conf.salt as u32 as u64;
    let got = scan_window(LOW20_QUAD_HUT_NORMAL, salt, QHUT112_HITS_K, |s| {
        is_quad_base(&conf, s, 128)
    });
    let want: Vec<(u64, Vec<SeedBits>)> = QHUT112_HITS
        .iter()
        .map(|&(e, h)| (e, h.to_vec()))
        .collect();
    assert_eq!(got, want);
}

/// 经典星座（is_quad_base_feature24_classic）窗口扫描命中。
const QCLASSIC_HITS_K: u64 = 65536;
/// 条目：`(低 20 位表条目, [(seed, 半径 f32 位模式)])`
const QCLASSIC_HITS: &[ScanGroup] = &[
    (0x43f18, &[
        (26102803108, 0x3f800000),
        (27177593508, 0x3f800000),
        (46804839076, 0x3f800000),
        (62546061988, 0x3f800000),
        (63586249380, 0x3f800000),
        (64190229156, 0x3f800000),
    ]),
    (0x79a0a, &[
        (11906352534, 0x3f800000),
        (12375066006, 0x3f800000),
        (14526743958, 0x3f800000),
        (27039401366, 0x3f800000),
        (29052667286, 0x3f800000),
        (64419038614, 0x3f800000),
        (64421135766, 0x3f800000),
    ]),
    (0xc751a, &[
        (8855314598, 0x3f800000),
        (10030768294, 0x3f800000),
        (12651159718, 0x3f800000),
        (28251873446, 0x3f800000),
    ]),
    (0xf520a, &[
        (11840798102, 0x3f800000),
        (30058757526, 0x3f800000),
        (30665883030, 0x3f800000),
        (38751452566, 0x3f800000),
        (47305735574, 0x3f800000),
        (61502406038, 0x3f800000),
        (63882673558, 0x3f800000),
    ]),
];

#[test]
fn quad_classic_scan_matches_c() {
    let conf = hut_conf();
    let salt = conf.salt as u32 as u64;
    let got = scan_window(LOW20_QUAD_CLASSIC, salt, QCLASSIC_HITS_K, |s| {
        is_quad_base_feature24_classic(&conf, s)
    });
    let want: Vec<(u64, Vec<SeedBits>)> = QCLASSIC_HITS
        .iter()
        .map(|&(e, h)| (e, h.to_vec()))
        .collect();
    assert_eq!(got, want);
    // classic 变体命中时恒返回 1.0（C 注释：实际应为 122.781311/127.887650）
    for &(_, hits) in QCLASSIC_HITS {
        for &(_, bits) in hits {
            assert_eq!(bits, 1.0f32.to_bits());
        }
    }
}

/// 小屋 radius=124（dispatcher → feature 一般路径）窗口扫描命中。
const QFEATURE_HITS_K: u64 = 262144;
/// 条目：`(低 20 位表条目, [(seed, 半径 f32 位模式)])`
const QFEATURE_HITS: &[ScanGroup] = &[
    (0x43f18, &[
        (26102803108, 0x42f05768),
        (27177593508, 0x42f05768),
        (46804839076, 0x42f05768),
        (62546061988, 0x42f05768),
        (63586249380, 0x42f05768),
        (64190229156, 0x42f05768),
        (77542795940, 0x42f05768),
        (79793040036, 0x42f05768),
        (80021629604, 0x42f05768),
        (96369978020, 0x42f05768),
        (99319622308, 0x42f05768),
        (113009830564, 0x42f05768),
        (113616956068, 0x42f05768),
        (130156145316, 0x42f05768),
        (147403123364, 0x42f05768),
        (163509250724, 0x42f05768),
        (181694704292, 0x42f05768),
        (198976285348, 0x42f05768),
        (200521886372, 0x42f05768),
        (201596676772, 0x42f05768),
        (217164884644, 0x42f05768),
        (217768864420, 0x42f05768),
        (218843654820, 0x42f05768),
    ]),
    (0x65118, &[
        (11740593316, 0x42f3a5b5),
        (17876860068, 0x42f3a5b5),
        (60325313700, 0x42f3a5b5),
        (62103698596, 0x42f3a5b5),
        (62338579620, 0x42f3a5b5),
        (64488160420, 0x42f3a5b5),
        (70810025124, 0x42f3a5b5),
        (72823291044, 0x42f3a5b5),
        (79350676644, 0x42f3a5b5),
        (79486991524, 0x42f3a5b5),
        (81598823588, 0x42f3a5b5),
        (97704950948, 0x42f3a5b5),
        (114951928996, 0x42f3a5b5),
        (130087074980, 0x42f3a5b5),
        (132100340900, 0x42f3a5b5),
        (140571786404, 0x42f3a5b5),
        (165453446308, 0x42f3a5b5),
        (167565278372, 0x42f3a5b5),
        (173788576932, 0x42f3a5b5),
        (176036723876, 0x42f3a5b5),
        (184713690276, 0x42f3a5b5),
        (218066795684, 0x42f3a5b5),
        (234038705316, 0x42f3a5b5),
        (236286852260, 0x42f3a5b5),
        (251283586212, 0x42f3a5b5),
        (253298949284, 0x42f3a5b5),
        (266418732196, 0x42f3a5b5),
        (268429900964, 0x42f3a5b5),
        (268431998116, 0x42f3a5b5),
        (270680145060, 0x42f3a5b5),
    ]),
    (0x75618, &[
        (11610636708, 0x42f3a5b5),
        (12182110628, 0x42f3a5b5),
        (26136560036, 0x42f3a5b5),
        (61973741988, 0x42f3a5b5),
        (64221888932, 0x42f3a5b5),
        (64223986084, 0x42f3a5b5),
        (79220720036, 0x42f3a5b5),
        (106115645860, 0x42f3a5b5),
        (114587091364, 0x42f3a5b5),
        (115158565284, 0x42f3a5b5),
        (121004376484, 0x42f3a5b5),
        (148982481316, 0x42f3a5b5),
        (165088608676, 0x42f3a5b5),
        (167200440740, 0x42f3a5b5),
        (182335586724, 0x42f3a5b5),
        (184447418788, 0x42f3a5b5),
        (200553546148, 0x42f3a5b5),
        (202566812068, 0x42f3a5b5),
        (209329078692, 0x42f3a5b5),
        (234480222628, 0x42f3a5b5),
        (251490222500, 0x42f3a5b5),
        (253168992676, 0x42f3a5b5),
        (255182258596, 0x42f3a5b5),
    ]),
    (0xb5e18, &[
        (2820688292, 0x42f3a5b5),
        (11508140452, 0x42f3a5b5),
        (27108854180, 0x42f3a5b5),
        (28185741732, 0x42f3a5b5),
        (28422719908, 0x42f3a5b5),
        (38285625764, 0x42f3a5b5),
        (44355832228, 0x42f3a5b5),
        (45432719780, 0x42f3a5b5),
        (46507510180, 0x42f3a5b5),
        (61637413284, 0x42f3a5b5),
        (62009657764, 0x42f3a5b5),
        (78884391332, 0x42f3a5b5),
        (79256635812, 0x42f3a5b5),
        (96870615460, 0x42f3a5b5),
        (97474595236, 0x42f3a5b5),
        (97947503012, 0x42f3a5b5),
        (114117593508, 0x42f3a5b5),
        (114623007140, 0x42f3a5b5),
        (136029686180, 0x42f3a5b5),
        (150087944612, 0x42f3a5b5),
        (162839677348, 0x42f3a5b5),
        (163440511396, 0x42f3a5b5),
        (167236356516, 0x42f3a5b5),
        (180687489444, 0x42f3a5b5),
        (182233090468, 0x42f3a5b5),
        (184014621092, 0x42f3a5b5),
        (200589461924, 0x42f3a5b5),
        (215216048548, 0x42f3a5b5),
        (233806252452, 0x42f3a5b5),
        (251053230500, 0x42f3a5b5),
        (251526138276, 0x42f3a5b5),
        (269744097700, 0x42f3a5b5),
        (269981075876, 0x42f3a5b5),
    ]),
    (0xc751a, &[
        (8855314598, 0x42f05768),
        (10030768294, 0x42f05768),
        (12651159718, 0x42f05768),
        (28251873446, 0x42f05768),
        (77542285478, 0x42f05768),
        (79221055654, 0x42f05768),
        (94821769382, 0x42f05768),
        (95997223078, 0x42f05768),
        (96367370406, 0x42f05768),
        (113614348454, 0x42f05768),
        (115158900902, 0x42f05768),
        (132307312806, 0x42f05768),
        (146831138982, 0x42f05768),
        (148982816934, 0x42f05768),
        (149554290854, 0x42f05768),
        (163508740262, 0x42f05768),
        (167200776358, 0x42f05768),
        (183843774630, 0x42f05768),
        (184447754406, 0x42f05768),
        (197800321190, 0x42f05768),
        (201125355686, 0x42f05768),
        (201594069158, 0x42f05768),
        (232328880294, 0x42f05768),
        (234949271718, 0x42f05768),
        (249575858342, 0x42f05768),
        (250413670566, 0x42f05768),
        (269810229414, 0x42f05768),
    ]),
    (0xf520a, &[
        (11840798102, 0x42f05768),
        (30058757526, 0x42f05768),
        (30665883030, 0x42f05768),
        (38751452566, 0x42f05768),
        (47305735574, 0x42f05768),
        (61502406038, 0x42f05768),
        (63882673558, 0x42f05768),
        (73650158998, 0x42f05768),
        (81129651606, 0x42f05768),
        (95793986966, 0x42f05768),
        (97235778966, 0x42f05768),
        (97807252886, 0x42f05768),
        (148172455318, 0x42f05768),
        (150891412886, 0x42f05768),
        (183637392790, 0x42f05768),
        (197834063254, 0x42f05768),
        (215081041302, 0x42f05768),
        (217461308822, 0x42f05768),
        (234138910102, 0x42f05768),
        (253399154070, 0x42f05768),
        (269603847574, 0x42f05768),
        (270678637974, 0x42f05768),
        (272224238998, 0x42f05768),
    ]),
];

#[test]
fn quad_feature_scan_matches_c() {
    let conf = hut_conf();
    let salt = conf.salt as u32 as u64;
    let got = scan_window(LOW20_QUAD_HUT_NORMAL, salt, QFEATURE_HITS_K, |s| {
        is_quad_base(&conf, s, 124)
    });
    let want: Vec<(u64, Vec<SeedBits>)> = QFEATURE_HITS
        .iter()
        .map(|&(e, h)| (e, h.to_vec()))
        .collect();
    assert_eq!(got, want);
}

/// 海底神殿窗口扫描：`(radius, 窗口大小, [(seed, 半径 f32 位模式)])`。
/// radius=128 的窗口全不命中（四连海底神殿极稀有，该窗口验证早期拒绝路径）。
const QLARGE_SCANS: &[(i32, u64, &[SeedBits])] = &[
    (128, 4194304, &[
    ]),
    (200, 16777216, &[
        (15144902, 0x4342d42a),
    ]),
];

#[test]
fn quad_monument_scan_matches_c() {
    let conf = mon_conf();
    for &(radius, n, want) in QLARGE_SCANS {
        let mut got = Vec::new();
        for s in 0..n {
            let r = is_quad_base(&conf, s, radius);
            if r != 0.0 {
                got.push((s, r.to_bits()));
            }
        }
        let want: Vec<SeedBits> = want.to_vec();
        assert_eq!(got, want, "monument scan radius {radius}");
    }
}

#[test]
fn scan_for_quads_matches_c() {
    // 小屋：s48 = 窗口扫描找到的第一个底座，扫 region [-3,3]²，lowBitN=20
    let conf = hut_conf();
    let salt = conf.salt as u32 as u64;
    let mut qp = [Pos::default(); 64];
    let n = scan_for_quads(
        &conf, 128, 26102803108, LOW20_QUAD_HUT_NORMAL, 20, salt, -3, -3, 6, 6, &mut qp,
    );
    let want = [
Pos { x: 0, z: 0 },
];
    assert_eq!(n, want.len());
    assert_eq!(&qp[..n], &want);

    // 海底神殿：lowBitN=48，低比特值 = (底座 + salt) mod 2^48
    let conf = mon_conf();
    let mut qm = [Pos::default(); 8];
    let n = scan_for_quads(
        &conf, 200, 15144902, &[25532215], 48,
        conf.salt as u32 as u64, -1, -1, 2, 2, &mut qm,
    );
    let want = [
Pos { x: 0, z: 0 },
];
    assert_eq!(n, want.len());
    assert_eq!(&qm[..n], &want);
}

/// `(seed, afk.x, afk.z, 刷怪面积)`：真实四连小屋底座（1.13+ 配置）。
const AFK_CASES: &[(u64, i32, i32, i32)] = &[
    (26102803108, 444, 444, 320),
    (27177593508, 444, 444, 320),
    (46804839076, 444, 444, 320),
    (62546061988, 444, 444, 320),
    (63586249380, 444, 444, 320),
    (64190229156, 444, 444, 320),
    (11740593316, 435, 453, 320),
    (17876860068, 435, 453, 320),
];

#[test]
fn optimal_afk_matches_c() {
    for &(seed, want_x, want_z, want_cnt) in AFK_CASES {
        // 与 isQuadBase* 内部一致：p0=(0,0) p1=(1,1) p2=(1,0) p3=(0,1)
        let p = [
            get_structure_pos(StructureType::SwampHut, McVersion::V1_13, seed, 0, 0).unwrap(),
            get_structure_pos(StructureType::SwampHut, McVersion::V1_13, seed, 1, 1).unwrap(),
            get_structure_pos(StructureType::SwampHut, McVersion::V1_13, seed, 1, 0).unwrap(),
            get_structure_pos(StructureType::SwampHut, McVersion::V1_13, seed, 0, 1).unwrap(),
        ];
        let (afk, spcnt) = get_optimal_afk(p, 8, 8, 10);
        assert_eq!((afk.x, afk.z, spcnt), (want_x, want_z, want_cnt),
            "afk of seed {seed}");
    }
}

/// 全 48 位搜索结果摘要：`(数量, 回绕和, 异或和)`。
const SEARCH48_SUMMARY: (usize, u64, u64) = (74474, 10480816149066680760, 617795116336);
const SEARCH48_FIRST: &[u64] = &[8855314598, 10030768294, 11840798102, 12651159718, 26102803108, 27177593508, 28251873446, 30058757526];
const SEARCH48_LAST: &[u64] = &[281435012423846, 281435015031460, 281435880832406, 281450512661910, 281453200485028, 281470380892326, 281470479968932, 281472289488278];

#[test]
fn search_all48_matches_c() {
    let conf = hut_conf();
    let threads = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4);
    let got = search_all48(threads, Some((LOW20_QUAD_IDEAL_ADJ, 20)), |s| {
        is_quad_base(&conf, s, 128) != 0.0
    }, None);
    let (want_n, want_sum, want_xor) = SEARCH48_SUMMARY;
    assert_eq!(got.len(), want_n, "search_all48 count");
    let sum = got.iter().fold(0u64, |a, &s| a.wrapping_add(s));
    let xor = got.iter().fold(0u64, |a, &s| a ^ s);
    assert_eq!((sum, xor), (want_sum, want_xor), "search_all48 checksums");
    assert_eq!(&got[..SEARCH48_FIRST.len()], SEARCH48_FIRST);
    assert_eq!(&got[got.len() - SEARCH48_LAST.len()..], SEARCH48_LAST);
    // 与 C 一致：低比特子集模式的结果不是全局升序（高位块步进 × 值集数组序），
    // 但每个高位块内、每个线程分区内的顺序是确定的，首尾样本已覆盖该行为。
}

/// LOW20_QUAD_IDEAL 减去小屋 salt 后的实际低 20 位取值（C 侧由生成器计算）。
const LOW20_QUAD_IDEAL_ADJ: &[u64] = &[0x92aa4, 0x160a6, 0x43d96];

