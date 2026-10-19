// 超解变 4G：保留第三下斧命中与瓶伤补丁，瓶爆计时／位置由 phial.rs 接管。
use crate::{config::SaedConfig, patch::WordPatch};
use alloc::vec::Vec;

/// (main 偏移，原版值，金手指值)。
const WORD_PATCHES: &[WordPatch] = &[
    // 超解增加第三次命中（3 hit）。
    // hitdata.rs 在内存中追加 0x50；原来的 0x4F 动作不再改用 0x1A。
    (0x011DADD4, 0xE5940000, 0xE1A00004),
    (0x011DADD8, 0xE59011CC, 0xE3A01000),
    (0x011DADDC, 0xE1A00004, 0xE3A02050),
    (0x011DADE0, 0xE12FFF31, 0xE3A03006),
    (0x011DADE4, 0xE3500000, 0xEBC37115),
    (0x011DADE8, 0x0A000006, 0xE320F000),
    (0x011DB18C, 0x42D80000, 0x42DA0000),
    (0x011DBDA0, 0xE5940000, 0xE1A00004),
    (0x011DBDA4, 0xE59011CC, 0xE3A01001),
    (0x011DBDA8, 0xE1A00004, 0xE3A02023),
    (0x011DBDAC, 0xE12FFF31, 0xE3A03006),
    (0x011DBDB0, 0xE3500000, 0xEBC36D22),
    (0x011DBDB4, 0x0A00000C, 0xE320F000),
    (0x011DBE50, 0x42D80000, 0x42DA0000),
    // 超解瓶伤：有瓶爆参数时，s18 固定为 1.35，不再按剩余瓶数增加。
    (0x0048906C, 0xEEB79A00, 0xED9F9A01),
    (0x00489070, 0xEE010A10, 0xEEB71A00),
    (0x00489074, 0xEEB81A41, 0xEA000000),
    (0x00489078, 0xEE019A00, 0x3FACCCCC),
    // 超解有瓶时统一使用指定特效档位（当前 r9=2，供用户核对特效）。
    // 保留真实瓶数与瓶爆次数；零瓶仍走原版返回，不创建瓶爆 shell。
    // INI 的 effect_gear 只修改第一条的立即数，选档补丁随 saed.enabled 安装。
    (0x011DE830, 0xE3A09004, 0xE3A09002), // mov r9, #2
    (0x011DE838, 0xE350000A, 0xE3500000), // cmp r0, #10 -> cmp r0, #0
    (0x011DE83C, 0x0A000007, 0x1A000007), // beq -> bne 0x011DE860
];

/// 超解总开关同时控制第三 hit、瓶伤和特效选档，共 21 处直接改值。
pub fn collect(config: &SaedConfig, words: &mut Vec<WordPatch>) {
    if !config.enabled {
        return;
    }
    // 1.35 沿用原金手指的精确字节 0x3FACCCCC，不重新作 f32 舍入。
    words.extend_from_slice(&WORD_PATCHES[..18]);
    let (site, original, _) = WORD_PATCHES[18];
    words.push((site, original, 0xE3A09000 | config.effect_gear as u32));
    words.extend_from_slice(&WORD_PATCHES[19..]);
}
