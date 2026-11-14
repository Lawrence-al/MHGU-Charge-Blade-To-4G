//! 原直接改字的等效实现。main 只通过 Forge 安装 Hook，常量／分支在 NRO 中执行。
use crate::{config::Config, patch::BranchPatch};
use alloc::vec::Vec;
use core::arch::{global_asm, naked_asm};

static mut RED_RETURN: u32 = 0;
static mut BLUE_RETURN: u32 = 0;
static mut SHAKE_RETURN: u32 = 0;
static mut GROUND_HIT_RETURN: u32 = 0;
static mut AIR_HIT_RETURN: u32 = 0;
static mut GROUND_FRAME_RETURN: u32 = 0;
static mut AIR_FRAME_RETURN: u32 = 0;
static mut DAMAGE_RETURN: u32 = 0;
static mut GEAR_RETURN: u32 = 0;
static mut HDP_ENTRY: u32 = 0;
static mut EFFECT_GEAR: u32 = 2;
global_asm!(".hidden {hdp}", ".hidden {gear}", hdp = sym HDP_ENTRY, gear = sym EFFECT_GEAR);

// 统一以不改 NZCV 的指令取回续跳地址，保留 r12/sp；无原 BL 时也保留 lr。
// delta 是相对覆盖区末端的转移，不能原样搬运 main 中的 PC 相对 B/BL/VLDR。
macro_rules! bridge {
    ($name:ident, $slot:ident, $advance:literal,
     [$($ins:literal),* $(,)?], [$($data:literal),* $(,)?] $(, $($args:tt)*)?) => {
        global_asm!(".hidden {slot}", slot = sym $slot);
        #[unsafe(naked)]
        unsafe extern "C" fn $name() {
            naked_asm!(
                ".arm",
                $($ins,)*
                "push {{r12, lr}}",
                "ldr r12, 90f",
                "91:",
                "add r12, pc, r12",
                "ldr r12, [r12]",
                $advance,
                "str r12, [sp, #4]",
                "pop {{r12, pc}}",
                ".balign 4",
                "90:",
                ".word {resume} - (91b + 8)",
                $($data,)*
                resume = sym $slot,
                $($($args)*)?
            );
        }
    };
}

// 红盾原来的第二条是 B 011DF8D8；蓝盾第二条是条件 MOV，写同一个档位 2。
bridge!(
    red_multiplier_bridge,
    RED_RETURN,
    "add r12, r12, #32",
    ["mov r1, #2"],
    []
);
bridge!(
    blue_multiplier_bridge,
    BLUE_RETURN,
    "add r12, r12, #0",
    ["mov r1, #2"],
    []
);
// 原 MOV 后是 B 0048864C。由原分支目标继续提交震动，而不是执行 fallthrough。
bridge!(
    aed_shake_bridge,
    SHAKE_RETURN,
    "add r12, r12, #168",
    ["mov r1, #20"],
    []
);

// 原六条指令整组替换为 HDP 调用；跳过旧虚函数查询及条件分支，共推进 24 字节。
bridge!(ground_hit_bridge, GROUND_HIT_RETURN, "add r12, r12, #16\nsub lr, r12, #4", [
    "push {{r4, lr}}", "mov r0, r4", "mov r1, #0", "mov r2, #0x50", "mov r3, #6",
    "ldr r12, 80f", "81:", "add r12, pc, r12", "ldr r12, [r12]", "blx r12", "pop {{r4, lr}}",
], ["80:", ".word {hdp} - (81b + 8)"], hdp = sym HDP_ENTRY);
bridge!(air_hit_bridge, AIR_HIT_RETURN, "add r12, r12, #16\nsub lr, r12, #4", [
    "push {{r4, lr}}", "mov r0, r4", "mov r1, #1", "mov r2, #0x23", "mov r3, #6",
    "ldr r12, 80f", "81:", "add r12, pc, r12", "ldr r12, [r12]", "blx r12", "pop {{r4, lr}}",
], ["80:", ".word {hdp} - (81b + 8)"], hdp = sym HDP_ENTRY);

// 直接改108的读取处，main的只读literal pool不动。
// 空中覆盖两条VLDR并保留11DBD58的MOV，110帧分支也会直接跳到这里。
bridge!(
    ground_frame_bridge,
    GROUND_FRAME_RETURN,
    "add r12, r12, #0",
    ["vldr s0, 80f", "mov r0, r4"],
    ["80:", ".word 0x42DA0000"]
);
bridge!(
    air_frame_bridge,
    AIR_FRAME_RETURN,
    "add r12, r12, #0",
    ["vldr s0, 80f", "vldr s1, 81f"],
    ["80:", ".word 0x00000000", "81:", ".word 0x42DA0000"]
);

// 与旧四条改字完全等效：s18=0x3FACCCCC，s2=1.0；保留前面的 s0=0.5。
bridge!(
    phial_damage_bridge,
    DAMAGE_RETURN,
    "add r12, r12, #8",
    ["vldr s18, 80f", "vmov.f32 s2, #1.0"],
    ["80:", ".word 0x3FACCCCC"]
);

// 瓶数已经由原 LDR 读到 r0。非零固定档位并跳 011DE860；零瓶回原零瓶分支。
// 只覆盖原 CMP/BEQ，保留 r10 的原写回和真实瓶数；ADDNE 保留 CMP 的 NZCV。
bridge!(effect_gear_bridge, GEAR_RETURN, "addne r12, r12, #32", [
    "ldr r9, 80f", "81:", "add r9, pc, r9", "ldr r9, [r9]", "cmp r0, #0",
], ["80:", ".word {gear} - (81b + 8)"], gear = sym EFFECT_GEAR);

fn push(
    branches: &mut Vec<BranchPatch>,
    offset: u32,
    original: u32,
    legacy: u32,
    second: u32,
    target: u32,
    continuation: *mut u32,
) {
    branches.push(BranchPatch {
        offset,
        original,
        cheat_call: legacy,
        second,
        target,
        continuation,
        call_original: false,
    });
}

pub unsafe fn collect(base: u32, config: &Config, branches: &mut Vec<BranchPatch>) {
    if !config.enabled {
        return;
    }
    if config.moveset.shield_multiplier {
        push(
            branches,
            0x011DF8B0,
            0xE3A01003,
            0xE3A01002,
            0xEA000007,
            red_multiplier_bridge as *const () as u32,
            &raw mut RED_RETURN,
        );
        push(
            branches,
            0x011DF8D0,
            0x13A01001,
            0xE3A01002,
            0x03A01002,
            blue_multiplier_bridge as *const () as u32,
            &raw mut BLUE_RETURN,
        );
    }
    if config.moveset.aed_camera_shake {
        push(
            branches,
            0x0048859C,
            0xE3A0100F,
            0xE3A01014,
            0xEA000029,
            aed_shake_bridge as *const () as u32,
            &raw mut SHAKE_RETURN,
        );
    }
    if config.saed.enabled {
        unsafe {
            HDP_ENTRY = base + 0x002B7240;
            EFFECT_GEAR = config.saed.effect_gear as u32;
        }
        push(
            branches,
            0x011DADD4,
            0xE5940000,
            0xE1A00004,
            0xE59011CC,
            ground_hit_bridge as *const () as u32,
            &raw mut GROUND_HIT_RETURN,
        );
        push(
            branches,
            0x011DBDA0,
            0xE5940000,
            0xE1A00004,
            0xE59011CC,
            air_hit_bridge as *const () as u32,
            &raw mut AIR_HIT_RETURN,
        );
        push(
            branches,
            0x011DADA0,
            0xED9F0AF9,
            0xED9F0AF9,
            0xE1A00004,
            ground_frame_bridge as *const () as u32,
            &raw mut GROUND_FRAME_RETURN,
        );
        push(
            branches,
            0x011DBD50,
            0xED9F0A3C,
            0xED9F0A3C,
            0xEDDF0A3D,
            air_frame_bridge as *const () as u32,
            &raw mut AIR_FRAME_RETURN,
        );
        push(
            branches,
            0x0048906C,
            0xEEB79A00,
            0xED9F9A01,
            0xEE010A10,
            phial_damage_bridge as *const () as u32,
            &raw mut DAMAGE_RETURN,
        );
        push(
            branches,
            0x011DE838,
            0xE350000A,
            0xE3500000,
            0x0A000007,
            effect_gear_bridge as *const () as u32,
            &raw mut GEAR_RETURN,
        );
    }
}
