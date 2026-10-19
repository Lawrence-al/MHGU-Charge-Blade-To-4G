// main 补丁插件共用的安装与 ARM32 缓存刷新；不分配代码洞。
extern crate alloc;

use alloc::vec::Vec;
use core::arch::naked_asm;
use core::{mem, ptr};
use forge::log::prelude::error;
use forge::patch::Patch;

/// (main 偏移，原版值，金手指值)。
pub type WordPatch = (u32, u32, u32);

/// 一个 8 字节绝对跳转；桥接函数负责补回 second，并返回 offset + 8。
pub struct BranchPatch {
    pub offset: u32,
    pub original: u32,
    pub cheat_call: u32,
    pub second: u32,
    pub target: u32,
    pub continuation: *mut u32,
}

const ARM_ABSOLUTE_JUMP: u32 = 0xE51FF004; // ldr pc, [pc, #-4]

/// libnx AArch32 svcFlushProcessDataCache：
/// r0=当前进程句柄，r1=长度低位，r2:r3=地址，r4=长度高位。
/// Rust 入口 r0=地址、r1=长度；保留 callee-saved r4。
#[unsafe(naked)]
unsafe extern "C" fn flush_code(_address: u32, _size: u32) -> u32 {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "mov r2, r0",
        "movw r0, #0x8001",
        "movt r0, #0xFFFF",
        "mov r3, #0",
        "mov r4, #0",
        "svc #0x5F",
        "dsb sy",
        "isb",
        "pop {{r4, pc}}",
    );
}

/// 先核对整组原值，再安装；缓存刷新失败时恢复本组已经写入的全部字节。
pub unsafe fn install(
    name: &str,
    game_base: u32,
    words: &[WordPatch],
    branches: &[BranchPatch],
) -> bool {
    for &(offset, original, replacement) in words {
        let current = unsafe { ptr::read_volatile((game_base + offset) as *const u32) };
        if current != original && current != replacement {
            error!(
                "{name}: main+0x{offset:08X}=0x{current:08X}; expected 0x{original:08X} or 0x{replacement:08X}, patches skipped"
            );
            return false;
        }
    }
    for branch in branches {
        let site = game_base + branch.offset;
        let first = unsafe { ptr::read_volatile(site as *const u32) };
        let second = unsafe { ptr::read_volatile((site + 4) as *const u32) };
        if (first != branch.original && first != branch.cheat_call) || second != branch.second {
            error!(
                "{name}: main+0x{:08X}=0x{first:08X}, next=0x{second:08X}; expected 0x{:08X}/0x{:08X} and 0x{:08X}, patches skipped",
                branch.offset, branch.original, branch.cheat_call, branch.second
            );
            return false;
        }
    }

    let mut patches = Vec::with_capacity(words.len() + branches.len());
    for &(offset, _, replacement) in words {
        patches.push(Patch::new(
            game_base + offset,
            &replacement.to_le_bytes(),
            false,
        ));
    }
    for branch in branches {
        let site = game_base + branch.offset;
        unsafe { ptr::write_volatile(branch.continuation, site + 8) };
        let mut jump = [0u8; 8];
        jump[..4].copy_from_slice(&ARM_ABSOLUTE_JUMP.to_le_bytes());
        jump[4..].copy_from_slice(&branch.target.to_le_bytes());
        patches.push(Patch::new(site, &jump, false));
    }

    let failed = patches.iter_mut().find_map(|patch| {
        patch.enable();
        let result = unsafe { flush_code(patch.addr(), patch.len()) };
        (result != 0).then_some((patch.addr(), result))
    });
    if let Some((address, result)) = failed {
        for patch in patches.iter_mut().rev().filter(|patch| patch.enabled()) {
            patch.disable();
            unsafe { flush_code(patch.addr(), patch.len()) };
        }
        error!(
            "{name}: cache flush at 0x{address:08X} failed 0x{result:08X}; original bytes restored"
        );
        return false;
    }
    // main 仍跳到本 NRO，保持 Patch 存活到进程结束，避免 Drop 立即撤销。
    for patch in patches {
        mem::forget(patch);
    }
    error!(
        "{name}: {} word patches and {} NRO branches installed; main codecave untouched",
        words.len(),
        branches.len()
    );
    true
}
