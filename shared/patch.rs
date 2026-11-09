// main 入口／内部桥统一通过 Forge Hook 安装；不分配代码洞。
extern crate alloc;

use crate::runtime_hook;
use alloc::vec::Vec;
use core::arch::naked_asm;
use core::{mem, ptr};
use forge::log::prelude::error;
use forge::patch::Patch;

/// (main 偏移，原版值，兼容的旧金手指值)。只做完整签名核对，不写指令。
/// 等效行为由同组 BranchPatch 的 NRO 桥执行，包括原来放在 main 的浮点常量。
pub type WordPatch = (u32, u32, u32);

/// 标准 Forge Hook；内部桥负责补回被覆盖的指令，continuation 接收 offset + 8。
/// call_original 的入口替换则接收 Forge 提供的 original，不能自行续跳函数 +8。
pub struct BranchPatch {
    pub offset: u32,
    pub original: u32,
    pub cheat_call: u32,
    pub second: u32,
    pub target: u32,
    pub continuation: *mut u32,
    pub call_original: bool,
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

/// 先核对整组原值，再调用标准 Hook 接口。安装失败时恢复已经写入的入口。
/// 正常路径由 Forge 完成代码映射／缓存维护；重编译端也能登记内部函数范围。
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
        let legacy_second = words
            .iter()
            .any(|&(offset, _, legacy)| offset == branch.offset + 4 && second == legacy);
        if (first != branch.original && first != branch.cheat_call)
            || (second != branch.second && !legacy_second)
        {
            error!(
                "{name}: main+0x{:08X}=0x{first:08X}, next=0x{second:08X}; expected 0x{:08X}/0x{:08X} and 0x{:08X}, patches skipped",
                branch.offset, branch.original, branch.cheat_call, branch.second
            );
            return false;
        }
    }

    let mut installed: Vec<(u32, [u8; 8])> = Vec::with_capacity(branches.len());
    for branch in branches {
        let site = game_base + branch.offset;
        let mut before = [0u8; 8];
        before[..4]
            .copy_from_slice(&unsafe { ptr::read_volatile(site as *const u32) }.to_le_bytes());
        before[4..].copy_from_slice(
            &unsafe { ptr::read_volatile((site + 4) as *const u32) }.to_le_bytes(),
        );
        let original = if branch.call_original {
            branch.continuation.cast()
        } else {
            unsafe { ptr::write_volatile(branch.continuation, site + 8) };
            ptr::null_mut()
        };
        unsafe { runtime_hook::create(site, branch.target as *const _, original) };
        installed.push((site, before));
        let first = unsafe { ptr::read_volatile(site as *const u32) };
        let target = unsafe { ptr::read_volatile((site + 4) as *const u32) };
        if first != ARM_ABSOLUTE_JUMP || target != branch.target {
            // Hook API 没有销毁接口；仅在失败恢复路径用 Patch 放回原字节。
            // 入口／内部返回槽仍留在 NRO 中，已登记的解释范围可以继续执行原代码。
            for &(address, bytes) in installed.iter().rev() {
                let restore = Patch::new(address, &bytes, true);
                unsafe { flush_code(address, 8) };
                mem::forget(restore);
            }
            error!("{name}: Forge Hook at 0x{site:08X} failed; original bytes restored");
            return false;
        }
    }
    error!(
        "{name}: {} instruction checks and {} Forge hooks installed; main codecave untouched",
        words.len(),
        branches.len()
    );
    true
}
