//! 仅替换盾斧瓶附加、延迟瓶爆与 AED 粒子重复创建条件的三个原生调用点。
extern crate alloc;

use alloc::vec::Vec;
use core::{arch::{global_asm, naked_asm}, ptr};
use crate::patch::{self, BranchPatch};

static mut ATTACHMENT_CONTINUATION: u32 = 0;
static mut BLAST_CONTINUATION: u32 = 0;
static mut REPEAT_CONTINUATION: u32 = 0;

global_asm!(".hidden {slot}", slot = sym ATTACHMENT_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym BLAST_CONTINUATION);
global_asm!(".hidden {slot}", slot = sym REPEAT_CONTINUATION);

/// 原 BL 的三个参数已在 r0-r2；成功／失败节点均由原 STR 写入 slot5596。
#[unsafe(naked)]
unsafe extern "C" fn attachment_bridge() {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "bl {create}",
        "pop {{r4, lr}}",
        "str r0, [r5]",
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
        create = sym super::create_attachment,
        slot = sym ATTACHMENT_CONTINUATION,
    );
}

/// 原 CMP 是后续 BEQ 的输入；返回地址解析只用不更新 NZCV 的指令。
#[unsafe(naked)]
unsafe extern "C" fn blast_bridge() {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "bl {create}",
        "pop {{r4, lr}}",
        "cmp r0, #0",
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
        create = sym super::create_blast,
        slot = sym BLAST_CONTINUATION,
    );
}

/// 非 BL 入口：保存原本活跃的调用者寄存器和 s0-s15，再补回 LDRB/CMP。
/// helper 只改变比较使用的 state，不写 shell 状态或伤害参数。
#[unsafe(naked)]
unsafe extern "C" fn repeat_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "vpush {{s0-s15}}",
        "mov r0, r4",
        "ldrb r1, [r4, #5]",
        "bl {state}",
        // 64 字节 VFP 保存区后是原 r0 的保存槽。
        "str r0, [sp, #64]",
        "vpop {{s0-s15}}",
        "pop {{r0-r3, r12, lr}}",
        "cmp r0, #0",
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
        state = sym super::visual_state,
        slot = sym REPEAT_CONTINUATION,
    );
}

pub fn install(base: u32, attachment: bool, blast: bool, aed_repeat: bool) {
    let mut branches = Vec::new();
    if attachment {
        branches.push(BranchPatch {
            offset: 0x00488358,
            original: 0xEB0066BC, // BL 004A1E50
            cheat_call: 0xEB0066BC,
            second: 0xE5850000, // STR r0,[r5]
            target: attachment_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(ATTACHMENT_CONTINUATION),
        });
    }
    if blast {
        branches.push(BranchPatch {
            offset: 0x004886EC,
            original: 0xEB0065D7, // BL 004A1E50
            cheat_call: 0xEB0065D7,
            second: 0xE3500000, // CMP r0,#0；原 STR 留在返回点
            target: blast_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(BLAST_CONTINUATION),
        });
    }
    if aed_repeat {
        branches.push(BranchPatch {
            offset: 0x00488654,
            original: 0xE5D40005, // LDRB r0,[r4,#5]
            cheat_call: 0xE5D40005,
            second: 0xE3500000, // CMP r0,#0
            target: repeat_bridge as *const () as u32,
            continuation: ptr::addr_of_mut!(REPEAT_CONTINUATION),
        });
    }
    if !branches.is_empty() {
        unsafe { patch::install("cb4g-phial", base, &[], &branches) };
    }
}
