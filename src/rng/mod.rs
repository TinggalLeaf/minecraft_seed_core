//! 随机数核心：Java LCG 与 Xoroshiro128++，及 MC 种子流水线助手。
//!
//! - [`JavaRandom`]：`java.util.Random` 的精确移植（48 位 LCG）。
//!   1.17 及更早版本的所有群系层与结构 rand 都使用它。
//! - [`Xoroshiro`]：1.18+ 气候噪声使用的 Xoroshiro128++（含 MC 的种子扩散）。
//! - [`seed`]：cubiomes 的种子流水线助手（`mcStepSeed`、layer/start/chunk
//!   seed），用于结构查找与旧版分层群系生成。

pub mod java;
pub mod seed;
pub mod xoroshiro;

pub use java::JavaRandom;
pub use xoroshiro::Xoroshiro;
