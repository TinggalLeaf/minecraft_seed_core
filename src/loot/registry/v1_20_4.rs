//! Minecraft 1.20.2–1.20.4 战利品表。
//!
//! 这些补丁版本未改动任何 loot table（对照 1.20.1 数据快照），因此
//! **不复制数据**，整体复用 [`super::v1_20_1`]：`include_str!` 只在
//! `v1_20_1.rs` 出现一次，二进制中只存一份 JSON。
//!
//! 将来若某版本与 1.20.1 也完全相同，照本文件再加一个 re-export 即可；
//! 若有差异，则用 `scripts/gen_registry.py` 生成独立的 `v<version>.rs`。

pub use super::v1_20_1::{get_raw, lookup_short, SHORT_NAMES, TABLES};
