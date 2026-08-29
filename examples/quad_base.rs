//! 四连底座高速搜索（quadbase）：四连女巫小屋底座判定与 region 扫描。
//!
//! 运行：cargo run --release --example quad_base

use minecraft_seed_core::structure::{
    LOW20_QUAD_HUT_NORMAL, Pos, StructureType, get_config, get_quad_hut_cst, is_quad_base,
    scan_for_quads,
};
use minecraft_seed_core::McVersion;

fn main() {
    let mc = McVersion::V1_20;
    let conf = get_config(StructureType::SwampHut, mc).expect("swamp hut config");
    let salt = conf.salt as u32 as u64;

    // 已知四连小屋底座（来自 golden 测试）：低 48 位 = 26102803108
    let s48 = 26102803108u64;
    let r = is_quad_base(&conf, s48, 128);
    println!(
        "底座种子低48位 {}: 包球半径 {}（>0 即四连底座）",
        s48, r
    );
    println!(
        "低20位 {:#x} 的星座分类: {:?}",
        s48 & 0xFFFFF,
        get_quad_hut_cst(s48 & 0xFFFFF)
    );

    // region 矩形扫描：验证该底座在 region (0,0) 被扫出
    let mut qp = [Pos::default(); 8];
    let n = scan_for_quads(
        &conf,
        128,
        s48,
        LOW20_QUAD_HUT_NORMAL,
        20,
        salt,
        -3,
        -3,
        6,
        6,
        &mut qp,
    );
    println!("scan_for_quads 命中 {} 处: {:?}", n, &qp[..n]);

    println!("\n说明：四连底座位置只由种子低 48 位决定；任意高 16 位的完整种子");
    println!("共享同一底座位置，可用 move_structure 平移底座，用群系检查挑选高位。");
}
