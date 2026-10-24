// patches.rs
// 派生变 4G：共用补正／震动与按风格选择的 GP 分支，不写 main 的代码洞。

use crate::{
    config::Config,
    config_file,
    patch::{BranchPatch, WordPatch},
    style::{self, Feature},
};
use alloc::vec::Vec;
use cb_game::{memory, player::ChargeBladePlayer};
use core::arch::{global_asm, naked_asm};
use core::ptr;

// 红盾、蓝盾补正修复为 1.2；黄盾仍为 1.0。
// 共用盾状态入口不按风格分流，因此新增风格同样使用这个补正。
const SHIELD_MULTIPLIER_PATCHES: &[WordPatch] = &[
    (0x011DF8B0, 0xE3A01003, 0xE3A01002),
    (0x011DF8D0, 0x13A01001, 0xE3A01002),
];

// 大解瓶爆震动只提供启停：开启固定为 20，关闭保留 GU 的 15。
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

// 让汇编中的 PC 相对引用在链接时解析成本 NRO 内的相对距离。
global_asm!(".hidden {slot}", slot = sym MAIN_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym J_GUARD_CONTINUATION);

/// 从 main 的函数中间进入；r4 是玩家，r0 是已读取的标志，r1 是写回地址。
/// 当前风格启用后退时不再追加 0x20；狩技 0x54 仍追加该位。
/// 保留原 r0 已有的所有位；这里只决定是否执行原来的 ORR，不额外清位。
#[inline(never)]
unsafe extern "C" fn gp_recoil_enabled(actor: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    u32::from(unsafe {
        player.action_id() != 0x54 && style::enabled_for(player, Feature::GpRecoil)
    })
}

/// 调用风格查询时保存 NZCV、全部调用者保存寄存器及 d0-d15，栈保持八字节对齐。
#[unsafe(naked)]
unsafe extern "C" fn gp_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "mrs r12, cpsr",
        "push {{r12, lr}}",
        "vpush {{d0-d15}}",
        "mov r0, r4",
        "bl {enabled}",
        "cmp r0, #0",
        "bne 2f",
        // 原 r0 位于 128 字节 VFP 保存区和 8 字节标志保存区之后。
        "ldr r0, [sp, #136]",
        "orr r0, r0, #0x20",
        "str r0, [sp, #136]",
        "2:",
        "vpop {{d0-d15}}",
        "pop {{r12, lr}}",
        "msr cpsr_f, r12",
        "pop {{r0-r3, r12, lr}}",
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
        enabled = sym gp_recoil_enabled,
    );
}

/// 只改当前32字节临时命令的前24字节，原Pre/Start/End始终保留。
/// 0x44/0x4B本身是黄盾前置；后续输入钩子再按yellow_shield_aed决定是否转AA/AB。
#[inline(never)]
unsafe extern "C" fn rewrite_yellow_route(actor: u32, command: u32, index: u32) {
    let player = ChargeBladePlayer::from_address(actor);
    if unsafe { player.equipment().weapon_type() } != 14 {
        return;
    }
    let config = config_file::get();
    if !config.enabled {
        return;
    }
    let chain = unsafe { player.command_chain() };
    let target = match chain {
        0x81A
            if index == 3
                && config.moveset.j_guard_aed
                && unsafe { player.equipment().style() } == 3 =>
        {
            0x44
        }
        0x7FA
            if index == 1 && unsafe { style::enabled_for(player, Feature::ShieldThrustYellow) } =>
        {
            0x44
        }
        0x802 | 0x803 if unsafe { style::enabled_for(player, Feature::GuardRecoilYellow) } => {
            let original_count = unsafe { player.command_resource().weapon_table(chain).count() };
            // 新增命令只占副本的末条；原小退/中退派生全部保留。
            if index != original_count
                || unsafe { player.command_table().count() } != original_count + 1
            {
                return;
            }
            0x4B
        }
        _ => return,
    };
    unsafe {
        memory::write_u32(command, 0x10 | (target << 16));
        memory::write_u32(command + 4, 8 << 16);
        memory::write_u32(command + 8, 0x1B); // X+A
        for offset in [12, 16, 20] {
            memory::write_u32(command + offset, 0);
        }
    }
}

/// r4=玩家，r5=本次32字节命令，r8=表内序号；仅四条候选链进入Rust查询。
/// 保存NZCV、调用者寄存器和d0-d15；普通命令不进入查询，不分配对象。
#[unsafe(naked)]
unsafe extern "C" fn j_guard_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "mrs r12, cpsr",
        "push {{r12, lr}}",
        "movw r0, #{command_table}",
        "ldr r0, [r4, r0]",
        "movw r1, #0x07FA",
        "cmp r0, r1",
        "beq 3f",
        "movw r1, #0x081A",
        "cmp r0, r1",
        "beq 3f",
        "movw r1, #0x0802",
        "cmp r0, r1",
        "beq 3f",
        "movw r1, #0x0803",
        "cmp r0, r1",
        "bne 2f",
        "3:",
        "vpush {{d0-d15}}",
        "mov r0, r4",
        "mov r1, r5",
        "mov r2, r8",
        "bl {rewrite}",
        "vpop {{d0-d15}}",
        "2:",
        "pop {{r12, lr}}",
        "msr cpsr_f, r12",
        "pop {{r0-r3, r12, lr}}",
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
        slot = sym J_GUARD_CONTINUATION,
        rewrite = sym rewrite_yellow_route,
        command_table = const ChargeBladePlayer::COMMAND_TABLE_OFFSET,
    );
}

/// 按启动配置收集补丁；三种黄盾入口共用一处桥接，盾突/后退只看各自风格集合。
pub unsafe fn collect(
    config: &Config,
    words: &mut Vec<WordPatch>,
    branches: &mut Vec<BranchPatch>,
) {
    if config.moveset.shield_multiplier {
        words.extend_from_slice(SHIELD_MULTIPLIER_PATCHES);
    }
    if config.moveset.aed_camera_shake {
        words.push(AED_SHAKE_PATCH);
    }
    if !config.styles.gp_recoil.is_empty() {
        branches.push(BranchPatch {
            offset: GP_SITE,
            original: GP_ORIGINAL,
            cheat_call: GP_CHEAT_CALL,
            second: GP_SECOND,
            target: gp_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(MAIN_CONTINUATION),
        });
    }
    if config.moveset.j_guard_aed
        || !config.styles.shield_thrust_yellow.is_empty()
        || !config.styles.guard_recoil_yellow.is_empty()
    {
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
