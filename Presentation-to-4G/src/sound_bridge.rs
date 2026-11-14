//! 盾斧瓶爆的公共血声提交桥，包含盾突、一二解、大解和超解。
use core::arch::{global_asm, naked_asm};
use core::ptr;
use crate::patch::{self, BranchPatch};

static mut BLOOD_CONTINUATION: u32 = 0;
global_asm!(".hidden {slot}", slot = sym BLOOD_CONTINUATION);

/// 从001808CC的两条加载进入：r0是PC相对偏移，不能直接当声音管理器。
/// continuation=001808D4，恰好是原LDR使用的PC；借r4重放两条加载后提交血声。
/// 原001808B8会跳到D8，必须保留D8的MOV，不可将它覆盖成8字节Hook的地址字。
#[unsafe(naked)]
unsafe extern "C" fn blood_bridge() {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "ldr r4, 2f",
        "3:",
        "add r4, pc, r4",
        "ldr r4, [r4]",
        "ldr r0, [r4, r0]",
        "ldr r0, [r0]",
        "sub sp, sp, #8",
        "str r5, [sp]",
        "bl {submit}",
        "add sp, sp, #8",
        "pop {{r4, lr}}",
        "push {{r12, lr}}",
        "ldr r12, 4f",
        "5:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        // 跳过未改写的原BL；落到原MOV r0,r8，与无血声路径汇合。
        "add r12, r12, #4",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "2:",
        ".word {slot} - (3b + 8)",
        "4:",
        ".word {slot} - (5b + 8)",
        submit = sym crate::sound::submit_hit_blood,
        slot = sym BLOOD_CONTINUATION,
    );
}

pub fn install(base: u32) {
    let branch = BranchPatch {
        offset: 0x001808CC,
        original: 0xE79F0000, // LDR r0,[pc,r0]；覆盖区内无其它分支落点
        cheat_call: 0xE79F0000,
        second: 0xE5900000, // LDR r0,[r0]
        target: blood_bridge as *const () as u32,
        continuation: ptr::addr_of_mut!(BLOOD_CONTINUATION),
        call_original: false,
    };
    unsafe { patch::install("cb4g-aed-blood", base, &[], &[branch]) };
}
