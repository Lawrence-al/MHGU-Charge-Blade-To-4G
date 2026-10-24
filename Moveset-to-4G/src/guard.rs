//! 4G 白盾／红盾防性：技能修正仍由 GU 处理，GP 独立减威力 10，红盾用 40/50 阈值。
use crate::{
    patch::BranchPatch,
    style::{self, Feature},
};
use alloc::vec::Vec;
use cb_game::player::ChargeBladePlayer;
use core::arch::{global_asm, naked_asm};
use core::ptr;

// 盾斧 +0x878 虚函数；调用者已经完成防御性能技能修正。
const POWER_SITE: u32 = 0x011DFB0C;
const POWER_ORIGINAL: u32 = 0xE92D4830;
const POWER_SECOND: u32 = 0xE1A05000;
// 共用分类中 0/14/15 武器的 15/40 分支；仅红盾盾斧改走 40/50。
const RANK_SITE: u32 = 0x002F0800;
const RANK_ORIGINAL: u32 = 0xE3A08002;
const RANK_SECOND: u32 = 0xE3500027;
static mut POWER_CONTINUATION: u32 = 0;
static mut RANK_CONTINUATION: u32 = 0;
global_asm!(".hidden {slot}", slot = sym RANK_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym POWER_CONTINUATION);

#[inline(never)]
unsafe extern "C" fn guard_enabled(address: u32) -> u32 {
    u32::from(unsafe {
        style::enabled_for(
            ChargeBladePlayer::from_address(address),
            Feature::GuardPerformance,
        )
    })
}

/// 未启用当前风格时执行完整 GU 原函数：重放前两条后进入原函数 +8。
/// GU 原函数会调用 +0x85C 的盾状态查询；不以字段推测替代这个虚调用。
#[unsafe(naked)]
unsafe extern "C" fn original_guard_power(_address: u32, _power: *mut u32) {
    naked_asm!(
        ".arm",
        "push {{r4, r5, r11, lr}}",
        "mov r5, r0",
        "push {{r12, lr}}",
        "ldr r12, 2f",
        "3:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "2:",
        ".word {slot} - (3b + 8)",
        slot = sym POWER_CONTINUATION,
    );
}

/// 阶段值可能留到下个动作，所以还要核对当前动画和通用防御有效位。
/// 动画编号来自原版共享 GP 函数的五个调用者及四处内联 GP 窗口。
/// 不读取 +0x3340 的 0x20：GP 后退补丁会故意去掉该特殊反应位。
unsafe fn guard_point(player: ChargeBladePlayer) -> bool {
    unsafe {
        player.gp_stage() == 1
            && player.guard_flags() & 0x10 != 0
            && matches!(player.animation_id(),
                5014..=5016 | 5105..=5107 | 5109 | 5112..=5113 |
                5131 | 5146 | 7051 | 7151)
    }
}

/// 替代整个盾斧修正回调；其返回值原本就不被共用威力读取器使用。
/// 黄盾／蓝盾在 4G 不存在，保留 GU 的原减值，不叠加新的 GP 修正。
unsafe extern "C" fn guard_power(address: u32, power: *mut u32) {
    if unsafe { guard_enabled(address) } == 0 {
        unsafe { original_guard_power(address, power) };
        return;
    }
    let player = ChargeBladePlayer::from_address(address);
    let reduce = match unsafe { player.shield_state() } {
        0 | 2 => unsafe { guard_point(player) },
        1 => true,
        3 => unsafe { player.shield_flags() & 0x20 != 0 },
        _ => false,
    };
    if reduce {
        unsafe { *power = (*power).saturating_sub(10).max(1) };
    }
}

#[inline(never)]
unsafe extern "C" fn guard_rank_enabled(address: u32, weapon: u32) -> u32 {
    u32::from(
        weapon == 14
            && unsafe { ChargeBladePlayer::from_address(address).shield_state() == 2 }
            && unsafe { guard_enabled(address) != 0 },
    )
}

/// r0=技能／GP 修正后的威力，r1=武器类型，r4=玩家，r8=硬直档位。
/// 非红盾补回原两条指令；红盾直接跳到分类结束，后续受击／耐力流程不变。
#[unsafe(naked)]
unsafe extern "C" fn guard_rank_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "vpush {{d0-d15}}",
        "mov r0, r4",
        "bl {enabled}",
        "cmp r0, #0",
        "vpop {{d0-d15}}",
        "pop {{r0-r3, r12, lr}}",
        "push {{r2, r3, r12, lr}}",
        "mov r3, #0",
        "beq 2f",
        // main+002F0828 是原分类汇合点，相对普通返回地址 +0x20。
        "mov r3, #0x20",
        "mov r8, #2",
        "cmp r0, #49",
        "bhi 3f",
        "mov r8, #0",
        "cmp r0, #39",
        "movhi r8, #1",
        "b 3f",
        "2:",
        "mov r8, #2",
        "cmp r0, #39",
        "3:",
        "ldr r12, 4f",
        "5:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "add r12, r12, r3",
        "str r12, [sp, #12]",
        "pop {{r2, r3, r12, pc}}",
        ".balign 4",
        "4:",
        ".word {slot} - (5b + 8)",
        slot = sym RANK_CONTINUATION,
        enabled = sym guard_rank_enabled,
    );
}

pub fn collect(branches: &mut Vec<BranchPatch>) {
    branches.push(BranchPatch {
        offset: POWER_SITE,
        original: POWER_ORIGINAL,
        cheat_call: POWER_ORIGINAL,
        second: POWER_SECOND,
        target: guard_power as *const () as u32,
        continuation: ptr::addr_of_mut!(POWER_CONTINUATION),
    });
    branches.push(BranchPatch {
        offset: RANK_SITE,
        original: RANK_ORIGINAL,
        cheat_call: RANK_ORIGINAL,
        second: RANK_SECOND,
        target: guard_rank_bridge as *const () as u32,
        continuation: ptr::addr_of_mut!(RANK_CONTINUATION),
    });
}
