//! sin/cos 的 musl 变体实现，逐指令移植自 `bedrock.wasm` 的 func7/8/11/13/17。
//!
//! Bedrock 要塞位置依赖 `sin`/`cos` 的**逐位精确**结果，wasm 内嵌的是一套
//! musl libc 的变体（`__rem_pio2` 的常量表被截断定制过，与上游 musl 不同），
//! 因此不能直接调用 Rust 标准库的 `f64::sin`/`cos`（平台 libm 结果不保证一致）。
//!
//! 常量为保持十六进制浮点的可读性以注释标注；十进制值为等值的最短回环表示。
//!
//! 注意：`__rem_pio2` 的大参数路径（|x| 的高位指数 > 1094263290，约 |x| > 5e5）
//! 未移植——要塞角度输入恒在 [0, ~10.06] 内，走不到该路径。

// INV_PIO2/PI_4 与 std 常量同位模式，但此处是 wasm 内嵌常量表的字面移植，保留显式值。
// `x - x` 是 musl 刻意的 NaN 传播写法（inf/NaN 输入 → NaN），勿改。
#![allow(clippy::approx_constant, clippy::eq_op)]

// ---- __sin/__cos 多项式系数（musl，未改动） ----
const S1: f64 = -0.16666666666666632; // -0x1.5555555555549p-3
const S2: f64 = 0.00833333333332249; // 0x1.111111110f8a6p-7
const S3: f64 = -0.0001984126982985795; // -0x1.a01a019c161d5p-13
const S4: f64 = 2.7557313707070068e-06; // 0x1.71de357b1fe7dp-19
const S5: f64 = -2.5050760253406863e-08; // -0x1.ae5e68a2b9cebp-26
const S6: f64 = 1.58969099521155e-10; // 0x1.5d93a5acfd57cp-33

const C1: f64 = 0.0416666666666666; // 0x1.555555555554cp-5
const C2: f64 = -0.001388888888887411; // -0x1.6c16c16c15177p-10
const C3: f64 = 2.480158728947673e-05; // 0x1.a01a019cb159p-16
const C4: f64 = -2.7557314351390663e-07; // -0x1.27e4f809c52adp-22
const C5: f64 = 2.087572321298175e-09; // 0x1.1ee9ebdb4b1c4p-29
const C6: f64 = -1.1359647557788195e-11; // -0x1.8fae9be8838d4p-37

// ---- __rem_pio2 常量（注意：本 wasm 变体把 musl 的部分常量截断定制过） ----
const PIO2_1: f64 = 1.5707963267341256; // 0x1.921fb544p+0
const PIO2_1T: f64 = 6.077100506506192e-11; // 0x1.0b4611a626331p-34
const PIO2_2: f64 = 3.1415926534682512; // 0x1.921fb544p+1
const PIO2_2T: f64 = 1.2154201013012384e-10; // 0x1.0b4611a626331p-33
const PIO2_3: f64 = 4.712388980202377; // 0x1.2d97c7f3p+2
const PIO2_3T: f64 = 1.8231301519518578e-10; // 0x1.90e91a79394cap-33
const PIO2_4: f64 = 6.2831853069365025; // 0x1.921fb544p+2
const PIO2_4T: f64 = 2.430840202602477e-10; // 0x1.0b4611a626331p-32
const INV_PIO2: f64 = 0.6366197723675814; // 0x1.45f306dc9c883p-1
/// 截断版 pio2_1t（与 musl 上游不同，wasm func17 定制）。
const PIO2_1T_CUT: f64 = 6.077100506303966e-11; // 0x1.0b4611a6p-34
const PIO2_2T_FULL: f64 = 2.0222662487959506e-21; // 0x1.3198a2e037073p-69
/// 截断版 pio2_3t（与 musl 上游不同）。
const PIO2_3T_CUT: f64 = 2.0222662487111665e-21; // 0x1.3198a2ep-69
const PIO2_3T_TAIL: f64 = 8.4784276603689e-32; // 0x1.b839a252049c1p-104
const PI_4: f64 = 0.7853981633974483; // 0x1.921fb54442d18p-1
const TWO52: f64 = 6755399441055744.0; // 0x1.8p+52

/// wasm func7 `__sin(x, y, iy)`。
fn sin_kernel(x: f64, y: f64, iy: i32) -> f64 {
    let z = x * x;
    let r = z * z * z * (S5 + z * S6) + (S2 + z * (S3 + z * S4));
    let v = z * x;
    if iy == 0 {
        v * (z * r + S1) + x
    } else {
        x - ((z * (0.5 * y - v * r) - y) + v * (-S1))
    }
}

/// wasm func8 `__cos(x, y)`。
fn cos_kernel(x: f64, y: f64) -> f64 {
    let z = x * x;
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    let r = z * (C1 + z * (C2 + z * C3)) + z * z * (z * z) * (C4 + z * (C5 + z * C6));
    w + (((1.0 - w) - hz) + (z * r - x * y))
}

/// wasm func17 `__rem_pio2(x, y)` 的小参数与中参数路径。
///
/// 返回象限计数 n，并把约化结果写入 `y[0]`（高部）与 `y[1]`（低部）。
/// 大参数路径（|x| 超过 medium 上限）在本库中不可达，见模块文档。
fn rem_pio2(x: f64, y: &mut [f64; 2]) -> i32 {
    let bits = x.to_bits() as i64;
    let hx = (x.to_bits() >> 32) as u32 as i32;
    let ix = hx & 0x7fff_ffff;

    // ---- 小参数路径：|x| ~<= pi/4 的若干档，直接减去 k*(pi/2) ----
    if ix <= 1074752122 {
        if hx & 0x000f_ffff == 598523 {
            // 特例转入 medium 路径
            return rem_pio2_medium(x, ix, y);
        }
        if ix <= 1073928572 {
            if bits >= 0 {
                let t = x + (-PIO2_1);
                let y0 = t + (-PIO2_1T);
                y[1] = (t - y0) + (-PIO2_1T);
                y[0] = y0;
                return 1;
            }
            let t = x + PIO2_1;
            let y0 = t + PIO2_1T;
            y[1] = (t - y0) + PIO2_1T;
            y[0] = y0;
            return -1;
        }
        if bits >= 0 {
            let t = x + (-PIO2_2);
            let y0 = t + (-PIO2_2T);
            y[1] = (t - y0) + (-PIO2_2T);
            y[0] = y0;
            return 2;
        }
        let t = x + PIO2_2;
        let y0 = t + PIO2_2T;
        y[1] = (t - y0) + PIO2_2T;
        y[0] = y0;
        return -2;
    }
    if ix <= 1075594811 {
        if ix <= 1075183036 {
            if ix == 1074977148 {
                return rem_pio2_medium(x, ix, y);
            }
            if bits >= 0 {
                let t = x + (-PIO2_3);
                let y0 = t + (-PIO2_3T);
                y[1] = (t - y0) + (-PIO2_3T);
                y[0] = y0;
                return 3;
            }
            let t = x + PIO2_3;
            let y0 = t + PIO2_3T;
            y[1] = (t - y0) + PIO2_3T;
            y[0] = y0;
            return -3;
        }
        if ix == 1075388923 {
            return rem_pio2_medium(x, ix, y);
        }
        if bits >= 0 {
            let t = x + (-PIO2_4);
            let y0 = t + (-PIO2_4T);
            y[1] = (t - y0) + (-PIO2_4T);
            y[0] = y0;
            return 4;
        }
        let t = x + PIO2_4;
        let y0 = t + PIO2_4T;
        y[1] = (t - y0) + PIO2_4T;
        y[0] = y0;
        return -4;
    }
    if ix > 1094263290 {
        unreachable!("rem_pio2 大参数路径不可达（|x| 受限，见模块文档）");
    }
    rem_pio2_medium(x, ix, y)
}

/// wasm func17 的 medium 路径（`ix <= 1094263290`）。
fn rem_pio2_medium(x: f64, ix: i32, y: &mut [f64; 2]) -> i32 {
    let mut fn_ = x * INV_PIO2 + TWO52 - TWO52;
    let mut n = fn_ as i32; // wasm 为 trunc_sat；此处 |fn_| 恒在 i32 范围内
    let mut r = x + fn_ * (-PIO2_1);
    let mut w = fn_ * PIO2_1T;
    let y0 = r - w;
    if y0 < -PI_4 {
        n -= 1;
        fn_ -= 1.0;
        w = fn_ * PIO2_1T;
        r = x + fn_ * (-PIO2_1);
    } else if y0 > PI_4 {
        n += 1;
        fn_ += 1.0;
        w = fn_ * PIO2_1T;
        r = x + fn_ * (-PIO2_1);
    }
    let mut y0 = r - w;
    let ex = (ix as u32 >> 20) as i32;
    let ey = ((y0.to_bits() >> 52) & 2047) as i32;
    if ex - ey >= 17 {
        let w1 = fn_ * PIO2_1T_CUT;
        let r1 = r - w1;
        let w2 = fn_ * PIO2_2T_FULL - ((r - r1) - w1);
        y0 = r1 - w2;
        let ey2 = ((y0.to_bits() >> 52) & 2047) as i32;
        if ex - ey2 < 50 {
            r = r1;
            w = w2;
            y[0] = y0;
            y[1] = (r - y[0]) - w;
            return n;
        }
        let w3 = fn_ * PIO2_3T_CUT;
        let r2 = r1 - w3;
        let w4 = fn_ * PIO2_3T_TAIL - ((r1 - r2) - w3);
        y0 = r2 - w4;
        r = r2;
        w = w4;
    }
    y[0] = y0;
    y[1] = (r - y[0]) - w;
    n
}

/// wasm func11 `sin`。
pub(crate) fn sin(x: f64) -> f64 {
    let ix = ((x.to_bits() >> 32) as u32 as i32) & 0x7fff_ffff;
    if ix <= 1072243195 {
        if ix < 1045430272 {
            return x;
        }
        return sin_kernel(x, 0.0, 0);
    }
    if ix >= 2146435072u32 as i32 {
        return x - x;
    }
    let mut y = [0.0f64; 2];
    let n = rem_pio2(x, &mut y);
    match n & 3 {
        0 => sin_kernel(y[0], y[1], 1),
        1 => cos_kernel(y[0], y[1]),
        2 => -sin_kernel(y[0], y[1], 1),
        _ => -cos_kernel(y[0], y[1]),
    }
}

/// wasm func13 `cos`。
pub(crate) fn cos(x: f64) -> f64 {
    let ix = ((x.to_bits() >> 32) as u32 as i32) & 0x7fff_ffff;
    if ix <= 1072243195 {
        if ix < 1044816030 {
            return 1.0;
        }
        return cos_kernel(x, 0.0);
    }
    if ix >= 2146435072u32 as i32 {
        return x - x;
    }
    let mut y = [0.0f64; 2];
    let n = rem_pio2(x, &mut y);
    match n & 3 {
        0 => cos_kernel(y[0], y[1]),
        1 => -sin_kernel(y[0], y[1], 1),
        2 => -cos_kernel(y[0], y[1]),
        _ => sin_kernel(y[0], y[1], 1),
    }
}
