//! Bedrock 群系层栈逐层对拍测试。
//!
//! 数据源：`tests/fixtures/bedrock_layers.json`（由 `reference/site` 探针脚本
//! 从 `bedrock.wasm` 导出 `p` 的内部层栈转储），已转写为 [`bedrock_layer_data`]。
//!
//! 覆盖两部分：
//! - 栈构造：两个种子下 54 层的全字段快照（scale/s1/s2 由 f_1/f_p 传播产生）；
//! - 逐层求值：54 层 × 3 个区域的输出向量（层 53 的 f_ga 在过滤路径中
//!   从不被求值，Rust 侧不实现，跳过）。

mod bedrock_layer_data;

use bedrock_layer_data::{LayerSnapshot, LAYER_VECTORS, STACK_SEED12345, STACK_SEED_NEG};
use minecraft_seed_core::bedrock::layers::LayerStack;

/// seedNeg 对应的 64 位种子（两个 u32 零扩展拼接，与 wasm 位模式一致）。
const SEED_NEG: i64 = ((0xA8F3_F4A9_u64 << 32) | 0x8A8F_3F4A_u64) as i64;

fn check_stack(seed: i64, expected: &[LayerSnapshot]) {
    let stack = LayerStack::new(seed);
    assert_eq!(stack.layers.len(), expected.len(), "层数不符");
    for (i, (got, want)) in stack.layers.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got.func, want.func, "层 {i}: func");
        assert_eq!(got.b5, want.b5, "层 {i}: b5");
        assert_eq!(got.b6, want.b6, "层 {i}: b6");
        assert_eq!(got.scale, want.scale, "层 {i}: scale");
        assert_eq!(got.salt, want.salt, "层 {i}: salt");
        assert_eq!(got.s1, want.s1, "层 {i}: s1");
        assert_eq!(got.s2, want.s2, "层 {i}: s2");
        assert_eq!(got.p1, want.p1, "层 {i}: p1");
        assert_eq!(got.p2, want.p2, "层 {i}: p2");
    }
}

#[test]
fn stack_seed12345() {
    check_stack(12345, STACK_SEED12345);
}

#[test]
fn stack_seed_neg() {
    check_stack(SEED_NEG, STACK_SEED_NEG);
}

#[test]
fn layer_vectors() {
    let stack = LayerStack::new(12345);
    let mut failures = Vec::new();
    for v in LAYER_VECTORS {
        if v.layer == 53 {
            continue; // f_ga 不实现（见模块文档）
        }
        let [x, z, w, h] = v.area;
        // 与探针 probe_bedrock_layers.mjs 完全一致：(w+128)×(h+128) 缓冲区、
        // 0x7fffffff 哨兵填充 —— zoom 层会读父层区域右/下各一格的"越界"数据，
        // 探针里这些格子是哨兵值，必须复刻才能逐层对拍。
        let mut buf = vec![0x7fff_ffffi32; ((w + 128) * (h + 128)) as usize];
        stack.apply(v.layer, &mut buf, x, z, w, h);
        let got = &buf[..(w * h) as usize];
        if got != v.values {
            let first_diff = got
                .iter()
                .zip(v.values.iter())
                .position(|(a, b)| a != b)
                .unwrap();
            failures.push(format!(
                "层 {} 区域 {:?}: 首个差异在格 {}（{} 行 {} 列）：got {} want {}",
                v.layer,
                v.area,
                first_diff,
                first_diff / w as usize,
                first_diff % w as usize,
                got[first_diff],
                v.values[first_diff]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
