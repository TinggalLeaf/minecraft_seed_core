//! 结构部件（piece）生成：末地城部件树、下界堡垒布局、1.13 及更早
//! 村庄的房屋列表。
//!
//! 逐函数移植自 cubiomes `finders.c` 的 `getEndCityPieces` /
//! `getFortressPieces` / `getHouseList`（finders.h:413/444/493）。
//!
//! 与 C 的差异仅在于内存管理：C 需要调用方提供固定大小的 `Piece` 缓冲
//! （末地城 `END_CITY_PIECES_MAX = 421`），并用 `next` 指针模拟处理队列；
//! 这里改为返回 `Vec<Piece>`，堡垒的处理队列用保序的 `Vec<usize>`
//! 复刻链表的随机抽取/尾部追加语义，随机数消费顺序与 C 完全一致。

use crate::rng::java::JavaRandom;
use crate::structure::region::{chunk_generate_rnd, set_attempt_seed};
use crate::version::McVersion;

/// 三维方块坐标（对应 cubiomes `Pos3`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pos3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// 结构部件（对应 cubiomes `Piece`，去掉 `next` 链表指针）。
///
/// `pos` 为部件原点，`bb0`/`bb1` 为包围盒两端（含），`rot` 为朝向
/// （0:北 1:东 2:南 3:西），`depth` 为递归生成分配的生代深度，
/// `piece_type` 取值见 [`end_city`] / [`fortress`] 模块常量。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    /// 部件名（cubiomes 静态表中的名字）。
    pub name: &'static str,
    pub pos: Pos3,
    pub bb0: Pos3,
    pub bb1: Pos3,
    pub rot: i32,
    pub depth: i32,
    pub piece_type: i32,
}

/// 末地城部件类型常量（对应 finders.h 的 End City piece types 枚举）。
pub mod end_city {
    pub const BASE_FLOOR: i32 = 0;
    pub const BASE_ROOF: i32 = 1;
    pub const BRIDGE_END: i32 = 2;
    pub const BRIDGE_GENTLE_STAIRS: i32 = 3;
    pub const BRIDGE_PIECE: i32 = 4;
    pub const BRIDGE_STEEP_STAIRS: i32 = 5;
    pub const FAT_TOWER_BASE: i32 = 6;
    pub const FAT_TOWER_MIDDLE: i32 = 7;
    pub const FAT_TOWER_TOP: i32 = 8;
    pub const SECOND_FLOOR_1: i32 = 9;
    pub const SECOND_FLOOR_2: i32 = 10;
    pub const SECOND_ROOF: i32 = 11;
    pub const END_SHIP: i32 = 12;
    pub const THIRD_FLOOR_1: i32 = 13;
    pub const THIRD_FLOOR_2: i32 = 14;
    pub const THIRD_ROOF: i32 = 15;
    pub const TOWER_BASE: i32 = 16;
    #[allow(dead_code)]
    pub const TOWER_FLOOR: i32 = 17; // C 中未使用
    pub const TOWER_PIECE: i32 = 18;
    pub const TOWER_TOP: i32 = 19;
    /// C 侧建议的部件缓冲上限 `END_CITY_PIECES_MAX`。
    pub const PIECES_MAX: usize = 421;
}

/// 下界堡垒部件类型常量（对应 finders.h 的 Fortress piece types 枚举）。
pub mod fortress {
    pub const FORTRESS_START: i32 = 0;
    pub const BRIDGE_STRAIGHT: i32 = 1;
    pub const BRIDGE_CROSSING: i32 = 2;
    pub const BRIDGE_FORTIFIED_CROSSING: i32 = 3;
    pub const BRIDGE_STAIRS: i32 = 4;
    pub const BRIDGE_SPAWNER: i32 = 5;
    pub const BRIDGE_CORRIDOR_ENTRANCE: i32 = 6;
    pub const CORRIDOR_STRAIGHT: i32 = 7;
    pub const CORRIDOR_CROSSING: i32 = 8;
    pub const CORRIDOR_TURN_RIGHT: i32 = 9;
    pub const CORRIDOR_TURN_LEFT: i32 = 10;
    pub const CORRIDOR_STAIRS: i32 = 11;
    pub const CORRIDOR_T_CROSSING: i32 = 12;
    pub const CORRIDOR_NETHER_WART: i32 = 13;
    pub const FORTRESS_END: i32 = 14;
    /// 部件类型总数（C 的 `PIECE_COUNT`）。
    pub const PIECE_COUNT: usize = 15;
}

/// 1.13 及更早村庄的房屋类型下标（对应 finders.h 的房屋枚举）。
pub mod house {
    pub const HOUSE_SMALL: usize = 0;
    pub const CHURCH: usize = 1;
    pub const LIBRARY: usize = 2;
    pub const WOOD_HUT: usize = 3;
    pub const BUTCHER: usize = 4;
    pub const FARM_LARGE: usize = 5;
    pub const FARM_SMALL: usize = 6;
    pub const BLACKSMITH: usize = 7;
    pub const HOUSE_LARGE: usize = 8;
    /// 房屋类型总数（C 的 `HOUSE_NUM`）。
    pub const HOUSE_NUM: usize = 9;
}

// =============================================================================
// 末地城（getEndCityPieces）
// =============================================================================

/// `addEndCityPiece` 的静态部件信息表：尺寸 (sx, sy, sz) 与名字。
struct EndCityInfo {
    sx: i32,
    sy: i32,
    sz: i32,
    name: &'static str,
}

const END_CITY_INFO: [EndCityInfo; 20] = [
    EndCityInfo { sx: 9, sy: 3, sz: 9, name: "base_floor" },
    EndCityInfo { sx: 11, sy: 1, sz: 11, name: "base_roof" },
    EndCityInfo { sx: 4, sy: 5, sz: 1, name: "bridge_end" },
    EndCityInfo { sx: 4, sy: 6, sz: 7, name: "bridge_gentle_stairs" },
    EndCityInfo { sx: 4, sy: 5, sz: 3, name: "bridge_piece" },
    EndCityInfo { sx: 4, sy: 6, sz: 3, name: "bridge_steep_stairs" },
    EndCityInfo { sx: 12, sy: 3, sz: 12, name: "fat_tower_base" },
    EndCityInfo { sx: 12, sy: 7, sz: 12, name: "fat_tower_middle" },
    EndCityInfo { sx: 16, sy: 5, sz: 16, name: "fat_tower_top" },
    EndCityInfo { sx: 11, sy: 7, sz: 11, name: "second_floor_1" },
    EndCityInfo { sx: 11, sy: 7, sz: 11, name: "second_floor_2" },
    EndCityInfo { sx: 13, sy: 1, sz: 13, name: "second_roof" },
    EndCityInfo { sx: 12, sy: 23, sz: 28, name: "ship" },
    EndCityInfo { sx: 13, sy: 7, sz: 13, name: "third_floor_1" },
    EndCityInfo { sx: 13, sy: 7, sz: 13, name: "third_floor_2" },
    EndCityInfo { sx: 15, sy: 1, sz: 15, name: "third_roof" },
    EndCityInfo { sx: 6, sy: 6, sz: 6, name: "tower_base" },
    EndCityInfo { sx: 6, sy: 3, sz: 6, name: "tower_floor" }, // unused
    EndCityInfo { sx: 6, sy: 3, sz: 6, name: "tower_piece" },
    EndCityInfo { sx: 8, sy: 4, sz: 8, name: "tower_top" },
];

/// 末地城生成上下文，对应 C 的 `PieceEnv`。
///
/// C 里 `PieceEnv` 在每次递归时按值复制（`env->y` 只向下传递），而
/// `ship` 是指针（全局共享）。这里 `y` 改为逐帧传参，`ship` 挂在
/// builder 上共享，语义等价。
///
/// 注意 C 的碰撞检查范围是**父批次的缓冲区**（`env->list[0..*env->n]`，
/// 其中 `env` 是父帧按值复制的 `PieceEnv`，其 `list` 指向父批次起点），
/// 而不是全局已接受的全部部件。`batch_stack` 记录各帧批次的起始下标，
/// 用于复刻这一范围。
struct EndCityBuilder {
    pieces: Vec<Piece>,
    rng: JavaRandom,
    ship: bool,
    batch_stack: Vec<usize>,
}

type EndCityGenFn = fn(&mut EndCityBuilder, usize, i32, i32) -> bool;

impl EndCityBuilder {
    /// `addEndCityPiece`：向当前批次追加一个部件，返回其下标。
    fn add_piece(
        &mut self,
        prev: Option<usize>,
        rot: i32,
        px: i32,
        py: i32,
        pz: i32,
        typ: usize,
    ) -> usize {
        let info = &END_CITY_INFO[typ];
        let idx = self.pieces.len();
        let mut pos = Pos3 { x: px, y: py, z: pz };
        if let Some(pi) = prev {
            pos = self.pieces[pi].pos;
        }
        let mut p = Piece {
            name: info.name,
            pos,
            bb0: pos,
            bb1: pos,
            rot,
            depth: 0,
            piece_type: typ as i32,
        };
        p.bb1.y += info.sy;
        match rot {
            0 => {
                p.bb1.x += info.sx;
                p.bb1.z += info.sz;
            }
            1 => {
                p.bb0.x -= info.sz;
                p.bb1.z += info.sx;
            }
            2 => {
                p.bb0.x -= info.sx;
                p.bb0.z -= info.sz;
            }
            3 => {
                p.bb1.x += info.sz;
                p.bb0.z -= info.sx;
            }
            _ => unreachable!(),
        }
        if let Some(pi) = prev {
            let (mut dx, dy, mut dz) = (0, py, 0);
            match self.pieces[pi].rot {
                0 => {
                    dx += px;
                    dz += pz;
                }
                1 => {
                    dx -= pz;
                    dz += px;
                }
                2 => {
                    dx -= px;
                    dz -= pz;
                }
                3 => {
                    dx += pz;
                    dz -= px;
                }
                _ => unreachable!(),
            }
            p.pos.x += dx;
            p.pos.y += dy;
            p.pos.z += dz;
            p.bb0.x += dx;
            p.bb0.y += dy;
            p.bb0.z += dz;
            p.bb1.x += dx;
            p.bb1.y += dy;
            p.bb1.z += dz;
        }
        self.pieces.push(p);
        idx
    }

    /// `genPiecesRecusively`（原文如此）：在局部批次内生成子树，碰撞
    /// 检查通过才并入总表，失败则整批回滚（C 靠不写回 `*env->n` 丢弃，
    /// 这里用 `truncate` 复刻）。
    fn gen_pieces_recursively(
        &mut self,
        gen_fn: EndCityGenFn,
        current: usize,
        depth: i32,
        y: i32,
    ) -> bool {
        if depth > 8 {
            return false;
        }
        let base = self.pieces.len();
        self.batch_stack.push(base);
        let ok = gen_fn(self, current, depth, y);
        self.batch_stack.pop();
        if !ok {
            self.pieces.truncate(base);
            return false;
        }
        // C 的 Piece.depth 是 int8_t，next(rng, 32) 的完整 32 位值会被
        // 截断为低 8 位；这里的碰撞检查也比较截断后的值。
        let gendepth = self.rng.next(32) as i8 as i32;
        // 碰撞检查范围是父批次 [parent_start, base)，见 batch_stack 注释。
        let parent_start = self.batch_stack.last().copied().unwrap_or(0);
        let n = self.pieces.len();
        for i in base..n {
            self.pieces[i].depth = gendepth;
            let p = self.pieces[i];
            for q in self.pieces[parent_start..base].iter() {
                // 与父批次内先行部件的包围盒碰撞检查
                if q.bb1.x >= p.bb0.x
                    && q.bb0.x <= p.bb1.x
                    && q.bb1.z >= p.bb0.z
                    && q.bb0.z <= p.bb1.z
                    && q.bb1.y >= p.bb0.y
                    && q.bb0.y <= p.bb1.y
                {
                    if self.pieces[current].depth != q.depth {
                        self.pieces.truncate(base);
                        return false;
                    }
                    break;
                }
            }
        }
        true
    }
}

/// `genTower`。
fn gen_tower(b: &mut EndCityBuilder, current: usize, depth: i32, y: i32) -> bool {
    let rot = b.pieces[current].rot;
    let x = 3 + b.rng.next_int_bound(2);
    let z = 3 + b.rng.next_int_bound(2);
    let mut base = current;
    base = b.add_piece(Some(base), rot, x, -3, z, end_city::TOWER_BASE as usize);
    base = b.add_piece(Some(base), rot, 0, 7, 0, end_city::TOWER_PIECE as usize);
    let mut floor = b.rng.next_int_bound(3) == 0;
    let floorcnt = 1 + b.rng.next_int_bound(3);
    for i in 0..floorcnt {
        base = b.add_piece(Some(base), rot, 0, 4, 0, end_city::TOWER_PIECE as usize);
        if i < floorcnt - 1 && b.rng.next(1) != 0 {
            floor = true;
        }
    }
    if floor {
        const BINFO: [[i32; 4]; 4] = [
            [0, 1, -1, 0],  // 0
            [1, 6, -1, 1],  // 90
            [3, 0, -1, 5],  // 270
            [2, 5, -1, 6],  // 180
        ];
        for bi in &BINFO {
            if b.rng.next(1) == 0 {
                continue;
            }
            let brot = (rot + bi[0]) & 3;
            let bridge =
                b.add_piece(Some(base), brot, bi[1], bi[2], bi[3], end_city::BRIDGE_END as usize);
            b.gen_pieces_recursively(gen_bridge, bridge, depth + 1, y);
        }
    } else if depth != 7 {
        return b.gen_pieces_recursively(gen_fat_tower, base, depth + 1, y);
    }
    b.add_piece(Some(base), rot, -1, 4, -1, end_city::TOWER_TOP as usize);
    true
}

/// `genBridge`。
fn gen_bridge(b: &mut EndCityBuilder, current: usize, depth: i32, _y: i32) -> bool {
    let rot = b.pieces[current].rot;
    let floorcnt = 1 + b.rng.next_int_bound(4);
    let mut base = current;
    base = b.add_piece(Some(base), rot, 0, 0, -4, end_city::BRIDGE_PIECE as usize);
    b.pieces[base].depth = -1;
    let mut y = 0;
    for _ in 0..floorcnt {
        if b.rng.next(1) != 0 {
            base = b.add_piece(Some(base), rot, 0, y, -4, end_city::BRIDGE_PIECE as usize);
            y = 0;
            continue;
        }
        if b.rng.next(1) != 0 {
            base = b.add_piece(Some(base), rot, 0, y, -4, end_city::BRIDGE_STEEP_STAIRS as usize);
        } else {
            base =
                b.add_piece(Some(base), rot, 0, y, -8, end_city::BRIDGE_GENTLE_STAIRS as usize);
        }
        y = 4;
    }
    if !b.ship && b.rng.next_int_bound(10 - depth) == 0 {
        let x = -8 + b.rng.next_int_bound(8);
        let z = -70 + b.rng.next_int_bound(10);
        base = b.add_piece(Some(base), rot, x, y, z, end_city::END_SHIP as usize);
        b.ship = true;
    } else {
        // C 里这里写 `env->y = y + 1` 再递归 genHouseTower；
        // y 只向下传递，等价于直接把 y + 1 传给下一帧。
        if !b.gen_pieces_recursively(gen_house_tower, base, depth + 1, y + 1) {
            return false;
        }
    }
    base = b.add_piece(Some(base), (rot + 2) & 3, 4, y, 0, end_city::BRIDGE_END as usize);
    b.pieces[base].depth = -1;
    true
}

/// `genHouseTower`。
fn gen_house_tower(b: &mut EndCityBuilder, current: usize, depth: i32, y: i32) -> bool {
    if depth > 8 {
        return false;
    }
    let rot = b.pieces[current].rot;
    let mut base = current;
    base = b.add_piece(Some(base), rot, -3, y, -11, end_city::BASE_FLOOR as usize);
    let size = b.rng.next_int_bound(3);
    if size == 0 {
        b.add_piece(Some(base), rot, -1, 4, -1, end_city::BASE_ROOF as usize);
        return true;
    }
    base = b.add_piece(Some(base), rot, -1, 0, -1, end_city::SECOND_FLOOR_2 as usize);
    if size == 1 {
        base = b.add_piece(Some(base), rot, -1, 8, -1, end_city::SECOND_ROOF as usize);
    } else {
        base = b.add_piece(Some(base), rot, -1, 4, -1, end_city::THIRD_FLOOR_2 as usize);
        base = b.add_piece(Some(base), rot, -1, 8, -1, end_city::THIRD_ROOF as usize);
    }
    b.gen_pieces_recursively(gen_tower, base, depth + 1, y);
    true
}

/// `genFatTower`。
fn gen_fat_tower(b: &mut EndCityBuilder, current: usize, depth: i32, y: i32) -> bool {
    let rot = b.pieces[current].rot;
    let mut base = current;
    base = b.add_piece(Some(base), rot, -3, 4, -3, end_city::FAT_TOWER_BASE as usize);
    base = b.add_piece(Some(base), rot, 0, 4, 0, end_city::FAT_TOWER_MIDDLE as usize);
    const BINFO: [[i32; 4]; 4] = [
        [0, 4, -1, 0],   // 0
        [1, 12, -1, 4],  // 90
        [3, 0, -1, 8],   // 270
        [2, 8, -1, 12],  // 180
    ];
    let mut j = 0;
    while j < 2 && b.rng.next_int_bound(3) != 0 {
        base = b.add_piece(Some(base), rot, 0, 8, 0, end_city::FAT_TOWER_MIDDLE as usize);
        for bi in &BINFO {
            if b.rng.next(1) == 0 {
                continue;
            }
            let brot = (rot + bi[0]) & 3;
            let bridge =
                b.add_piece(Some(base), brot, bi[1], bi[2], bi[3], end_city::BRIDGE_END as usize);
            b.gen_pieces_recursively(gen_bridge, bridge, depth + 1, y);
        }
        j += 1;
    }
    b.add_piece(Some(base), rot, -2, 8, -2, end_city::FAT_TOWER_TOP as usize);
    true
}

/// 生成末地城部件树（对应 `getEndCityPieces`，finders.h:413）。
///
/// `chunk_x`/`chunk_z` 为 16×16 区块坐标（应取自真实可生成末地城的区块，
/// 见 [`crate::structure::get_structure_pos`]）。返回按接受顺序排列的部件。
pub fn get_end_city_pieces(seed: u64, chunk_x: i32, chunk_z: i32) -> Vec<Piece> {
    let mut b = EndCityBuilder {
        pieces: Vec::new(),
        rng: chunk_generate_rnd(seed, chunk_x, chunk_z),
        ship: false,
        batch_stack: Vec::new(),
    };
    let rot = b.rng.next_int_bound(4);
    let x = chunk_x * 16 + 8;
    let z = chunk_z * 16 + 8;
    let mut base = b.add_piece(None, rot, x, 0, z, end_city::BASE_FLOOR as usize);
    base = b.add_piece(Some(base), rot, -1, 0, -1, end_city::SECOND_FLOOR_1 as usize);
    base = b.add_piece(Some(base), rot, -1, 4, -1, end_city::THIRD_FLOOR_1 as usize);
    base = b.add_piece(Some(base), rot, -1, 8, -1, end_city::THIRD_ROOF as usize);
    b.gen_pieces_recursively(gen_tower, base, 1, 0);
    b.pieces
}

// =============================================================================
// 下界堡垒（getFortressPieces）
// =============================================================================

/// `fortress_info` 静态表：包围盒偏移/尺寸、随机跳过数、可重复性、
/// 权重、数量上限与名字。
struct FortressInfo {
    offset: Pos3,
    size: Pos3,
    skip: u64,
    repeatable: bool,
    weight: i32,
    max: i32,
    name: &'static str,
}

#[allow(clippy::too_many_arguments)] // 与 fortress_info 表的列一一对应
const fn fi(
    ox: i32, oy: i32, oz: i32,
    sx: i32, sy: i32, sz: i32,
    skip: u64, repeatable: bool, weight: i32, max: i32,
    name: &'static str,
) -> FortressInfo {
    FortressInfo {
        offset: Pos3 { x: ox, y: oy, z: oz },
        size: Pos3 { x: sx, y: sy, z: sz },
        skip, repeatable, weight, max, name,
    }
}

const FORTRESS_INFO: [FortressInfo; fortress::PIECE_COUNT] = [
    fi(0, 0, 0, 18, 9, 18, 0, false, 0, 0, "NeStart"), // FORTRESS_START
    fi(-1, -3, 0, 4, 9, 18, 0, true, 30, 0, "NeBS"),  // BRIDGE_STRAIGHT
    fi(-8, -3, 0, 18, 9, 18, 0, false, 10, 4, "NeBCr"), // BRIDGE_CROSSING
    fi(-2, 0, 0, 6, 8, 6, 0, false, 10, 4, "NeRC"),   // BRIDGE_FORTIFIED_CROSSING
    fi(-2, 0, 0, 6, 10, 6, 0, false, 10, 3, "NeSR"),  // BRIDGE_STAIRS
    fi(-2, 0, 0, 6, 7, 8, 0, false, 5, 2, "NeMT"),    // BRIDGE_SPAWNER
    fi(-5, -3, 0, 12, 13, 12, 0, false, 5, 1, "NeCE"), // BRIDGE_CORRIDOR_ENTRANCE
    fi(-1, 0, 0, 4, 6, 4, 0, true, 25, 0, "NeSC"),    // CORRIDOR_STRAIGHT
    fi(-1, 0, 0, 4, 6, 4, 0, false, 15, 5, "NeSCSC"), // CORRIDOR_CROSSING
    fi(-1, 0, 0, 4, 6, 4, 1, false, 5, 10, "NeSCRT"), // CORRIDOR_TURN_RIGHT
    fi(-1, 0, 0, 4, 6, 4, 1, false, 5, 10, "NeSCLT"), // CORRIDOR_TURN_LEFT
    fi(-1, -7, 0, 4, 13, 9, 0, true, 10, 3, "NeCCS"), // CORRIDOR_STAIRS
    fi(-3, 0, 0, 8, 6, 8, 0, false, 7, 2, "NeCTB"),   // CORRIDOR_T_CROSSING
    fi(-5, -3, 0, 12, 13, 12, 0, false, 5, 2, "NeCSR"), // CORRIDOR_NETHER_WART
    fi(-1, -3, 0, 4, 9, 7, 1, false, 0, 0, "NeBEF"),  // FORTRESS_END
];

/// 堡垒生成上下文，对应 C 的 `PieceEnv`（堡垒用法）。
///
/// C 用 `next` 指针维护待处理队列：新部件追加到链尾，主循环随机抽一个
/// 摘下。这里 `queue` 用保序 `Vec<usize>` 复刻同一语义。
struct FortressBuilder {
    pieces: Vec<Piece>,
    queue: Vec<usize>,
    rng: JavaRandom,
    ntyp: [i32; fortress::PIECE_COUNT],
    typlast: i32,
}

impl FortressBuilder {
    /// `addFortressPiece`：碰撞检查通过后按 `pending` 决定是否接受。
    /// 返回接受后的下标；碰撞或 `pending == false`（部件被丢弃）返回
    /// `None`。注意随机数跳过发生在碰撞检查之后、接受与否无关，与 C 一致。
    #[allow(clippy::too_many_arguments)] // 与 C 的 addFortressPiece 参数一致
    fn add_piece(
        &mut self,
        typ: usize,
        x: i32,
        y: i32,
        z: i32,
        depth: i32,
        facing: i32,
        pending: bool,
    ) -> Option<usize> {
        let info = &FORTRESS_INFO[typ];
        let pos = Pos3 { x, y, z };
        let (mut b0, mut b1) = (pos, pos);
        let (d0, d1) = (info.offset, info.size);
        b0.y += d0.y;
        b1.y += d0.y + d1.y;
        match facing {
            0 => {
                // 北
                b0.x += d0.x;
                b0.z += d0.z - d1.z;
                b1.x += d0.x + d1.x;
                b1.z += d0.z;
            }
            1 => {
                // 东
                b0.x += d0.z;
                b0.z += d0.x;
                b1.x += d0.z + d1.z;
                b1.z += d0.x + d1.x;
            }
            2 => {
                // 南
                b0.x += d0.x;
                b0.z += d0.z;
                b1.x += d0.x + d1.x;
                b1.z += d0.z + d1.z;
            }
            3 => {
                // 西
                b0.x += d0.z - d1.z;
                b0.z += d0.x;
                b1.x += d0.z;
                b1.z += d0.x + d1.x;
            }
            _ => unreachable!(),
        }
        for q in &self.pieces {
            if q.bb1.x >= b0.x
                && q.bb0.x <= b1.x
                && q.bb1.z >= b0.z
                && q.bb0.z <= b1.z
                && q.bb1.y >= b0.y
                && q.bb0.y <= b1.y
            {
                return None; // 碰撞
            }
        }
        self.rng.skip(info.skip);
        if !pending {
            return None;
        }
        let idx = self.pieces.len();
        self.pieces.push(Piece {
            name: info.name,
            pos,
            bb0: b0,
            bb1: b1,
            rot: facing,
            depth,
            piece_type: typ as i32,
        });
        self.ntyp[typ] += 1;
        if typ as i32 != fortress::FORTRESS_END {
            self.typlast = typ as i32;
        }
        self.queue.push(idx);
        Some(idx)
    }

    /// `extendFortress`：从部件 `p` 向指定方向延伸一个部件。
    fn extend(&mut self, p: usize, offh: i32, offv: i32, turn: i32, corridor: bool) {
        let pb = self.pieces[p];
        let depth = pb.depth + 1;
        let mut facing = pb.rot;
        let typ0 = if corridor {
            fortress::CORRIDOR_STRAIGHT
        } else {
            fortress::BRIDGE_STRAIGHT
        } as usize;
        let typ1 = typ0 + if corridor { 7 } else { 6 };
        let mut valid = -1;
        let mut weight_tot = 0;

        let y = pb.bb0.y + offv;
        let (x, z);
        if turn == 0 {
            // 前方
            match facing {
                0 => {
                    x = pb.bb0.x + offh;
                    z = pb.bb0.z - 1;
                }
                1 => {
                    x = pb.bb1.x + 1;
                    z = pb.bb0.z + offh;
                }
                2 => {
                    x = pb.bb0.x + offh;
                    z = pb.bb1.z + 1;
                }
                3 => {
                    x = pb.bb0.x - 1;
                    z = pb.bb0.z + offh;
                }
                _ => unreachable!(),
            }
        } else if turn == -1 {
            // 左转
            if facing & 1 != 0 {
                x = pb.bb0.x + offh;
                z = pb.bb0.z - 1;
                facing = 0;
            } else {
                x = pb.bb0.x - 1;
                z = pb.bb0.z + offh;
                facing = 3;
            }
        } else if turn == 1 {
            // 右转
            if facing & 1 != 0 {
                x = pb.bb0.x + offh;
                z = pb.bb1.z + 1;
                facing = 2;
            } else {
                x = pb.bb1.x + 1;
                z = pb.bb0.z + offh;
                facing = 1;
            }
        } else {
            unreachable!();
        }

        // 距起始部件超过 112 格则收尾（valid 保持 -1，不入队列）
        let start = self.pieces[0];
        if (x - start.bb0.x).abs() > 112 || (z - start.bb0.z).abs() > 112 {
            self.add_piece(
                fortress::FORTRESS_END as usize,
                x, y, z, depth, facing,
                valid >= 0,
            );
            return;
        }

        valid = 0;
        for (t, info) in FORTRESS_INFO[typ0..typ1].iter().enumerate() {
            let t = typ0 + t;
            let max = info.max;
            if max > 0 && self.ntyp[t] >= max {
                continue;
            }
            if max > 0 {
                valid = 1;
            }
            weight_tot += info.weight;
        }

        if valid == 0 || weight_tot <= 0 || depth > 30 {
            self.add_piece(
                fortress::FORTRESS_END as usize,
                x, y, z, depth, facing,
                valid >= 0,
            );
            return;
        }

        for _ in 0..5 {
            let mut n = self.rng.next_int_bound(weight_tot);
            for (t, info) in FORTRESS_INFO[typ0..typ1].iter().enumerate() {
                let t = typ0 + t;
                if info.max > 0 && self.ntyp[t] >= info.max {
                    continue;
                }
                n -= info.weight;
                if n >= 0 {
                    continue;
                }
                if self.typlast == t as i32 && !info.repeatable {
                    break;
                }
                if self.add_piece(t, x, y, z, depth, facing, true).is_some() {
                    return;
                }
            }
        }

        self.add_piece(
            fortress::FORTRESS_END as usize,
            x, y, z, depth, facing,
            valid >= 0,
        );
    }

    /// `extendFortressPiece`：按部件类型分发延伸规则。
    fn extend_piece(&mut self, p: usize) {
        let typ = self.pieces[p].piece_type;
        use fortress as f;
        if typ == f::BRIDGE_STRAIGHT {
            self.extend(p, 1, 3, 0, false);
        } else if typ == f::BRIDGE_CROSSING || typ == f::FORTRESS_START {
            self.extend(p, 8, 3, 0, false);
            self.extend(p, 8, 3, -1, false);
            self.extend(p, 8, 3, 1, false);
        } else if typ == f::BRIDGE_FORTIFIED_CROSSING {
            self.extend(p, 2, 0, 0, false);
            self.extend(p, 2, 0, -1, false);
            self.extend(p, 2, 0, 1, false);
        } else if typ == f::BRIDGE_STAIRS {
            self.extend(p, 2, 6, 1, false);
        } else if typ == f::BRIDGE_CORRIDOR_ENTRANCE {
            self.extend(p, 5, 3, 0, true);
        } else if typ == f::CORRIDOR_STRAIGHT {
            self.extend(p, 1, 0, 0, true);
        } else if typ == f::CORRIDOR_CROSSING {
            self.extend(p, 1, 0, 0, true);
            self.extend(p, 1, 0, -1, true);
            self.extend(p, 1, 0, 1, true);
        } else if typ == f::CORRIDOR_TURN_RIGHT {
            self.extend(p, 1, 0, 1, true);
        } else if typ == f::CORRIDOR_TURN_LEFT {
            self.extend(p, 1, 0, -1, true);
        } else if typ == f::CORRIDOR_STAIRS {
            self.extend(p, 1, 0, 0, true);
        } else if typ == f::CORRIDOR_T_CROSSING {
            let h = if self.pieces[p].rot == 0 || self.pieces[p].rot == 3 {
                5
            } else {
                1
            };
            let c0 = self.rng.next_int_bound(8) != 0;
            self.extend(p, h, 0, -1, c0);
            let c1 = self.rng.next_int_bound(8) != 0;
            self.extend(p, h, 0, 1, c1);
        } else if typ == f::CORRIDOR_NETHER_WART {
            self.extend(p, 5, 3, 0, true);
            self.extend(p, 5, 11, 0, true);
        }
    }
}

/// 生成下界堡垒部件布局（对应 `getFortressPieces`，finders.h:444）。
///
/// 1.15 及更早与 1.16+ 的随机源初始化不同（`setAttemptSeed` vs
/// `chunkGenerateRnd`），由 `mc` 自动选择。C 的缓冲上限 `n` 实际从未被
/// 强制执行，这里返回 `Vec<Piece>` 不再设上限。
pub fn get_fortress_pieces(
    mc: McVersion,
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
) -> Vec<Piece> {
    let rng = if mc <= McVersion::V1_15 {
        let mut s = seed;
        let mut r = set_attempt_seed(&mut s, chunk_x, chunk_z);
        r.next_int_bound(3);
        r.next_int_bound(8);
        r.next_int_bound(8);
        r
    } else {
        chunk_generate_rnd(seed, chunk_x, chunk_z)
    };
    let mut b = FortressBuilder {
        pieces: Vec::new(),
        queue: Vec::new(),
        rng,
        ntyp: [0; fortress::PIECE_COUNT],
        typlast: 0,
    };
    b.ntyp[0] = 1;
    let pos = Pos3 {
        x: chunk_x * 16 + 2,
        y: 64,
        z: chunk_z * 16 + 2,
    };
    let start = Piece {
        name: FORTRESS_INFO[0].name,
        pos,
        bb0: pos,
        bb1: Pos3 {
            x: pos.x + FORTRESS_INFO[0].size.x,
            y: pos.y + FORTRESS_INFO[0].size.y,
            z: pos.z + FORTRESS_INFO[0].size.z,
        },
        rot: b.rng.next_int_bound(4),
        depth: 0,
        piece_type: fortress::FORTRESS_START,
    };
    b.pieces.push(start);
    b.extend_piece(0);
    while !b.queue.is_empty() {
        let i = b.rng.next_int_bound(b.queue.len() as i32) as usize;
        let q = b.queue.remove(i);
        b.extend_piece(q);
    }
    b.pieces
}

// =============================================================================
// 村庄房屋列表（getHouseList，mc < 1.14）
// =============================================================================

/// 1.13 及更早村庄的房屋数量表（`getHouseList` 的 Rust 友好返回形式）。
///
/// `houses` 按 [`house`] 模块常量索引；`rng_state` 为 C 原样返回的
/// 48 位 LCG 内部状态（供后续村庄生成流程继续使用）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseList {
    pub houses: [i32; house::HOUSE_NUM],
    pub rng_state: u64,
}

/// 计算村庄各类房屋数量（对应 `getHouseList`，finders.h:493）。
///
/// 仅适用于 mc < 1.14 的旧版村庄；`chunk_x`/`chunk_z` 为村庄原点区块。
#[allow(clippy::identity_op)] // 保留 C 源码 `max - min + 1` 的范围写法
pub fn get_house_list(seed: u64, chunk_x: i32, chunk_z: i32) -> HouseList {
    let mut rng = chunk_generate_rnd(seed, chunk_x, chunk_z);
    rng.skip(1);
    let mut houses = [0; house::HOUSE_NUM];
    houses[house::HOUSE_SMALL] = rng.next_int_bound(4 - 2 + 1) + 2;
    houses[house::CHURCH] = rng.next_int_bound(1 - 0 + 1);
    houses[house::LIBRARY] = rng.next_int_bound(2 - 0 + 1);
    houses[house::WOOD_HUT] = rng.next_int_bound(5 - 2 + 1) + 2;
    houses[house::BUTCHER] = rng.next_int_bound(2 - 0 + 1);
    houses[house::FARM_LARGE] = rng.next_int_bound(4 - 1 + 1) + 1;
    houses[house::FARM_SMALL] = rng.next_int_bound(4 - 2 + 1) + 2;
    houses[house::BLACKSMITH] = rng.next_int_bound(1 - 0 + 1);
    houses[house::HOUSE_LARGE] = rng.next_int_bound(3 - 0 + 1);
    HouseList {
        houses,
        rng_state: rng.raw_state(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 静态表内容直接对照 finders.c 的 info[] / fortress_info[] 逐项核对。
    #[test]
    fn end_city_info_table() {
        assert_eq!(END_CITY_INFO[end_city::BASE_FLOOR as usize].name, "base_floor");
        assert_eq!(END_CITY_INFO[end_city::END_SHIP as usize].name, "ship");
        assert_eq!(
            (END_CITY_INFO[12].sx, END_CITY_INFO[12].sy, END_CITY_INFO[12].sz),
            (12, 23, 28)
        );
        assert_eq!(END_CITY_INFO.len(), 20);
    }

    #[test]
    fn fortress_info_table() {
        assert_eq!(FORTRESS_INFO[0].name, "NeStart");
        assert_eq!(FORTRESS_INFO[fortress::FORTRESS_END as usize].name, "NeBEF");
        assert_eq!(
            FORTRESS_INFO[fortress::BRIDGE_STRAIGHT as usize].weight,
            30
        );
        assert!(FORTRESS_INFO[fortress::BRIDGE_STRAIGHT as usize].repeatable);
        assert_eq!(FORTRESS_INFO[fortress::CORRIDOR_TURN_RIGHT as usize].skip, 1);
    }

    #[test]
    fn house_list_deterministic_and_bounded() {
        let h = get_house_list(0, 0, 0);
        assert!(h.houses[house::HOUSE_SMALL] >= 2);
        assert!(h.houses[house::HOUSE_SMALL] <= 4);
        assert!(h.houses[house::WOOD_HUT] >= 2);
        let h2 = get_house_list(0, 0, 0);
        assert_eq!(h, h2);
    }
}
