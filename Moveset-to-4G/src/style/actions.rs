//! 所有盾斧共用新增动作 0xAA/0xAB；按风格开关将 0x44/0x4B 命令导向它们。
use crate::patch::BranchPatch;
use alloc::vec::Vec;
use cb_game::{memory, player::ChargeBladePlayer};
use core::{
    arch::{global_asm, naked_asm},
    ptr,
};

pub const AED_PRELUDE: u16 = 0xAA;
pub const AED_PRELUDE_AFTER_CHARGE: u16 = 0xAB;
const PRELUDE_VARIANT: u32 = 16;
const PRELUDE_AFTER_CHARGE_VARIANT: u32 = 17;

// 原分发表只包含 0..0xA9；桥在其边界检查处接入两个新动作。
const DISPATCH_SITE: u32 = 0x011D5C64;
const DISPATCH_ORIGINAL: u32 = 0xE35000A9; // cmp r0, #0xA9
const DISPATCH_SECOND: u32 = 0x8A00030B; // bhi main+0x011D689C
const DISPATCH_DEFAULT_OFFSET: u32 = 0x011D689C;
const PRELUDE_UPDATE_OFFSET: u32 = 0x011DA204;
const CHAIN_UPDATE_OFFSET: u32 = 0x002E9C50;

static mut DISPATCH_CONTINUATION: u32 = 0;
static mut DISPATCH_DEFAULT: u32 = 0;
global_asm!(".hidden {slot}", slot = sym DISPATCH_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym DISPATCH_DEFAULT);

/// 逻辑变体 16/17 复用原变体 2/4 的动画与表现。
/// state3 不再查动画表；传 0 使用普通大解/超解分流，避开黄盾 0x4E/0x4F。
#[inline]
fn resource_variant(logical: u32, state: u32) -> u32 {
    if state == 3 {
        0
    } else if logical == PRELUDE_VARIANT {
        2
    } else {
        4
    }
}

/// 对齐 Moveset 黄盾修复后的链号。+6 是前置开始时缓存的有瓶标志。
/// state0 建输入窗口，state2→3 时只续链，保留原来已经缓存的按键。
#[inline]
fn chain_transition(before: u32, after: u32, shield: u32, has_phial: bool) -> Option<(u32, u32)> {
    let resume = match (before, after) {
        (0, 2) => 0,
        (2, 3) => 1,
        _ => return None,
    };
    let powered = matches!(shield, 1 | 2 | 3);
    let base = if powered { 0x7F0 } else { 0x7EC };
    Some((base + u32::from(!has_phial) + resume * 2, resume))
}

/// 调用原入口保留 Presentation 在 0x011DA204 安装的音效/盾牌特效钩子。
/// 原 16 项表仍只收到 2/4；新增 16/17 仅作为本插件的分发参数。
#[inline(never)]
unsafe extern "C" fn dispatch_new_action(actor: u32, logical: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    let base = crate::game_base();
    let before = unsafe { player.action_state() };
    let shield = unsafe { player.shield_state() };
    let update: unsafe extern "C" fn(u32, u32) -> u32 =
        unsafe { core::mem::transmute(base + PRELUDE_UPDATE_OFFSET) };
    let result = unsafe { update(actor, resource_variant(logical, before)) };
    if let Some((chain, resume)) =
        chain_transition(before, unsafe { player.action_state() }, shield, unsafe {
            player.has_phial()
        })
    {
        // 其它插件已经选中同一条链时，不重复初始化输入缓存。
        if unsafe { player.command_chain() } != chain {
            let set_chain: unsafe extern "C" fn(u32, u32, u32, u32) =
                unsafe { core::mem::transmute(base + CHAIN_UPDATE_OFFSET) };
            unsafe { set_chain(actor, chain, 0, resume) };
        }
    }
    result
}

/// 此处已经执行原分发函数的 push {r4,lr}; sub sp,#8，r4=玩家、r0=动作号。
/// 新动作直接按原函数的栈帧返回；其它动作重放原 cmp/bhi，全部寄存器不变。
#[unsafe(naked)]
unsafe extern "C" fn dispatch_bridge() {
    naked_asm!(
        ".arm",
        "cmp r0, #{new_action}",
        "beq 2f",
        "cmp r0, #{new_charge_action}",
        "bne 3f",
        "mov r1, #{charge_variant}",
        "b 5f",
        "2:",
        "mov r1, #{prelude_variant}",
        "5:",
        "mov r0, r4",
        "bl {dispatch}",
        "add sp, sp, #8",
        "pop {{r4, pc}}",
        "3:",
        "cmp r0, #0xA9",
        "bhi 4f",
        "push {{r12, lr}}",
        "ldr r12, 6f",
        "7:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        "4:",
        "push {{r12, lr}}",
        "ldr r12, 8f",
        "9:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "6:",
        ".word {continuation} - (7b + 8)",
        "8:",
        ".word {default} - (9b + 8)",
        new_action = const AED_PRELUDE,
        new_charge_action = const AED_PRELUDE_AFTER_CHARGE,
        prelude_variant = const PRELUDE_VARIANT,
        charge_variant = const PRELUDE_AFTER_CHARGE_VARIANT,
        dispatch = sym dispatch_new_action,
        continuation = sym DISPATCH_CONTINUATION,
        default = sym DISPATCH_DEFAULT,
    );
}

/// 这里只操作 Hit_func 栈上的 32 字节临时命令，不改共享 cPlCmd 或 ARC。
/// 输入判断使用的是 cmd_type 与 cmd_param；其结果不因仅改动作 ID 而变化。
#[inline(never)]
unsafe fn redirect_command(actor: u32, command: u32) {
    if unsafe { memory::read_u16(command) } != 0x10 {
        return;
    }
    let id = unsafe { memory::read_u16(command + 2) };
    let replacement = match id {
        0x44 => AED_PRELUDE,
        0x4B => AED_PRELUDE_AFTER_CHARGE,
        _ => return,
    };
    if unsafe {
        super::enabled_for(
            ChargeBladePlayer::from_address(actor),
            super::Feature::YellowShieldAed,
        )
    } {
        unsafe { ptr::write_unaligned((command + 2) as *mut u16, replacement) };
    }
}

// 唯一调用者是 Hit_func；位于读出记录与帧窗后、提交目标动作前。
// 不依赖本帧输入是否成功：已缓存输入在后续帧仍会使用本次临时记录。
crate::cb_hook! { 0x002ECEFC,
fn hook_command_input(player: u32, command: u32) -> u32, |original| {
    let result = unsafe { original(player, command) };
    unsafe { redirect_command(player, command) };
    result
}
}

pub fn collect(branches: &mut Vec<BranchPatch>) {
    unsafe {
        ptr::write_volatile(
            &raw mut DISPATCH_DEFAULT,
            crate::game_base() + DISPATCH_DEFAULT_OFFSET,
        );
    }
    branches.push(BranchPatch {
        offset: DISPATCH_SITE,
        original: DISPATCH_ORIGINAL,
        cheat_call: DISPATCH_ORIGINAL,
        second: DISPATCH_SECOND,
        target: dispatch_bridge as *const () as u32,
        continuation: &raw mut DISPATCH_CONTINUATION,
    });
}

pub fn enable(base: u32) {
    forge::install_hook!(base, hook_command_input);
}
