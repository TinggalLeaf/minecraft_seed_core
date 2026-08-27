//! # minecraft_seed_core
//!
//! 纯 Rust、零外部依赖的 Minecraft 种子计算核心库。
//! 参考 [cubiomes](https://github.com/Cubitect/cubiomes)（mcseedmap.com 的
//! WebAssembly 后端所使用的同一套算法）逐函数移植，保证与 Web 端结果一致。
//!
//! ## 模块结构
//!
//! - [`version`]：Minecraft 版本枚举（Java 1.7 – 1.21.x），按 cubiomes 的
//!   `MCVersion` 对齐，按序可比较。
//! - [`biome`]：全部生物群系 ID 常量与版本存在性查询。
//! - [`rng`]：Java LCG 随机数、Xoroshiro128++（1.18+）、MC 种子流水线
//!   （layerSalt / startSeed / chunkSeed）。
//! - [`noise`]：Perlin / 倍频（Octave）/ DoublePerlin 噪声（1.18+ 气候采样）。
//! - [`generator`]：按版本划分的生物群系生成器（1.18+ 多噪声 / 1.7–1.17 分层）。
//! - [`structure`]：结构候选位置计算（region 种子 + salt 规则）。
//! - [`bedrock`]：Bedrock 版计算（MT19937、结构散布、出生点、要塞，
//!   对齐网站 `bedrock.wasm`）。
//!
//! 每个版本的差异封装在 `generator::v*` 与 `structure` 的版本分派中，
//! 新增版本只需增加一个模块与枚举项。

pub mod bedrock;
pub mod biome;
pub mod rng;
pub mod version;
pub mod noise;
pub mod generator;
pub mod structure;

pub use biome::BiomeId;
pub use generator::{Generator, Range};
pub use structure::StructureType;
pub use version::{Dimension, McVersion};
