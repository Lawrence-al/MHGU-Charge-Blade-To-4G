//! 游戏浮点参数／返回值使用 VFP 寄存器，Rust 目标使用 soft-float ABI。
//! 保留原转换指令及栈对齐；上层不直接用普通 extern C f32 调用游戏。
#[cfg(target_arch = "arm")]
use core::arch::naked_asm;

#[cfg(target_arch = "arm")]
#[unsafe(naked)]
pub unsafe extern "C" fn trig_call(_function: u32, _radians: u32) -> u32 {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "mov r12, r0",
        "vmov s0, r1",
        "blx r12",
        "vmov r0, s0",
        "pop {{r4, pc}}",
    );
}

#[cfg(target_arch = "arm")]
#[unsafe(naked)]
pub unsafe extern "C" fn float_call8(_function: u32, _arguments: *const [u32; 8]) -> u32 {
    naked_asm!(
        ".arm",
        "push {{r4, lr}}",
        "mov r12, r0",
        "mov r4, r1",
        "sub sp, sp, #16",
        "ldr r0, [r4, #16]",
        "str r0, [sp]",
        "ldr r0, [r4, #20]",
        "str r0, [sp, #4]",
        "ldr r0, [r4, #24]",
        "str r0, [sp, #8]",
        "ldr r0, [r4, #28]",
        "str r0, [sp, #12]",
        "ldmia r4, {{r0-r3}}",
        "blx r12",
        "vmov r0, s0",
        "add sp, sp, #16",
        "pop {{r4, pc}}",
    );
}

// 主机检查只验证对象／参数布局，不以主机 ABI 调用游戏函数。
#[cfg(not(target_arch = "arm"))]
pub unsafe fn trig_call(_: u32, _: u32) -> u32 {
    unreachable!("ARM32 VFP 调用仅支持游戏进程")
}

#[cfg(not(target_arch = "arm"))]
pub unsafe fn float_call8(_: u32, _: *const [u32; 8]) -> u32 {
    unreachable!("ARM32 VFP 调用仅支持游戏进程")
}
