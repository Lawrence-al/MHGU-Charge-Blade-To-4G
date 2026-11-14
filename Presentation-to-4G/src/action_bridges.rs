//! 重编译解释器内调用会跳过函数入口钩子，改在原序言之后接回同一套事件处理。
extern crate alloc;

use alloc::vec::Vec;
use core::arch::{global_asm, naked_asm};
use core::ptr;
use cb_game::player::ChargeBladePlayer;
use crate::{action, phial, patch::{self, BranchPatch, WordPatch}};

type PlayerUpdate = unsafe extern "C" fn(u32, u32) -> u32;

static mut ACTION_CONTINUATION: u32 = 0;
static mut SAED_CONTINUATION: u32 = 0;
global_asm!(".hidden {slot}", slot = sym ACTION_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym SAED_CONTINUATION);

/// 原版入口和重编译内部桥共用，任务在原更新前执行，视觉登记在原更新后执行。
pub unsafe fn update_action(actor: u32, variant: u32, original: PlayerUpdate) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    let before = unsafe { player.action_state() };
    unsafe { action::on_phial_action_frame(player, variant, before) };
    let result = unsafe { original(actor, variant) };
    unsafe { phial::on_action_frame(player, variant, before) };
    result
}

/// 发瓶前缓存实际瓶数和阶段，原函数可能清空瓶数，返回值不能代表创建成功。
pub unsafe fn create_saed(actor: u32, aerial: u32, original: PlayerUpdate) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    let before = unsafe { player.action_state() };
    let had_phial = unsafe { player.phial_count() } != 0;
    let result = unsafe { original(actor, aerial) };
    unsafe { phial::on_saed_created(player, before, had_phial) };
    result
}

unsafe extern "C" fn interpreted_action_update(actor: u32, variant: u32) -> u32 {
    unsafe { update_action(actor, variant, original_action_update) }
}

unsafe extern "C" fn interpreted_saed_create(actor: u32, aerial: u32) -> u32 {
    unsafe { create_saed(actor, aerial, original_saed_create) }
}

/// 011DABC8的PUSH/MOV已执行，r0/r1仍是原参数。先撤回序言，再按普通函数调用。
#[unsafe(naked)]
unsafe extern "C" fn action_bridge() {
    naked_asm!(
        ".arm",
        "pop {{r4-r6, lr}}",
        "b {update}",
        update = sym interpreted_action_update,
    );
}

/// 011DE7E4的PUSH/MOV同样不改r0/r1，恢复r4-r10及真正的调用者LR。
#[unsafe(naked)]
unsafe extern "C" fn saed_bridge() {
    naked_asm!(
        ".arm",
        "pop {{r4-r10, lr}}",
        "b {create}",
        create = sym interpreted_saed_create,
    );
}

/// 重放原版前四条指令后续入+0x10，不再次经过+8的内部钩子。
#[unsafe(naked)]
unsafe extern "C" fn original_action_update(_actor: u32, _variant: u32) -> u32 {
    naked_asm!(
        ".arm",
        "push {{r4-r6, lr}}",
        "mov r4, r0",
        "mov r5, r1",
        "movw r0, #0x274A",
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
        slot = sym ACTION_CONTINUATION,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn original_saed_create(_actor: u32, _aerial: u32) -> u32 {
    naked_asm!(
        ".arm",
        "push {{r4-r10, lr}}",
        "mov r8, r1",
        "mov r5, r0",
        "movw r0, #0x332C",
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
        slot = sym SAED_CONTINUATION,
    );
}

pub fn install(base: u32, action_enabled: bool, saed_enabled: bool) {
    let mut words: Vec<WordPatch> = Vec::new();
    let mut branches = Vec::new();
    if action_enabled {
        words.extend_from_slice(&[
            (0x011DABC8, 0xE92D4070, 0xE92D4070),
            (0x011DABCC, 0xE1A04000, 0xE1A04000),
        ]);
        branches.push(BranchPatch {
            offset: 0x011DABD0,
            original: 0xE1A05001,
            cheat_call: 0xE1A05001,
            second: 0xE302074A,
            target: action_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(ACTION_CONTINUATION),
            call_original: false,
        });
    }
    if saed_enabled {
        words.extend_from_slice(&[
            (0x011DE7E4, 0xE92D47F0, 0xE92D47F0),
            (0x011DE7E8, 0xE1A08001, 0xE1A08001),
        ]);
        branches.push(BranchPatch {
            offset: 0x011DE7EC,
            original: 0xE1A05000,
            cheat_call: 0xE1A05000,
            second: 0xE303032C,
            target: saed_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(SAED_CONTINUATION),
            call_original: false,
        });
    }
    if !branches.is_empty() {
        unsafe { patch::install("cb4g-action-events", base, &words, &branches) };
    }
}
