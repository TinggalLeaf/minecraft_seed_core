# -*- coding: utf-8 -*-
r"""生成 src/loot/registry/v1_20_1.rs。

从 Python 参考项目 `E:\Projects\Minecraft\宝箱内容生成` 的
`src/loot_registry.py` 拉取 `ALL_TABLES`，但只保留 `data/loot/1.20.1/`
下实际有 JSON 文件的表（其余 id 在 Rust 侧将解析失败 → 调用方报错）。

用法（在 Python 项目根目录运行）：
    cd "E:/Projects/Minecraft/宝箱内容生成" \
    python "E:/Projects/Minecraft/minecraft_seed_core/scripts/gen_registry.py" \
    > "E:/Projects/Minecraft/minecraft_seed_core/src/loot/registry/v1_20_1.rs"

注意：若新版本数据与已有版本完全一致，不要运行本脚本复制数据——
在 `src/loot/registry/` 下新建 `v<version>.rs` 做 re-export 即可
（参照 `v1_20_4.rs`）。
"""
import io
import os
import sys

SOURCE_PROJECT = r"E:\Projects\Minecraft\宝箱内容生成"
DATA_DIR = r"E:\Projects\Minecraft\minecraft_seed_core\data\loot\1.20.1"

# 必须从源项目根目录加载（让 `from .loot_table import ...` 工作）
os.chdir(SOURCE_PROJECT)
sys.path.insert(0, SOURCE_PROJECT)

from src import loot_registry as R  # noqa: E402

# 还原 stdout 编码（Windows 默认为 GBK）
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

# 过滤：只保留 DATA_DIR 下实际存在的 JSON
def has_file(rel_path: str) -> bool:
    full = os.path.join(DATA_DIR, rel_path.replace("/", os.sep))
    return os.path.isfile(full)

filtered = [(k, v) for k, v in R.ALL_TABLES.items() if has_file(v)]
# 按 id 排序：Rust 侧 get_raw 用二分查找（853 次字符串比较 → ~10 次）。
filtered.sort(key=lambda kv: kv[0])
skipped = [(k, v) for k, v in R.ALL_TABLES.items() if not has_file(v)]

out = []
out.append('//! 由 `scripts/gen_registry.py` 自动生成（请勿手改）。')
out.append(f'//! 数据源：1.20.1 原版 `data/loot/1.20.1/`，共 {len(filtered)} 张表。')
if skipped:
    out.append(f'//! 跳过 {len(skipped)} 个 id（data 目录下无 JSON）：')
    for k, v in skipped[:10]:
        out.append(f"//!   - {k} -> {v}")
    if len(skipped) > 10:
        out.append(f'//!   ... 还有 {len(skipped) - 10} 个')
out.append('')
out.append('/// `(loot_table_id, relative_path_under_data/loot/1.20.1/)` 元组列表。')
out.append('/// **按 id 字典序排序**（`get_raw` 依赖此序做二分查找）。')
out.append('pub static TABLES: &[(&str, &str)] = &[')
for k, v in filtered:
    out.append(f'    ("{k}", "{v}"),')
out.append('];')
out.append('')
out.append('/// 按表 id 获取 JSON 字符串（编译期 include_str!，二分查找）。')
out.append('pub fn get_raw(loot_table_id: &str) -> Option<&\'static str> {')
out.append('    let idx = TABLES')
out.append('        .binary_search_by(|(id, _)| id.cmp(&loot_table_id))')
out.append('        .ok()?;')
out.append('    Some(match TABLES[idx].1 {')
for k, v in filtered:
    rel = v.replace("\\", "/")
    out.append(f'        "{rel}" => include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/loot/1.20.1/{rel}")),')
out.append('        _ => unreachable!("unhandled table path"),')
out.append('    })')
out.append('}')
out.append('')

# 短名别名表：与 Python `loot_registry.py::SHORT_NAMES` 完全一致
# （basename setdefault + sheep_ 前缀 + friendly 覆盖表）。
out.append('/// `(short_name, loot_table_id)` 短名别名表，与 Python')
out.append('/// `loot_registry.py::SHORT_NAMES` 一致。')
out.append('pub static SHORT_NAMES: &[(&str, &str)] = &[')
for short, full in R.SHORT_NAMES.items():
    if full not in {k for k, _ in filtered}:
        continue
    out.append(f'    ("{short}", "{full}"),')
out.append('];')
out.append('')
out.append('/// 按短名查全 id。')
out.append('pub fn lookup_short(short: &str) -> Option<&\'static str> {')
out.append('    SHORT_NAMES.iter().find(|(s, _)| *s == short).map(|(_, f)| *f)')
out.append('}')
sys.stdout.write('\n'.join(out))
