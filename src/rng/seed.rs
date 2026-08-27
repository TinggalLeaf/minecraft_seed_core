//! MC 种子流水线助手（cubiomes `rng.h` 的 seed pipeline）。
//!
//! 流水线：`getLayerSalt(n) -> layerSalt(ls)`；
//! `(worldSeed, ls) -> startSalt(st) / startSeed(ss)`；
//! `(ss, x, z) -> chunkSeed(cs)`。
//! 之后 `mcFirstInt(cs, mod)` 得到首个随机整数，`mcStepSeed(cs, st)` 推进。

/// `mcStepSeed`
#[inline]
pub fn step_seed(s: u64, salt: u64) -> u64 {
    s.wrapping_mul(s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407))
        .wrapping_add(salt)
}

/// `mcFirstInt`：对 `mod` 取非负模（等价于 `Math.floorMod`）。
#[inline]
pub fn first_int(s: u64, modulus: i32) -> i32 {
    let ret = ((s as i64) >> 24) % modulus as i64;
    let ret = ret as i32;
    if ret < 0 { ret + modulus } else { ret }
}

/// `mcFirstIsZero`
#[inline]
pub fn first_is_zero(s: u64, modulus: i32) -> bool {
    ((s as i64) >> 24) % modulus as i64 == 0
}

/// `getChunkSeed`
#[inline]
pub fn chunk_seed(start_seed: u64, x: i32, z: i32) -> u64 {
    let mut cs = start_seed.wrapping_add(x as i64 as u64);
    cs = step_seed(cs, z as i64 as u64);
    cs = step_seed(cs, x as i64 as u64);
    cs = step_seed(cs, z as i64 as u64);
    cs
}

/// `getLayerSalt`
#[inline]
pub fn layer_salt(salt: u64) -> u64 {
    let mut ls = step_seed(salt, salt);
    ls = step_seed(ls, salt);
    ls = step_seed(ls, salt);
    ls
}

/// `getStartSalt`
#[inline]
pub fn start_salt(world_seed: u64, layer_salt: u64) -> u64 {
    let mut st = world_seed;
    st = step_seed(st, layer_salt);
    st = step_seed(st, layer_salt);
    st = step_seed(st, layer_salt);
    st
}

/// `getStartSeed`
#[inline]
pub fn start_seed(world_seed: u64, layer_salt: u64) -> u64 {
    let mut ss = world_seed;
    ss = start_salt(ss, layer_salt);
    step_seed(ss, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 由 cubiomes C 实现交叉验证的固定点（结构查找的基础不变量）。
    #[test]
    fn step_seed_known_value() {
        // mcStepSeed(0, 0) == 0
        assert_eq!(step_seed(0, 0), 0);
        // mcStepSeed(1, 0) = 1*(1*6364136223846793005+1442695040888963407)+0
        assert_eq!(
            step_seed(1, 0),
            6364136223846793005u64.wrapping_add(1442695040888963407)
        );
    }

    #[test]
    fn first_int_non_negative() {
        for s in [0u64, 1, 42, u64::MAX, 0x8000_0000_0000_0000] {
            let v = first_int(s, 10);
            assert!((0..10).contains(&v));
        }
    }

    #[test]
    fn pipeline_deterministic() {
        let ls = layer_salt(100);
        let ss = start_seed(12345, ls);
        assert_eq!(chunk_seed(ss, 3, -7), chunk_seed(ss, 3, -7));
        assert_ne!(chunk_seed(ss, 3, -7), chunk_seed(ss, 3, -6));
    }
}
