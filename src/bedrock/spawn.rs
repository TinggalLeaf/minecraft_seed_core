//! Bedrock 出生点与要塞位置（wasm func24 `be_get_spawn` / func23 `be_get_strongholds`）。
//!
//! 两者都**只看种子的低 32 位**、与版本无关（wasm 中 mc 参数被忽略）：
//! - 出生点：`MT(seed_lo)`，`x = (mt[0] & 511) - 256`，`z = (mt[1] & 511) - 256`；
//! - 要塞：恒为 3 座，首环角度来自 MT 输出（经自定义 2π 常量与 f32 中间舍入），
//!   半径由 `mt[1] & 15` 派生，sin/cos 为 [`crate::bedrock::trig`] 的 musl 变体。

use super::mt::mt_outputs;
use super::trig;

/// 要塞首环角度乘数（wasm 自定义，**不是**精确的 2π）。
const TWO_PI_CUSTOM: f64 = 6.2831855; // 0x1.921fb6134ce3ep+2
/// 相邻要塞环的角度步长（wasm 自定义常量，≈ 2π/3.33…）。
const RING_STEP: f64 = 1.8849558; // 0x1.e28c769b67cffp+0
/// 0x1p-32（f32），用位模式保证精确。
const TWO_POW_NEG_32_F32: f32 = f32::from_bits(0x2F80_0000);

/// `be_get_spawn`：出生点 `(x, z)`。只用种子低 32 位，与版本无关。
pub fn get_spawn(seed: i64) -> [i32; 2] {
    let mt = mt_outputs(seed as u32, 2);
    [
        (mt[0] & 511) as i32 - 256,
        (mt[1] & 511) as i32 - 256,
    ]
}

/// `be_get_strongholds`：3 座初始要塞的 `(x, z)`。只用种子低 32 位，与版本无关。
pub fn get_strongholds(seed: i64) -> [[i32; 2]; 3] {
    let mt = mt_outputs(seed as u32, 2);
    // 首环角度：f32 中间舍入必须保留（wasm 为 f32.convert_i32_u → f32.mul → f64.promote_f32）
    let angle = f64::from(mt[0] as f32 * TWO_POW_NEG_32_F32) * TWO_PI_CUSTOM;
    let c = (mt[1] & 15) as i32;

    let mut out = [[0i32; 2]; 3];
    let mut e = angle;
    for (i, r) in [c + 40, c | 48, c + 56].into_iter().enumerate() {
        let r = f64::from(r);
        // wasm：i32.trunc_sat_f64_s(f64.floor(cos(e)*r)) << 4；Rust as 转换即截断饱和
        out[i][0] = ((trig::cos(e) * r).floor() as i32) << 4;
        out[i][1] = ((trig::sin(e) * r).floor() as i32) << 4;
        e += RING_STEP;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_seed_zero() {
        // 与网站 wasm 输出一致（seed=0 → (-84, -209)）
        assert_eq!(get_spawn(0), [-84, -209]);
    }

    #[test]
    fn spawn_uses_only_low_32_bits() {
        assert_eq!(get_spawn(0), get_spawn(1 << 32));
        assert_eq!(get_strongholds(5), get_strongholds(5 + (7 << 32)));
    }
}
