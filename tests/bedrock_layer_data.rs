#![allow(dead_code)]
//! 自动生成：Bedrock 群系层栈 golden 数据，由 reference/site/probe_bedrock_layers.mjs 从 bedrock.wasm 转储。
//! 请勿手改；结构见 tests/bedrock_layers.rs。

/// 单层快照（字段与 `LayerStack` 内部层结构一一对应）。
pub struct LayerSnapshot {
    pub idx: usize,
    pub func: i32,
    pub b5: i32,
    pub b6: i32,
    pub scale: i32,
    pub salt: i64,
    pub s1: i64,
    pub s2: i64,
    pub p1: i32,
    pub p2: i32,
}

/// 单层的区域求值向量。
pub struct LayerVector {
    pub layer: usize,
    pub area: [i32; 4],
    pub values: &'static [i32],
}

pub const STACK_SEED12345: &[LayerSnapshot] = &[
    LayerSnapshot { idx: 0, func: 1, b5: 1, b6: 0, scale: 4096, salt: 3107951898966440229, s1: -2202151823110491623, s2: 5693180511283642260, p1: -1, p2: -1 },
    LayerSnapshot { idx: 1, func: 2, b5: 2, b6: 3, scale: 2048, salt: -8774101820360152064, s1: -9176926699766066764, s2: 2214461879716045276, p1: 0, p2: -1 },
    LayerSnapshot { idx: 2, func: 3, b5: 1, b6: 2, scale: 2048, salt: 3107951898966440229, s1: -2202151823110491623, s2: 5693180511283642260, p1: 1, p2: -1 },
    LayerSnapshot { idx: 3, func: 4, b5: 2, b6: 3, scale: 1024, salt: 229918546094678885, s1: -5345742531800403239, s2: -2629338628931509676, p1: 2, p2: -1 },
    LayerSnapshot { idx: 4, func: 3, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 3, p2: -1 },
    LayerSnapshot { idx: 5, func: 3, b5: 1, b6: 2, scale: 1024, salt: -1473395045552829736, s1: 1878191319239717644, s2: 5268221042809191940, p1: 4, p2: -1 },
    LayerSnapshot { idx: 6, func: 3, b5: 1, b6: 2, scale: 1024, salt: 7231908362866731896, s1: -3135687461128383828, s2: -2477812798826881180, p1: 5, p2: -1 },
    LayerSnapshot { idx: 7, func: 5, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 6, p2: -1 },
    LayerSnapshot { idx: 8, func: 6, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 7, p2: -1 },
    LayerSnapshot { idx: 9, func: 3, b5: 1, b6: 2, scale: 1024, salt: 7590731853067264053, s1: 7744462235004260553, s2: -8899090500658609212, p1: 8, p2: -1 },
    LayerSnapshot { idx: 10, func: 7, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 9, p2: -1 },
    LayerSnapshot { idx: 11, func: 8, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 10, p2: -1 },
    LayerSnapshot { idx: 12, func: 9, b5: 1, b6: 2, scale: 1024, salt: 7590731853067264053, s1: 7744462235004260553, s2: -8899090500658609212, p1: 11, p2: -1 },
    LayerSnapshot { idx: 13, func: 4, b5: 2, b6: 3, scale: 512, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 12, p2: -1 },
    LayerSnapshot { idx: 14, func: 4, b5: 2, b6: 3, scale: 256, salt: 3006835321906069877, s1: -7276004378521872759, s2: 2570026718685293444, p1: 13, p2: -1 },
    LayerSnapshot { idx: 15, func: 3, b5: 1, b6: 2, scale: 256, salt: 5360640171528462240, s1: -1843762117891095980, s2: 2284786969955656764, p1: 14, p2: -1 },
    LayerSnapshot { idx: 16, func: 10, b5: 1, b6: 0, scale: 256, salt: 3038466749335869312, s1: -1689295995493398220, s2: -2040464962960187812, p1: 15, p2: -1 },
    LayerSnapshot { idx: 17, func: 11, b5: 1, b6: 2, scale: 256, salt: -7479281634960481323, s1: -8502833966774730583, s2: 134622301503629476, p1: 16, p2: -1 },
    LayerSnapshot { idx: 18, func: 12, b5: 1, b6: 2, scale: 256, salt: 5360640171528462240, s1: -1843762117891095980, s2: 2284786969955656764, p1: 17, p2: -1 },
    LayerSnapshot { idx: 19, func: 13, b5: 1, b6: 0, scale: 256, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 18, p2: -1 },
    LayerSnapshot { idx: 20, func: 4, b5: 2, b6: 3, scale: 128, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 19, p2: -1 },
    LayerSnapshot { idx: 21, func: 4, b5: 2, b6: 3, scale: 64, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 20, p2: -1 },
    LayerSnapshot { idx: 22, func: 14, b5: 1, b6: 2, scale: 64, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 21, p2: -1 },
    LayerSnapshot { idx: 23, func: 15, b5: 1, b6: 0, scale: 256, salt: 5723240131506253216, s1: 516892359012663380, s2: 9130591906023962172, p1: 18, p2: -1 },
    LayerSnapshot { idx: 24, func: 4, b5: 2, b6: 3, scale: 128, salt: 0, s1: 0, s2: 0, p1: 23, p2: -1 },
    LayerSnapshot { idx: 25, func: 4, b5: 2, b6: 3, scale: 64, salt: 0, s1: 0, s2: 0, p1: 24, p2: -1 },
    LayerSnapshot { idx: 26, func: 16, b5: 1, b6: 2, scale: 64, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 22, p2: 25 },
    LayerSnapshot { idx: 27, func: 17, b5: 1, b6: 0, scale: 64, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 26, p2: -1 },
    LayerSnapshot { idx: 28, func: 4, b5: 2, b6: 3, scale: 32, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 27, p2: -1 },
    LayerSnapshot { idx: 29, func: 3, b5: 1, b6: 2, scale: 32, salt: 7590731853067264053, s1: 7744462235004260553, s2: -8899090500658609212, p1: 28, p2: -1 },
    LayerSnapshot { idx: 30, func: 4, b5: 2, b6: 3, scale: 16, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 29, p2: -1 },
    LayerSnapshot { idx: 31, func: 18, b5: 1, b6: 2, scale: 16, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 30, p2: -1 },
    LayerSnapshot { idx: 32, func: 4, b5: 2, b6: 3, scale: 8, salt: 1827289100522298840, s1: 3244956126228409868, s2: 780976904742345476, p1: 31, p2: -1 },
    LayerSnapshot { idx: 33, func: 4, b5: 2, b6: 3, scale: 4, salt: -4039966243449460139, s1: 8445652212021114921, s2: -6901494209584643036, p1: 32, p2: -1 },
    LayerSnapshot { idx: 34, func: 19, b5: 1, b6: 2, scale: 4, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 33, p2: -1 },
    LayerSnapshot { idx: 35, func: 4, b5: 2, b6: 3, scale: 128, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 23, p2: -1 },
    LayerSnapshot { idx: 36, func: 4, b5: 2, b6: 3, scale: 64, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 35, p2: -1 },
    LayerSnapshot { idx: 37, func: 4, b5: 2, b6: 3, scale: 32, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 36, p2: -1 },
    LayerSnapshot { idx: 38, func: 4, b5: 2, b6: 3, scale: 16, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 37, p2: -1 },
    LayerSnapshot { idx: 39, func: 4, b5: 2, b6: 3, scale: 8, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 38, p2: -1 },
    LayerSnapshot { idx: 40, func: 4, b5: 2, b6: 3, scale: 4, salt: 5852781679691581125, s1: -5843250442491085831, s2: -8284349418487666060, p1: 39, p2: -1 },
    LayerSnapshot { idx: 41, func: 20, b5: 1, b6: 2, scale: 4, salt: 3107951898966440229, s1: -2202151823110491623, s2: 5693180511283642260, p1: 40, p2: -1 },
    LayerSnapshot { idx: 42, func: 19, b5: 1, b6: 2, scale: 4, salt: 5692911206796425088, s1: -8751801926545037004, s2: 267861578235887196, p1: 41, p2: -1 },
    LayerSnapshot { idx: 43, func: 21, b5: 1, b6: 0, scale: 4, salt: 5723240131506253216, s1: 516892359012663380, s2: 9130591906023962172, p1: 34, p2: 42 },
    LayerSnapshot { idx: 44, func: 22, b5: 1, b6: 0, scale: 256, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: -1, p2: -1 },
    LayerSnapshot { idx: 45, func: 23, b5: 1, b6: 2, scale: 256, salt: -5014677998924433960, s1: -5000735582825282036, s2: -5744981796498193660, p1: 44, p2: -1 },
    LayerSnapshot { idx: 46, func: 4, b5: 2, b6: 3, scale: 128, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 45, p2: -1 },
    LayerSnapshot { idx: 47, func: 4, b5: 2, b6: 3, scale: 64, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 46, p2: -1 },
    LayerSnapshot { idx: 48, func: 4, b5: 2, b6: 3, scale: 32, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 47, p2: -1 },
    LayerSnapshot { idx: 49, func: 4, b5: 2, b6: 3, scale: 16, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 48, p2: -1 },
    LayerSnapshot { idx: 50, func: 4, b5: 2, b6: 3, scale: 8, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 49, p2: -1 },
    LayerSnapshot { idx: 51, func: 4, b5: 2, b6: 3, scale: 4, salt: 837738509879401688, s1: -3339952365635603188, s2: -2153542429646173180, p1: 50, p2: -1 },
    LayerSnapshot { idx: 52, func: 24, b5: 1, b6: 17, scale: 4, salt: 5723240131506253216, s1: 516892359012663380, s2: 9130591906023962172, p1: 43, p2: 51 },
    LayerSnapshot { idx: 53, func: 25, b5: 4, b6: 7, scale: 1, salt: -8738471090773341224, s1: -7400833155798160372, s2: -1667550387359888124, p1: 52, p2: -1 },
];

pub const STACK_SEED_NEG: &[LayerSnapshot] = &[
    LayerSnapshot { idx: 0, func: 1, b5: 1, b6: 0, scale: 4096, salt: 3107951898966440229, s1: 4018730730739680311, s2: -7440088689908162890, p1: -1, p2: -1 },
    LayerSnapshot { idx: 1, func: 2, b5: 2, b6: 3, scale: 2048, salt: -8774101820360152064, s1: -253125571099535382, s2: -2258170651548656566, p1: 0, p2: -1 },
    LayerSnapshot { idx: 2, func: 3, b5: 1, b6: 2, scale: 2048, salt: 3107951898966440229, s1: 4018730730739680311, s2: -7440088689908162890, p1: 1, p2: -1 },
    LayerSnapshot { idx: 3, func: 4, b5: 2, b6: 3, scale: 1024, salt: 229918546094678885, s1: -2780003067870840329, s2: 2869052856188326774, p1: 2, p2: -1 },
    LayerSnapshot { idx: 4, func: 3, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 3, p2: -1 },
    LayerSnapshot { idx: 5, func: 3, b5: 1, b6: 2, scale: 1024, salt: -1473395045552829736, s1: 6169967864852094754, s2: -2025538029933948750, p1: 4, p2: -1 },
    LayerSnapshot { idx: 6, func: 3, b5: 1, b6: 2, scale: 1024, salt: 7231908362866731896, s1: -3394473632085943486, s2: -5110839197973290734, p1: 5, p2: -1 },
    LayerSnapshot { idx: 7, func: 5, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 6, p2: -1 },
    LayerSnapshot { idx: 8, func: 6, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 7, p2: -1 },
    LayerSnapshot { idx: 9, func: 3, b5: 1, b6: 2, scale: 1024, salt: 7590731853067264053, s1: -312055923205738969, s2: -192077542437227674, p1: 8, p2: -1 },
    LayerSnapshot { idx: 10, func: 7, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 9, p2: -1 },
    LayerSnapshot { idx: 11, func: 8, b5: 1, b6: 2, scale: 1024, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 10, p2: -1 },
    LayerSnapshot { idx: 12, func: 9, b5: 1, b6: 2, scale: 1024, salt: 7590731853067264053, s1: -312055923205738969, s2: -192077542437227674, p1: 11, p2: -1 },
    LayerSnapshot { idx: 13, func: 4, b5: 2, b6: 3, scale: 512, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 12, p2: -1 },
    LayerSnapshot { idx: 14, func: 4, b5: 2, b6: 3, scale: 256, salt: 3006835321906069877, s1: -8343000506315197209, s2: 3960643855108662566, p1: 13, p2: -1 },
    LayerSnapshot { idx: 15, func: 3, b5: 1, b6: 2, scale: 256, salt: 5360640171528462240, s1: -3120226038188263158, s2: -7637348300700273238, p1: 14, p2: -1 },
    LayerSnapshot { idx: 16, func: 10, b5: 1, b6: 0, scale: 256, salt: 3038466749335869312, s1: 2475622428941440362, s2: -1666117729207309110, p1: 15, p2: -1 },
    LayerSnapshot { idx: 17, func: 11, b5: 1, b6: 2, scale: 256, salt: -7479281634960481323, s1: 8214203749232429703, s2: -1120035060666403514, p1: 16, p2: -1 },
    LayerSnapshot { idx: 18, func: 12, b5: 1, b6: 2, scale: 256, salt: 5360640171528462240, s1: -3120226038188263158, s2: -7637348300700273238, p1: 17, p2: -1 },
    LayerSnapshot { idx: 19, func: 13, b5: 1, b6: 0, scale: 256, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 18, p2: -1 },
    LayerSnapshot { idx: 20, func: 4, b5: 2, b6: 3, scale: 128, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 19, p2: -1 },
    LayerSnapshot { idx: 21, func: 4, b5: 2, b6: 3, scale: 64, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 20, p2: -1 },
    LayerSnapshot { idx: 22, func: 14, b5: 1, b6: 2, scale: 64, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 21, p2: -1 },
    LayerSnapshot { idx: 23, func: 15, b5: 1, b6: 0, scale: 256, salt: 5723240131506253216, s1: -2902601916608442614, s2: 7187599410132457386, p1: 18, p2: -1 },
    LayerSnapshot { idx: 24, func: 4, b5: 2, b6: 3, scale: 128, salt: 0, s1: 0, s2: 0, p1: 23, p2: -1 },
    LayerSnapshot { idx: 25, func: 4, b5: 2, b6: 3, scale: 64, salt: 0, s1: 0, s2: 0, p1: 24, p2: -1 },
    LayerSnapshot { idx: 26, func: 16, b5: 1, b6: 2, scale: 64, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 22, p2: 25 },
    LayerSnapshot { idx: 27, func: 17, b5: 1, b6: 0, scale: 64, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 26, p2: -1 },
    LayerSnapshot { idx: 28, func: 4, b5: 2, b6: 3, scale: 32, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 27, p2: -1 },
    LayerSnapshot { idx: 29, func: 3, b5: 1, b6: 2, scale: 32, salt: 7590731853067264053, s1: -312055923205738969, s2: -192077542437227674, p1: 28, p2: -1 },
    LayerSnapshot { idx: 30, func: 4, b5: 2, b6: 3, scale: 16, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 29, p2: -1 },
    LayerSnapshot { idx: 31, func: 18, b5: 1, b6: 2, scale: 16, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 30, p2: -1 },
    LayerSnapshot { idx: 32, func: 4, b5: 2, b6: 3, scale: 8, salt: 1827289100522298840, s1: 769706341937190434, s2: 6823048820176795058, p1: 31, p2: -1 },
    LayerSnapshot { idx: 33, func: 4, b5: 2, b6: 3, scale: 4, salt: -4039966243449460139, s1: 5306891017264436231, s2: -7405635961008208698, p1: 32, p2: -1 },
    LayerSnapshot { idx: 34, func: 19, b5: 1, b6: 2, scale: 4, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 33, p2: -1 },
    LayerSnapshot { idx: 35, func: 4, b5: 2, b6: 3, scale: 128, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 23, p2: -1 },
    LayerSnapshot { idx: 36, func: 4, b5: 2, b6: 3, scale: 64, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 35, p2: -1 },
    LayerSnapshot { idx: 37, func: 4, b5: 2, b6: 3, scale: 32, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 36, p2: -1 },
    LayerSnapshot { idx: 38, func: 4, b5: 2, b6: 3, scale: 16, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 37, p2: -1 },
    LayerSnapshot { idx: 39, func: 4, b5: 2, b6: 3, scale: 8, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 38, p2: -1 },
    LayerSnapshot { idx: 40, func: 4, b5: 2, b6: 3, scale: 4, salt: 5852781679691581125, s1: 5596709375085064855, s2: 2747628026570217110, p1: 39, p2: -1 },
    LayerSnapshot { idx: 41, func: 20, b5: 1, b6: 2, scale: 4, salt: 3107951898966440229, s1: 4018730730739680311, s2: -7440088689908162890, p1: 40, p2: -1 },
    LayerSnapshot { idx: 42, func: 19, b5: 1, b6: 2, scale: 4, salt: 5692911206796425088, s1: 137071530283152746, s2: 2309729127844564170, p1: 41, p2: -1 },
    LayerSnapshot { idx: 43, func: 21, b5: 1, b6: 0, scale: 4, salt: 5723240131506253216, s1: -2902601916608442614, s2: 7187599410132457386, p1: 34, p2: 42 },
    LayerSnapshot { idx: 44, func: 22, b5: 1, b6: 0, scale: 256, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: -1, p2: -1 },
    LayerSnapshot { idx: 45, func: 23, b5: 1, b6: 2, scale: 256, salt: -5014677998924433960, s1: 2499832517810687522, s2: 4806030702596431282, p1: 44, p2: -1 },
    LayerSnapshot { idx: 46, func: 4, b5: 2, b6: 3, scale: 128, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 45, p2: -1 },
    LayerSnapshot { idx: 47, func: 4, b5: 2, b6: 3, scale: 64, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 46, p2: -1 },
    LayerSnapshot { idx: 48, func: 4, b5: 2, b6: 3, scale: 32, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 47, p2: -1 },
    LayerSnapshot { idx: 49, func: 4, b5: 2, b6: 3, scale: 16, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 48, p2: -1 },
    LayerSnapshot { idx: 50, func: 4, b5: 2, b6: 3, scale: 8, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 49, p2: -1 },
    LayerSnapshot { idx: 51, func: 4, b5: 2, b6: 3, scale: 4, salt: 837738509879401688, s1: 3866042592484919586, s2: 1176246878152019634, p1: 50, p2: -1 },
    LayerSnapshot { idx: 52, func: 24, b5: 1, b6: 17, scale: 4, salt: 5723240131506253216, s1: -2902601916608442614, s2: 7187599410132457386, p1: 43, p2: 51 },
    LayerSnapshot { idx: 53, func: 25, b5: 4, b6: 7, scale: 1, salt: -8738471090773341224, s1: 8094348732813208610, s2: -6115715247814920270, p1: 52, p2: -1 },
];

pub const LAYER_VECTORS: &[LayerVector] = &[
    LayerVector { layer: 0, area: [-3, 2, 6, 5], values: LV_0_N3_2 },
    LayerVector { layer: 0, area: [17, -11, 4, 4], values: LV_0_17_N11 },
    LayerVector { layer: 0, area: [0, 0, 3, 3], values: LV_0_0_0 },
    LayerVector { layer: 1, area: [-3, 2, 6, 5], values: LV_1_N3_2 },
    LayerVector { layer: 1, area: [17, -11, 4, 4], values: LV_1_17_N11 },
    LayerVector { layer: 1, area: [0, 0, 3, 3], values: LV_1_0_0 },
    LayerVector { layer: 2, area: [-3, 2, 6, 5], values: LV_2_N3_2 },
    LayerVector { layer: 2, area: [17, -11, 4, 4], values: LV_2_17_N11 },
    LayerVector { layer: 2, area: [0, 0, 3, 3], values: LV_2_0_0 },
    LayerVector { layer: 3, area: [-3, 2, 6, 5], values: LV_3_N3_2 },
    LayerVector { layer: 3, area: [17, -11, 4, 4], values: LV_3_17_N11 },
    LayerVector { layer: 3, area: [0, 0, 3, 3], values: LV_3_0_0 },
    LayerVector { layer: 4, area: [-3, 2, 6, 5], values: LV_4_N3_2 },
    LayerVector { layer: 4, area: [17, -11, 4, 4], values: LV_4_17_N11 },
    LayerVector { layer: 4, area: [0, 0, 3, 3], values: LV_4_0_0 },
    LayerVector { layer: 5, area: [-3, 2, 6, 5], values: LV_5_N3_2 },
    LayerVector { layer: 5, area: [17, -11, 4, 4], values: LV_5_17_N11 },
    LayerVector { layer: 5, area: [0, 0, 3, 3], values: LV_5_0_0 },
    LayerVector { layer: 6, area: [-3, 2, 6, 5], values: LV_6_N3_2 },
    LayerVector { layer: 6, area: [17, -11, 4, 4], values: LV_6_17_N11 },
    LayerVector { layer: 6, area: [0, 0, 3, 3], values: LV_6_0_0 },
    LayerVector { layer: 7, area: [-3, 2, 6, 5], values: LV_7_N3_2 },
    LayerVector { layer: 7, area: [17, -11, 4, 4], values: LV_7_17_N11 },
    LayerVector { layer: 7, area: [0, 0, 3, 3], values: LV_7_0_0 },
    LayerVector { layer: 8, area: [-3, 2, 6, 5], values: LV_8_N3_2 },
    LayerVector { layer: 8, area: [17, -11, 4, 4], values: LV_8_17_N11 },
    LayerVector { layer: 8, area: [0, 0, 3, 3], values: LV_8_0_0 },
    LayerVector { layer: 9, area: [-3, 2, 6, 5], values: LV_9_N3_2 },
    LayerVector { layer: 9, area: [17, -11, 4, 4], values: LV_9_17_N11 },
    LayerVector { layer: 9, area: [0, 0, 3, 3], values: LV_9_0_0 },
    LayerVector { layer: 10, area: [-3, 2, 6, 5], values: LV_10_N3_2 },
    LayerVector { layer: 10, area: [17, -11, 4, 4], values: LV_10_17_N11 },
    LayerVector { layer: 10, area: [0, 0, 3, 3], values: LV_10_0_0 },
    LayerVector { layer: 11, area: [-3, 2, 6, 5], values: LV_11_N3_2 },
    LayerVector { layer: 11, area: [17, -11, 4, 4], values: LV_11_17_N11 },
    LayerVector { layer: 11, area: [0, 0, 3, 3], values: LV_11_0_0 },
    LayerVector { layer: 12, area: [-3, 2, 6, 5], values: LV_12_N3_2 },
    LayerVector { layer: 12, area: [17, -11, 4, 4], values: LV_12_17_N11 },
    LayerVector { layer: 12, area: [0, 0, 3, 3], values: LV_12_0_0 },
    LayerVector { layer: 13, area: [-3, 2, 6, 5], values: LV_13_N3_2 },
    LayerVector { layer: 13, area: [17, -11, 4, 4], values: LV_13_17_N11 },
    LayerVector { layer: 13, area: [0, 0, 3, 3], values: LV_13_0_0 },
    LayerVector { layer: 14, area: [-3, 2, 6, 5], values: LV_14_N3_2 },
    LayerVector { layer: 14, area: [17, -11, 4, 4], values: LV_14_17_N11 },
    LayerVector { layer: 14, area: [0, 0, 3, 3], values: LV_14_0_0 },
    LayerVector { layer: 15, area: [-3, 2, 6, 5], values: LV_15_N3_2 },
    LayerVector { layer: 15, area: [17, -11, 4, 4], values: LV_15_17_N11 },
    LayerVector { layer: 15, area: [0, 0, 3, 3], values: LV_15_0_0 },
    LayerVector { layer: 16, area: [-3, 2, 6, 5], values: LV_16_N3_2 },
    LayerVector { layer: 16, area: [17, -11, 4, 4], values: LV_16_17_N11 },
    LayerVector { layer: 16, area: [0, 0, 3, 3], values: LV_16_0_0 },
    LayerVector { layer: 17, area: [-3, 2, 6, 5], values: LV_17_N3_2 },
    LayerVector { layer: 17, area: [17, -11, 4, 4], values: LV_17_17_N11 },
    LayerVector { layer: 17, area: [0, 0, 3, 3], values: LV_17_0_0 },
    LayerVector { layer: 18, area: [-3, 2, 6, 5], values: LV_18_N3_2 },
    LayerVector { layer: 18, area: [17, -11, 4, 4], values: LV_18_17_N11 },
    LayerVector { layer: 18, area: [0, 0, 3, 3], values: LV_18_0_0 },
    LayerVector { layer: 19, area: [-3, 2, 6, 5], values: LV_19_N3_2 },
    LayerVector { layer: 19, area: [17, -11, 4, 4], values: LV_19_17_N11 },
    LayerVector { layer: 19, area: [0, 0, 3, 3], values: LV_19_0_0 },
    LayerVector { layer: 20, area: [-3, 2, 6, 5], values: LV_20_N3_2 },
    LayerVector { layer: 20, area: [17, -11, 4, 4], values: LV_20_17_N11 },
    LayerVector { layer: 20, area: [0, 0, 3, 3], values: LV_20_0_0 },
    LayerVector { layer: 21, area: [-3, 2, 6, 5], values: LV_21_N3_2 },
    LayerVector { layer: 21, area: [17, -11, 4, 4], values: LV_21_17_N11 },
    LayerVector { layer: 21, area: [0, 0, 3, 3], values: LV_21_0_0 },
    LayerVector { layer: 22, area: [-3, 2, 6, 5], values: LV_22_N3_2 },
    LayerVector { layer: 22, area: [17, -11, 4, 4], values: LV_22_17_N11 },
    LayerVector { layer: 22, area: [0, 0, 3, 3], values: LV_22_0_0 },
    LayerVector { layer: 23, area: [-3, 2, 6, 5], values: LV_23_N3_2 },
    LayerVector { layer: 23, area: [17, -11, 4, 4], values: LV_23_17_N11 },
    LayerVector { layer: 23, area: [0, 0, 3, 3], values: LV_23_0_0 },
    LayerVector { layer: 24, area: [-3, 2, 6, 5], values: LV_24_N3_2 },
    LayerVector { layer: 24, area: [17, -11, 4, 4], values: LV_24_17_N11 },
    LayerVector { layer: 24, area: [0, 0, 3, 3], values: LV_24_0_0 },
    LayerVector { layer: 25, area: [-3, 2, 6, 5], values: LV_25_N3_2 },
    LayerVector { layer: 25, area: [17, -11, 4, 4], values: LV_25_17_N11 },
    LayerVector { layer: 25, area: [0, 0, 3, 3], values: LV_25_0_0 },
    LayerVector { layer: 26, area: [-3, 2, 6, 5], values: LV_26_N3_2 },
    LayerVector { layer: 26, area: [17, -11, 4, 4], values: LV_26_17_N11 },
    LayerVector { layer: 26, area: [0, 0, 3, 3], values: LV_26_0_0 },
    LayerVector { layer: 27, area: [-3, 2, 6, 5], values: LV_27_N3_2 },
    LayerVector { layer: 27, area: [17, -11, 4, 4], values: LV_27_17_N11 },
    LayerVector { layer: 27, area: [0, 0, 3, 3], values: LV_27_0_0 },
    LayerVector { layer: 28, area: [-3, 2, 6, 5], values: LV_28_N3_2 },
    LayerVector { layer: 28, area: [17, -11, 4, 4], values: LV_28_17_N11 },
    LayerVector { layer: 28, area: [0, 0, 3, 3], values: LV_28_0_0 },
    LayerVector { layer: 29, area: [-3, 2, 6, 5], values: LV_29_N3_2 },
    LayerVector { layer: 29, area: [17, -11, 4, 4], values: LV_29_17_N11 },
    LayerVector { layer: 29, area: [0, 0, 3, 3], values: LV_29_0_0 },
    LayerVector { layer: 30, area: [-3, 2, 6, 5], values: LV_30_N3_2 },
    LayerVector { layer: 30, area: [17, -11, 4, 4], values: LV_30_17_N11 },
    LayerVector { layer: 30, area: [0, 0, 3, 3], values: LV_30_0_0 },
    LayerVector { layer: 31, area: [-3, 2, 6, 5], values: LV_31_N3_2 },
    LayerVector { layer: 31, area: [17, -11, 4, 4], values: LV_31_17_N11 },
    LayerVector { layer: 31, area: [0, 0, 3, 3], values: LV_31_0_0 },
    LayerVector { layer: 32, area: [-3, 2, 6, 5], values: LV_32_N3_2 },
    LayerVector { layer: 32, area: [17, -11, 4, 4], values: LV_32_17_N11 },
    LayerVector { layer: 32, area: [0, 0, 3, 3], values: LV_32_0_0 },
    LayerVector { layer: 33, area: [-3, 2, 6, 5], values: LV_33_N3_2 },
    LayerVector { layer: 33, area: [17, -11, 4, 4], values: LV_33_17_N11 },
    LayerVector { layer: 33, area: [0, 0, 3, 3], values: LV_33_0_0 },
    LayerVector { layer: 34, area: [-3, 2, 6, 5], values: LV_34_N3_2 },
    LayerVector { layer: 34, area: [17, -11, 4, 4], values: LV_34_17_N11 },
    LayerVector { layer: 34, area: [0, 0, 3, 3], values: LV_34_0_0 },
    LayerVector { layer: 35, area: [-3, 2, 6, 5], values: LV_35_N3_2 },
    LayerVector { layer: 35, area: [17, -11, 4, 4], values: LV_35_17_N11 },
    LayerVector { layer: 35, area: [0, 0, 3, 3], values: LV_35_0_0 },
    LayerVector { layer: 36, area: [-3, 2, 6, 5], values: LV_36_N3_2 },
    LayerVector { layer: 36, area: [17, -11, 4, 4], values: LV_36_17_N11 },
    LayerVector { layer: 36, area: [0, 0, 3, 3], values: LV_36_0_0 },
    LayerVector { layer: 37, area: [-3, 2, 6, 5], values: LV_37_N3_2 },
    LayerVector { layer: 37, area: [17, -11, 4, 4], values: LV_37_17_N11 },
    LayerVector { layer: 37, area: [0, 0, 3, 3], values: LV_37_0_0 },
    LayerVector { layer: 38, area: [-3, 2, 6, 5], values: LV_38_N3_2 },
    LayerVector { layer: 38, area: [17, -11, 4, 4], values: LV_38_17_N11 },
    LayerVector { layer: 38, area: [0, 0, 3, 3], values: LV_38_0_0 },
    LayerVector { layer: 39, area: [-3, 2, 6, 5], values: LV_39_N3_2 },
    LayerVector { layer: 39, area: [17, -11, 4, 4], values: LV_39_17_N11 },
    LayerVector { layer: 39, area: [0, 0, 3, 3], values: LV_39_0_0 },
    LayerVector { layer: 40, area: [-3, 2, 6, 5], values: LV_40_N3_2 },
    LayerVector { layer: 40, area: [17, -11, 4, 4], values: LV_40_17_N11 },
    LayerVector { layer: 40, area: [0, 0, 3, 3], values: LV_40_0_0 },
    LayerVector { layer: 41, area: [-3, 2, 6, 5], values: LV_41_N3_2 },
    LayerVector { layer: 41, area: [17, -11, 4, 4], values: LV_41_17_N11 },
    LayerVector { layer: 41, area: [0, 0, 3, 3], values: LV_41_0_0 },
    LayerVector { layer: 42, area: [-3, 2, 6, 5], values: LV_42_N3_2 },
    LayerVector { layer: 42, area: [17, -11, 4, 4], values: LV_42_17_N11 },
    LayerVector { layer: 42, area: [0, 0, 3, 3], values: LV_42_0_0 },
    LayerVector { layer: 43, area: [-3, 2, 6, 5], values: LV_43_N3_2 },
    LayerVector { layer: 43, area: [17, -11, 4, 4], values: LV_43_17_N11 },
    LayerVector { layer: 43, area: [0, 0, 3, 3], values: LV_43_0_0 },
    LayerVector { layer: 44, area: [-3, 2, 6, 5], values: LV_44_N3_2 },
    LayerVector { layer: 44, area: [17, -11, 4, 4], values: LV_44_17_N11 },
    LayerVector { layer: 44, area: [0, 0, 3, 3], values: LV_44_0_0 },
    LayerVector { layer: 45, area: [-3, 2, 6, 5], values: LV_45_N3_2 },
    LayerVector { layer: 45, area: [17, -11, 4, 4], values: LV_45_17_N11 },
    LayerVector { layer: 45, area: [0, 0, 3, 3], values: LV_45_0_0 },
    LayerVector { layer: 46, area: [-3, 2, 6, 5], values: LV_46_N3_2 },
    LayerVector { layer: 46, area: [17, -11, 4, 4], values: LV_46_17_N11 },
    LayerVector { layer: 46, area: [0, 0, 3, 3], values: LV_46_0_0 },
    LayerVector { layer: 47, area: [-3, 2, 6, 5], values: LV_47_N3_2 },
    LayerVector { layer: 47, area: [17, -11, 4, 4], values: LV_47_17_N11 },
    LayerVector { layer: 47, area: [0, 0, 3, 3], values: LV_47_0_0 },
    LayerVector { layer: 48, area: [-3, 2, 6, 5], values: LV_48_N3_2 },
    LayerVector { layer: 48, area: [17, -11, 4, 4], values: LV_48_17_N11 },
    LayerVector { layer: 48, area: [0, 0, 3, 3], values: LV_48_0_0 },
    LayerVector { layer: 49, area: [-3, 2, 6, 5], values: LV_49_N3_2 },
    LayerVector { layer: 49, area: [17, -11, 4, 4], values: LV_49_17_N11 },
    LayerVector { layer: 49, area: [0, 0, 3, 3], values: LV_49_0_0 },
    LayerVector { layer: 50, area: [-3, 2, 6, 5], values: LV_50_N3_2 },
    LayerVector { layer: 50, area: [17, -11, 4, 4], values: LV_50_17_N11 },
    LayerVector { layer: 50, area: [0, 0, 3, 3], values: LV_50_0_0 },
    LayerVector { layer: 51, area: [-3, 2, 6, 5], values: LV_51_N3_2 },
    LayerVector { layer: 51, area: [17, -11, 4, 4], values: LV_51_17_N11 },
    LayerVector { layer: 51, area: [0, 0, 3, 3], values: LV_51_0_0 },
    LayerVector { layer: 52, area: [-3, 2, 6, 5], values: LV_52_N3_2 },
    LayerVector { layer: 52, area: [17, -11, 4, 4], values: LV_52_17_N11 },
    LayerVector { layer: 52, area: [0, 0, 3, 3], values: LV_52_0_0 },
    LayerVector { layer: 53, area: [-3, 2, 6, 5], values: LV_53_N3_2 },
    LayerVector { layer: 53, area: [17, -11, 4, 4], values: LV_53_17_N11 },
    LayerVector { layer: 53, area: [0, 0, 3, 3], values: LV_53_0_0 },
];

const LV_0_N3_2: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_0_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_0_0_0: &[i32] = &[1, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_1_N3_2: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1];
const LV_1_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_1_0_0: &[i32] = &[1, 1, 0, 1, 1, 0, 0, 0, 0];
const LV_2_N3_2: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1];
const LV_2_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_2_0_0: &[i32] = &[1, 1, 0, 1, 1, 1, 0, 0, 0];
const LV_3_N3_2: &[i32] = &[1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_3_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_3_0_0: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 1, 1];
const LV_4_N3_2: &[i32] = &[1, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_4_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_4_0_0: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 1, 1];
const LV_5_N3_2: &[i32] = &[0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_5_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_5_0_0: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 1, 1];
const LV_6_N3_2: &[i32] = &[0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LV_6_17_N11: &[i32] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1];
const LV_6_0_0: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 0, 1];
const LV_7_N3_2: &[i32] = &[0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 1];
const LV_7_17_N11: &[i32] = &[0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 1];
const LV_7_0_0: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 0, 1];
const LV_8_N3_2: &[i32] = &[0, 4, 3, 4, 0, 3, 4, 0, 4, 1, 1, 0, 1, 1, 1, 0, 0, 4, 0, 0, 1, 0, 0, 0, 0, 4, 0, 1, 0, 1];
const LV_8_17_N11: &[i32] = &[0, 0, 1, 0, 1, 1, 1, 0, 0, 0, 1, 0, 0, 3, 0, 1];
const LV_8_0_0: &[i32] = &[4, 1, 1, 4, 1, 1, 4, 0, 3];
const LV_9_N3_2: &[i32] = &[1, 4, 3, 4, 0, 3, 4, 0, 4, 1, 1, 0, 1, 1, 1, 0, 1, 4, 4, 1, 1, 1, 1, 0, 0, 4, 0, 1, 1, 0];
const LV_9_17_N11: &[i32] = &[1, 0, 1, 1, 1, 0, 1, 0, 3, 0, 1, 0, 0, 3, 0, 1];
const LV_9_0_0: &[i32] = &[4, 1, 1, 4, 1, 1, 4, 0, 3];
const LV_10_N3_2: &[i32] = &[2, 4, 3, 4, 0, 3, 4, 0, 4, 2, 1, 0, 2, 1, 2, 0, 2, 4, 4, 2, 1, 1, 1, 0, 0, 4, 0, 1, 1, 0];
const LV_10_17_N11: &[i32] = &[2, 0, 1, 1, 2, 0, 1, 0, 3, 0, 1, 0, 0, 3, 0, 1];
const LV_10_0_0: &[i32] = &[4, 2, 1, 4, 2, 2, 4, 0, 3];
const LV_11_N3_2: &[i32] = &[2, 3, 3, 3, 0, 3, 3, 0, 3, 2, 1, 0, 2, 1, 2, 0, 2, 3, 3, 2, 1, 1, 1, 0, 0, 3, 0, 1, 1, 0];
const LV_11_17_N11: &[i32] = &[2, 0, 1, 1, 2, 0, 1, 0, 3, 0, 1, 0, 0, 3, 0, 1];
const LV_11_0_0: &[i32] = &[3, 2, 1, 3, 2, 2, 3, 0, 3];
const LV_12_N3_2: &[i32] = &[2, 3, 3, 3, 0, 3, 3, 0, 3, 2, 1, 0, 2, 1, 2, 0, 2, 3, 3, 2, 1, 2561, 1, 0, 0, 3, 0, 1, 1, 0];
const LV_12_17_N11: &[i32] = &[2, 0, 1, 1, 2, 0, 1, 0, 3, 0, 1, 0, 0, 3, 0, 1];
const LV_12_0_0: &[i32] = &[3, 2, 1, 3075, 2, 2, 3, 0, 3];
const LV_13_N3_2: &[i32] = &[2, 2, 2, 3075, 2, 2, 3, 3, 3, 3075, 3, 0, 3, 3, 3, 3, 0, 0, 3, 3, 3, 3, 2, 0, 0, 3, 2, 2, 2, 1];
const LV_13_17_N11: &[i32] = &[0, 2, 0, 0, 4, 0, 0, 0, 4, 0, 0, 4, 4, 4, 4, 4];
const LV_13_0_0: &[i32] = &[3, 3, 2, 3, 2, 2, 3075, 2, 2];
const LV_14_N3_2: &[i32] = &[2, 2, 3, 3, 3, 2, 2, 2, 2, 3, 2, 2, 2, 2, 3075, 3075, 2, 2, 2, 3, 3075, 3075, 3075, 2, 3, 3, 3075, 3075, 3, 3];
const LV_14_17_N11: &[i32] = &[1, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 2, 1, 1, 2, 2];
const LV_14_0_0: &[i32] = &[3, 3, 3, 3, 3, 3, 3, 3, 2];
const LV_15_N3_2: &[i32] = &[2, 2, 3, 3, 3, 2, 2, 2, 2, 3, 2, 2, 2, 2, 3075, 3075, 2, 2, 2, 3, 3075, 3075, 3075, 2, 3, 3, 3075, 3075, 3, 3];
const LV_15_17_N11: &[i32] = &[1, 1, 0, 0, 1, 1, 1, 4, 1, 1, 1, 2, 1, 1, 2, 2];
const LV_15_0_0: &[i32] = &[3, 3, 3, 3, 3, 3, 3, 3, 2];
const LV_16_N3_2: &[i32] = &[1, 4, 3, 5, 3, 1, 1, 3, 29, 5, 29, 4, 27, 4, 32, 32, 29, 1, 1, 3, 32, 32, 32, 1, 3, 4, 32, 32, 1, 3];
const LV_16_17_N11: &[i32] = &[2, 2, 0, 0, 35, 2, 2, 12, 35, 2, 35, 29, 2, 35, 1, 1];
const LV_16_0_0: &[i32] = &[5, 4, 1, 3, 3, 4, 5, 3, 1];
const LV_17_N3_2: &[i32] = &[1, 4, 3, 5, 3, 1, 1, 3, 29, 5, 29, 4, 27, 4, 32, 32, 29, 1, 1, 3, 32, 32, 32, 1, 3, 4, 32, 32, 1, 3];
const LV_17_17_N11: &[i32] = &[2, 2, 0, 0, 35, 2, 2, 12, 35, 2, 35, 29, 2, 35, 1, 1];
const LV_17_0_0: &[i32] = &[5, 4, 1, 3, 3, 4, 5, 3, 1];
const LV_18_N3_2: &[i32] = &[1, 4, 3, 5, 3, 1, 1, 3, 29, 5, 29, 4, 27, 4, 32, 32, 29, 1, 1, 3, 32, 32, 32, 1, 3, 4, 32, 32, 1, 3];
const LV_18_17_N11: &[i32] = &[2, 2, 0, 0, 35, 2, 2, 12, 35, 2, 35, 29, 2, 35, 1, 1];
const LV_18_0_0: &[i32] = &[5, 4, 1, 3, 3, 4, 5, 3, 1];
const LV_19_N3_2: &[i32] = &[1, 4, 3, 5, 3, 1, 1, 3, 29, 5, 29, 4, 27, 4, 32, 32, 29, 1, 1, 3, 32, 32, 32, 1, 3, 4, 32, 32, 1, 3];
const LV_19_17_N11: &[i32] = &[2, 2, 0, 0, 35, 2, 2, 12, 35, 2, 35, 29, 2, 35, 1, 1];
const LV_19_0_0: &[i32] = &[5, 4, 1, 3, 3, 4, 5, 3, 1];
const LV_20_N3_2: &[i32] = &[3, 3, 3, 3, 3, 3, 3, 3, 3, 5, 3, 3, 4, 3, 5, 5, 5, 3, 3, 3, 5, 5, 5, 29, 3, 29, 29, 5, 29, 29];
const LV_20_17_N11: &[i32] = &[35, 35, 2, 2, 2, 0, 2, 2, 2, 0, 0, 2, 2, 2, 0, 0];
const LV_20_0_0: &[i32] = &[5, 4, 4, 3, 3, 4, 3, 3, 3];
const LV_21_N3_2: &[i32] = &[5, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 5, 3, 3, 3, 3, 3, 5, 3, 3];
const LV_21_17_N11: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 1, 27, 1, 1, 1, 27, 27, 35, 35];
const LV_21_0_0: &[i32] = &[5, 4, 4, 3, 3, 4, 3, 3, 3];
const LV_22_N3_2: &[i32] = &[5, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 5, 3, 3, 3, 3, 3, 5, 3, 3];
const LV_22_17_N11: &[i32] = &[1, 1, 1, 1, 1, 1, 1, 1, 27, 1, 1, 1, 27, 27, 35, 35];
const LV_22_0_0: &[i32] = &[5, 4, 4, 3, 3, 4, 3, 3, 3];
const LV_23_N3_2: &[i32] = &[152657, 113812, 204743, 295289, 296217, 164876, 58590, 15647, 122255, 167760, 90835, 123411, 16088, 113712, 11971, 177765, 252736, 213949, 287649, 192694, 104152, 189089, 31309, 2045, 96031, 168844, 203663, 225771, 21850, 120675];
const LV_23_17_N11: &[i32] = &[23303, 30801, 0, 0, 31667, 242522, 269230, 138127, 151539, 71568, 210391, 21942, 244643, 272154, 254321, 46863];
const LV_23_0_0: &[i32] = &[130184, 205389, 89643, 235304, 279752, 4359, 295289, 296217, 164876];
const LV_24_N3_2: &[i32] = &[160734, 160734, 235304, 235304, 235304, 279752, 160734, 204743, 235304, 235304, 295289, 296217, 204743, 204743, 295289, 295289, 296217, 296217, 113812, 204743, 295289, 295289, 296217, 296217, 122255, 122255, 167760, 167760, 90835, 90835];
const LV_24_17_N11: &[i32] = &[154063, 154063, 154063, 134577, 215808, 0, 0, 134577, 200593, 0, 0, 134577, 107249, 200593, 200593, 0];
const LV_24_0_0: &[i32] = &[130184, 130184, 205389, 130184, 130184, 205389, 235304, 235304, 279752];
const LV_25_N3_2: &[i32] = &[44612, 44612, 130184, 130184, 130184, 130184, 160734, 235304, 235304, 130184, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 235304, 295289, 295289];
const LV_25_17_N11: &[i32] = &[250301, 205260, 205260, 12249, 11345, 250301, 250301, 12249, 11345, 250301, 12249, 12249, 11345, 11345, 11345, 12249];
const LV_25_0_0: &[i32] = &[130184, 130184, 130184, 130184, 130184, 130184, 130184, 130184, 130184];
const LV_26_N3_2: &[i32] = &[5, 3, 131, 131, 131, 131, 34, 3, 3, 131, 3, 3, 34, 3, 3, 34, 3, 3, 3, 3, 3, 5, 3, 3, 34, 3, 3, 5, 3, 3];
const LV_26_17_N11: &[i32] = &[132, 4, 18, 18, 1, 132, 132, 4, 27, 1, 1, 1, 27, 27, 35, 35];
const LV_26_0_0: &[i32] = &[133, 132, 132, 131, 131, 132, 131, 131, 131];
const LV_27_N3_2: &[i32] = &[5, 3, 131, 131, 131, 131, 34, 3, 3, 131, 3, 3, 34, 3, 3, 34, 3, 3, 3, 3, 3, 5, 3, 3, 34, 3, 3, 5, 3, 3];
const LV_27_17_N11: &[i32] = &[132, 4, 18, 18, 1, 132, 132, 4, 27, 1, 1, 1, 27, 27, 35, 35];
const LV_27_0_0: &[i32] = &[133, 132, 132, 131, 131, 132, 131, 131, 131];
const LV_28_N3_2: &[i32] = &[131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 3, 131, 131, 131, 131, 131, 3, 3, 131, 131, 131, 131, 3, 3, 3, 131, 3, 3];
const LV_28_17_N11: &[i32] = &[29, 29, 29, 1, 29, 29, 29, 1, 27, 18, 29, 1, 18, 18, 29, 29];
const LV_28_0_0: &[i32] = &[133, 133, 132, 131, 131, 131, 131, 131, 131];
const LV_29_N3_2: &[i32] = &[131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 3, 131, 131, 131, 131, 131, 3, 3, 131, 131, 131, 131, 3, 3, 3, 131, 3, 3];
const LV_29_17_N11: &[i32] = &[29, 29, 29, 1, 29, 29, 29, 1, 27, 18, 29, 1, 18, 18, 29, 29];
const LV_29_0_0: &[i32] = &[133, 133, 132, 131, 131, 131, 131, 131, 131];
const LV_30_N3_2: &[i32] = &[133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_30_17_N11: &[i32] = &[5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4];
const LV_30_0_0: &[i32] = &[133, 133, 133, 131, 133, 133, 131, 131, 131];
const LV_31_N3_2: &[i32] = &[133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_31_17_N11: &[i32] = &[5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 4, 4];
const LV_31_0_0: &[i32] = &[133, 133, 133, 131, 133, 133, 131, 131, 131];
const LV_32_N3_2: &[i32] = &[131, 131, 131, 131, 133, 133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_32_17_N11: &[i32] = &[5, 19, 19, 19, 5, 133, 19, 19, 133, 133, 133, 133, 133, 133, 133, 133];
const LV_32_0_0: &[i32] = &[133, 133, 133, 133, 133, 133, 131, 133, 133];
const LV_33_N3_2: &[i32] = &[133, 133, 133, 133, 133, 133, 131, 131, 131, 133, 133, 133, 131, 131, 131, 131, 131, 133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_33_17_N11: &[i32] = &[133, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133];
const LV_33_0_0: &[i32] = &[133, 133, 133, 133, 133, 133, 133, 133, 133];
const LV_34_N3_2: &[i32] = &[133, 133, 133, 133, 133, 133, 131, 131, 131, 133, 133, 133, 131, 131, 131, 131, 131, 133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_34_17_N11: &[i32] = &[133, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133];
const LV_34_0_0: &[i32] = &[133, 133, 133, 133, 133, 133, 133, 133, 133];
const LV_35_N3_2: &[i32] = &[190949, 160734, 160734, 235304, 279752, 279752, 190949, 160734, 235304, 295289, 279752, 296217, 113812, 204743, 295289, 295289, 295289, 296217, 204743, 204743, 204743, 167760, 167760, 90835, 15647, 122255, 122255, 167760, 90835, 90835];
const LV_35_17_N11: &[i32] = &[154063, 154063, 134577, 134577, 215808, 0, 134577, 134577, 200593, 0, 0, 134577, 107249, 200593, 0, 0];
const LV_35_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 279752, 279752];
const LV_36_N3_2: &[i32] = &[44612, 160734, 160734, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 235304, 279752, 160734, 160734, 235304, 235304, 235304, 279752, 160734, 160734, 235304, 295289, 279752, 279752, 160734, 235304, 235304, 295289, 279752, 279752];
const LV_36_17_N11: &[i32] = &[250301, 205260, 205260, 205260, 250301, 250301, 205260, 205260, 11345, 250301, 12249, 205260, 11345, 11345, 12249, 12249];
const LV_36_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 130184, 130184];
const LV_37_N3_2: &[i32] = &[44612, 160734, 160734, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 160734, 235304, 235304, 235304, 235304, 160734, 235304, 235304, 235304, 235304, 235304];
const LV_37_17_N11: &[i32] = &[175266, 175266, 175266, 175266, 175266, 175266, 175266, 175266, 100077, 175266, 175266, 175266, 100077, 100077, 214675, 214675];
const LV_37_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 130184, 130184];
const LV_38_N3_2: &[i32] = &[44612, 160734, 160734, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 235304, 235304, 235304, 130184, 130184];
const LV_38_17_N11: &[i32] = &[82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 82987, 89643, 89643];
const LV_38_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 130184, 130184];
const LV_39_N3_2: &[i32] = &[44612, 160734, 160734, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 235304, 235304, 235304, 130184, 130184];
const LV_39_17_N11: &[i32] = &[205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389];
const LV_39_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 130184, 130184];
const LV_40_N3_2: &[i32] = &[44612, 160734, 160734, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 130184, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 160734, 235304, 235304, 235304, 130184, 160734, 235304, 235304, 235304, 130184, 130184];
const LV_40_17_N11: &[i32] = &[205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389, 205389];
const LV_40_0_0: &[i32] = &[130184, 205389, 205389, 235304, 130184, 205389, 235304, 130184, 130184];
const LV_41_N3_2: &[i32] = &[-1, -1, -1, -1, -1, 7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];
const LV_41_17_N11: &[i32] = &[-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];
const LV_41_0_0: &[i32] = &[7, 7, -1, -1, 7, 7, -1, -1, 7];
const LV_42_N3_2: &[i32] = &[-1, -1, -1, -1, -1, 7, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];
const LV_42_17_N11: &[i32] = &[-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];
const LV_42_0_0: &[i32] = &[7, 7, -1, -1, 7, 7, -1, -1, 7];
const LV_43_N3_2: &[i32] = &[133, 133, 133, 133, 133, 7, 131, 131, 131, 133, 133, 133, 131, 131, 131, 131, 131, 133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_43_17_N11: &[i32] = &[133, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133];
const LV_43_0_0: &[i32] = &[7, 7, 133, 133, 7, 7, 133, 133, 7];
const LV_44_N3_2: &[i32] = &[40, 42, 42, 42, 40, 0, 0, 42, 44, 42, 0, 0, 0, 42, 0, 44, 42, 44, 44, 0, 0, 44, 0, 44, 44, 42, 42, 0, 42, 0];
const LV_44_17_N11: &[i32] = &[42, 0, 44, 44, 0, 0, 40, 44, 44, 44, 42, 46, 42, 44, 0, 42];
const LV_44_0_0: &[i32] = &[44, 42, 42, 42, 44, 44, 42, 40, 0];
const LV_45_N3_2: &[i32] = &[40, 42, 42, 42, 40, 0, 0, 42, 44, 42, 0, 0, 0, 42, 0, 44, 42, 44, 44, 0, 0, 44, 0, 44, 44, 42, 42, 0, 42, 0];
const LV_45_17_N11: &[i32] = &[42, 0, 44, 44, 0, 0, 40, 44, 44, 44, 42, 46, 42, 44, 0, 42];
const LV_45_0_0: &[i32] = &[44, 42, 42, 42, 44, 44, 42, 40, 0];
const LV_46_N3_2: &[i32] = &[44, 44, 44, 42, 44, 44, 42, 42, 42, 42, 42, 40, 42, 42, 42, 42, 40, 40, 42, 42, 42, 42, 42, 40, 42, 44, 42, 42, 42, 0];
const LV_46_17_N11: &[i32] = &[42, 40, 44, 0, 0, 42, 42, 0, 0, 42, 42, 42, 42, 42, 42, 42];
const LV_46_0_0: &[i32] = &[44, 44, 42, 44, 42, 44, 42, 44, 44];
const LV_47_N3_2: &[i32] = &[44, 44, 44, 44, 42, 42, 44, 44, 44, 44, 42, 44, 44, 44, 42, 42, 44, 44, 42, 44, 42, 42, 42, 44, 42, 42, 42, 42, 42, 42];
const LV_47_17_N11: &[i32] = &[40, 40, 40, 40, 42, 44, 44, 40, 42, 44, 44, 44, 42, 42, 42, 44];
const LV_47_0_0: &[i32] = &[44, 44, 44, 44, 44, 42, 44, 42, 42];
const LV_48_N3_2: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 42, 44, 44, 44, 44, 42, 42, 44, 44, 44, 44, 44, 42, 44, 44, 44, 44, 44, 42];
const LV_48_17_N11: &[i32] = &[42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42];
const LV_48_0_0: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_49_N3_2: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_49_17_N11: &[i32] = &[42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42];
const LV_49_0_0: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_50_N3_2: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_50_17_N11: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_50_0_0: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_51_N3_2: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_51_17_N11: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_51_0_0: &[i32] = &[44, 44, 44, 44, 44, 44, 44, 44, 44];
const LV_52_N3_2: &[i32] = &[133, 133, 133, 133, 133, 7, 131, 131, 131, 133, 133, 133, 131, 131, 131, 131, 131, 133, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131, 131];
const LV_52_17_N11: &[i32] = &[133, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133, 132, 133, 133, 133];
const LV_52_0_0: &[i32] = &[7, 7, 133, 133, 7, 7, 133, 133, 7];
const LV_53_N3_2: &[i32] = &[133, 133, 7, 7, 7, 7, 133, 133, 133, 7, 7, 7, 133, 133, 133, 133, 7, 7, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133];
const LV_53_17_N11: &[i32] = &[133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133, 133];
const LV_53_0_0: &[i32] = &[7, 7, 7, 7, 7, 7, 7, 7, 7];
