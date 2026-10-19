// patches.rs
// 派生变 4G：原金手指的直接改值与 GP 分支，不写 main 的代码洞。

use crate::{
    config::MovesetConfig,
    patch::{BranchPatch, WordPatch},
};
use alloc::vec::Vec;
use cb_game::player::{ChargeBladePlayer, Equipment};
use core::arch::{global_asm, naked_asm};
use core::ptr;

/// (main 偏移，原版值，金手指值)。同时接受已经写入的同一金手指值。
const YELLOW_SHIELD_PATCHES: &[WordPatch] = &[
    // 上黄盾变正常大解：公会、空战、炼金。
    // 炼金只在 GP 后或蓄力斩后正常大解，其它派生仍上黄盾。
    (0x011DA634, 0xE3550002, 0xE3550005),
    (0x011DA5EC, 0xE3550002, 0xE3550005),
    (0x011DA610, 0xE3550002, 0xE3550005),
    (0x011DA5C0, 0xE3550002, 0xE3550005),
    (0x011DA43C, 0xE3550002, 0xE3550005),
    (0x011DA4B8, 0x00000040, 0x000001A8),
    (0x011DA4C0, 0x00000040, 0x000001A8),
];

// 红盾、蓝盾补正修复为 1.2；黄盾仍为 1.0。
const SHIELD_MULTIPLIER_PATCHES: &[WordPatch] = &[
    (0x011DF8B0, 0xE3A01003, 0xE3A01002),
    (0x011DF8D0, 0x13A01001, 0xE3A01002),
];

// 大解瓶爆震动的原值与默认值；强度在启动配置中编码。
const AED_SHAKE_PATCH: WordPatch = (0x0048859C, 0xE3A0100F, 0xE3A01014);

const GP_SITE: u32 = 0x011DDE74;
const GP_ORIGINAL: u32 = 0xE3800020;
const GP_CHEAT_CALL: u32 = 0xEB085037;
const GP_SECOND: u32 = 0xE5810000; // str r0, [r1]

// Hit_func 已成功读出且确认 Active 的派生记录；只改本次栈上的记录。
const J_GUARD_SITE: u32 = 0x002EA87C;
const J_GUARD_ORIGINAL: u32 = 0xE3A00000; // mov r0, #0
const J_GUARD_SECOND: u32 = 0xE30013E7; // movw r1, #999

/// 实际 main 基址在加载后才确定；返回地址保存在 NRO 的可写数据段。
static mut MAIN_CONTINUATION: u32 = 0;
static mut J_GUARD_CONTINUATION: u32 = 0;
// 两个派生共用一个入口；bit0=J 防成功，bit1=武士盾突，在启动时固定。
static mut AED_ROUTE_FLAGS: u32 = 0;

// 让汇编中的 PC 相对引用在链接时解析成本 NRO 内的相对距离。
global_asm!(".hidden {slot}", slot = sym MAIN_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym J_GUARD_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym AED_ROUTE_FLAGS);

/// 从 main 的函数中间进入；r4 是玩家，r0 是已读取的标志，r1 是写回地址。
/// 公会、空战、炼金盾斧 GP 防御后后退；狩技和其它风格保持原行为。
/// 此处只使用汇编，避免普通 Rust 函数破坏原金手指保留的寄存器／浮点寄存器。
#[unsafe(naked)]
unsafe extern "C" fn gp_bridge() {
    naked_asm!(
        ".arm",
        "push {{r4, r5, lr}}",
        "movw r5, #{equipment}",
        "ldr r5, [r4, r5]",
        "ldrb r5, [r5, #{style}]",
        "cmp r5, #1",
        "beq 2f",
        "cmp r5, #3",
        "beq 2f",
        "cmp r5, #5",
        "beq 2f",
        "movw r5, #{action_id}",
        "ldrb r5, [r4, r5]",
        "cmp r5, #0x54",
        "beq 2f",
        "orr r0, r0, #0",
        "b 3f",
        "2:",
        "orr r0, r0, #0x20",
        "3:",
        "pop {{r4, r5, lr}}",
        // 绝对跳转覆盖了两条指令，先补回第二条写回，再跳过这 8 字节。
        "str r0, [r1]",
        // 临时使用 r12 读取返回地址；退出后 r12、lr、sp 都保持进入时的值。
        "push {{r12, lr}}",
        "ldr r12, 4f",
        "5:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "4:",
        ".word {slot} - (5b + 8)",
        slot = sym MAIN_CONTINUATION,
        equipment = const ChargeBladePlayer::EQUIPMENT_OFFSET,
        style = const Equipment::STYLE_OFFSET,
        action_id = const ChargeBladePlayer::ACTION_ID_OFFSET,
    );
}

/// r4=玩家，r5=本次 32 字节命令，r8=表内记录序号。
/// 只处理武士盾斧：J 防成功 0x81A 第 3 条、盾突 0x7FA 第 1 条，分别由启动标志控制。
/// 保留各自 Start/End/Pre；只改当前栈命令，不替换共享资源或分配新对象。
#[unsafe(naked)]
unsafe extern "C" fn j_guard_bridge() {
    naked_asm!(
        ".arm",
        "push {{r2, r3}}",
        "mrs r2, apsr",
        "cmp r8, #3",
        "beq 3f",
        "cmp r8, #1",
        "bne 2f",
        "movw r1, #0x07FA",
        "mov r3, #2",
        "b 4f",
        "3:",
        "movw r1, #0x081A",
        "mov r3, #1",
        "4:",
        "movw r0, #{command_table}",
        "ldr r0, [r4, r0]",
        "cmp r0, r1",
        "bne 2f",
        "ldr r0, 5f",
        "6:",
        "add r0, pc, r0",
        "ldr r0, [r0]",
        "and r0, r0, r3",
        "cmp r0, #0",
        "beq 2f",
        "movw r0, #{equipment}",
        "ldr r0, [r4, r0]",
        "ldrb r1, [r0, #{weapon}]",
        "cmp r1, #14",
        "bne 2f",
        "ldrb r0, [r0, #{style}]",
        "cmp r0, #3",
        "bne 2f",
        // act_st=0x10、act_no=0x44；与修改后的 071a 完整命令字段一致。
        "movw r0, #0x0010",
        "movt r0, #0x0044",
        "str r0, [r5]",
        // act_param=0、cmd_type=8，cmd_param0=0x1B 为 X+A 对应输入编号。
        "mov r0, #0x00080000",
        "str r0, [r5, #4]",
        "mov r0, #0x1B",
        "str r0, [r5, #8]",
        "mov r0, #0",
        "str r0, [r5, #12]",
        "str r0, [r5, #16]",
        "str r0, [r5, #20]",
        "2:",
        "msr APSR_nzcvq, r2",
        "pop {{r2, r3}}",
        // 补回覆盖的两条指令；保留进入时的 NZCV、浮点寄存器和全部其它寄存器。
        "mov r0, #0",
        "movw r1, #999",
        "push {{r12, lr}}",
        "ldr r12, 7f",
        "8:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "7:",
        ".word {slot} - (8b + 8)",
        "5:",
        ".word {flags} - (6b + 8)",
        slot = sym J_GUARD_CONTINUATION,
        flags = sym AED_ROUTE_FLAGS,
        command_table = const ChargeBladePlayer::COMMAND_TABLE_OFFSET,
        equipment = const ChargeBladePlayer::EQUIPMENT_OFFSET,
        weapon = const Equipment::WEAPON_OFFSET,
        style = const Equipment::STYLE_OFFSET,
    );
}

/// 按启动配置收集补丁；两个动作派生用启动时固定的位标志共用一处桥接。
pub unsafe fn collect(
    config: &MovesetConfig,
    words: &mut Vec<WordPatch>,
    branches: &mut Vec<BranchPatch>,
) {
    if config.yellow_shield_aed {
        words.extend_from_slice(YELLOW_SHIELD_PATCHES);
    }
    if config.shield_multiplier {
        words.extend_from_slice(SHIELD_MULTIPLIER_PATCHES);
    }
    if config.aed_camera_shake {
        let (site, original, _) = AED_SHAKE_PATCH;
        words.push((
            site,
            original,
            0xE3A01000 | config.aed_shake_strength as u32,
        ));
    }
    if config.gp_recoil {
        branches.push(BranchPatch {
            offset: GP_SITE,
            original: GP_ORIGINAL,
            cheat_call: GP_CHEAT_CALL,
            second: GP_SECOND,
            target: gp_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(MAIN_CONTINUATION),
        });
    }
    unsafe {
        ptr::write_volatile(
            ptr::addr_of_mut!(AED_ROUTE_FLAGS),
            (config.j_guard_aed as u32) | ((config.bushi_shield_thrust_aed as u32) << 1),
        );
    }
    if config.j_guard_aed || config.bushi_shield_thrust_aed {
        branches.push(BranchPatch {
            offset: J_GUARD_SITE,
            original: J_GUARD_ORIGINAL,
            cheat_call: J_GUARD_ORIGINAL,
            second: J_GUARD_SECOND,
            target: j_guard_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(J_GUARD_CONTINUATION),
        });
    }
}
