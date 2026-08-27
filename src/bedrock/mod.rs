//! Bedrock 版种子计算（对齐 mcseedmap.com Bedrock 模式的 `bedrock.wasm`）。
//!
//! 与 Java 版（crate 根下的 [`crate::generator`]/[`crate::structure`]）的关键差异：
//!
//! - 随机源为标准 **MT19937**，且几乎所有计算只用种子的**低 32 位**
//!   （region 散布的 region 种子用完整 64 位种子参与加法，但 MT 初始化仍取低 32 位）；
//! - 版本分派只体现在结构配置表的选择上（`village`/`ocean_ruin`/`shipwreck`
//!   在 mc>17 即 1.18+ 换用新配置）；出生点与要塞与版本无关；
//! - 要塞角度使用 wasm 内自定义的 2π/步长常量与 musl 变体 sin/cos，
//!   见 [`trig`] 模块文档。
//!
//! 全部函数逐指令移植自 `reference/site/bedrock.wasm`（见 `docs/INTEGRATION.md`），
//! 并由 `tests/bedrock_consistency.rs` 与网站 WASM 输出对拍。

#[doc(hidden)] pub mod mt; // 仅供一致性测试直接校验 MT19937 向量
mod trig;
mod version;
mod spawn;
mod structure;

pub use spawn::{get_spawn, get_strongholds};
pub use structure::{
    find_structures, get_config, get_config_raw, get_structure_pos, structures_in_regions,
    BeStructureConfig, BeStructureType,
};
pub use version::BedrockVersion;
