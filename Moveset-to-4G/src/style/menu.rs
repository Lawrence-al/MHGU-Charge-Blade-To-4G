//! 原版风格菜单第七项：复用原有八行布局，游戏装备仍保存公会 0。
use crate::patch::BranchPatch;
use alloc::vec::Vec;
use cb_game::{Game, memory, player::ChargeBladePlayer};
use core::{
    arch::{global_asm, naked_asm},
    mem, ptr,
};

const STYLE_ORDER: [u32; 6] = [0, 1, 2, 3, 5, 4];
const STYLE_NODES: [u32; 7] = [7, 8, 9, 10, 0x46, 0x47, 0x4E];
const FOUR_G_NAME: &[u8] = b"4G\0";
const FOUR_G_DESCRIPTION: &[u8] =
    b"4G Charge Blade\nAED / SAED, guard and GP.\nConfigure in Forge Plugins.\0";
const CONFIRM_SITE: u32 = 0x0054924C;
const COMPARE_SITE: u32 = 0x00549784;
static mut CONFIRM_CONTINUATION: u32 = 0;
static mut COMPARE_CONTINUATION: u32 = 0;
global_asm!(".hidden {confirm}", ".hidden {compare}",
    confirm = sym CONFIRM_CONTINUATION, compare = sym COMPARE_CONTINUATION);

/// 仅菜单显示使用行 6；这个编号绝不写入游戏装备的 +0x4D9。
fn confirmed_style(row: u32, original_style: u32) -> u32 {
    if row == 6 { 0 } else { original_style }
}

/// 原版会把当前项直接与公会 0 比较。额外选择需区分“公会”和“4G”。
/// 0x100 仅用于“不相等”的比较结果，不会与任意 u8 风格编号冲突。
fn comparison_style(
    row: u32,
    weapon: u8,
    actual_style: u8,
    selected: bool,
    original_style: u32,
) -> u32 {
    let active = weapon == 14 && actual_style == 0 && selected;
    if row == 6 {
        if active { 0 } else { 0x100 }
    } else if row == 0 && active {
        0x100
    } else {
        original_style
    }
}

unsafe fn local_player() -> ChargeBladePlayer {
    unsafe { Game::new(crate::game_base()).local_player() }
}

unsafe fn player_style(player: ChargeBladePlayer) -> u8 {
    unsafe { player.equipment().style() }
}

unsafe fn is_charge_blade() -> bool {
    unsafe { local_player().equipment().weapon_type() == 14 }
}

unsafe fn selected_for_player() -> bool {
    let player = unsafe { local_player() };
    (unsafe { player.equipment().weapon_type() == 14 && player_style(player) == 0 })
        && super::selection::selected()
}

unsafe fn text(ui: u32, node: u32, child: u32, value: &[u8]) {
    let write: unsafe extern "C" fn(u32, u32, u32, *const u8) -> u32 =
        unsafe { mem::transmute(crate::game_base() + 0x0056EA08) };
    unsafe { write(ui, node, child, value.as_ptr()) };
}

unsafe fn child_visible(ui: u32, node: u32, child: u32, visible: bool) {
    // IDA 导出的旧声明只有一个参数；原 ARM 实际沿用 r1/r2/r3 传递后三参。
    let set: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
        unsafe { mem::transmute(crate::game_base() + 0x0056E530) };
    unsafe { set(ui, node, child, visible as u32) };
}

unsafe fn current_highlight(ui: u32) {
    if unsafe { selected_for_player() } {
        unsafe {
            child_visible(ui, STYLE_NODES[0], 1, false);
            child_visible(ui, STYLE_NODES[6], 1, true);
        }
    } else if unsafe { is_charge_blade() } {
        unsafe { child_visible(ui, STYLE_NODES[6], 1, false) };
    }
}

unsafe extern "C" fn commit_row(row: u32, player: u32, original_style: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(player);
    if unsafe { player.equipment().weapon_type() == 14 } {
        super::selection::set_selected(row == 6);
    }
    confirmed_style(row, original_style)
}

unsafe extern "C" fn compare_row(row: u32, address: u32, original_style: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(address);
    comparison_style(
        row,
        unsafe { player.equipment().weapon_type() },
        unsafe { player_style(player) },
        super::selection::selected(),
        original_style,
    )
}

/// r0=确认行号、r6=玩家、r5=菜单；替代六项表读取并补回 movw r0,#0x13C4。
/// Rust 保存配置会调用文件函数，因此进入前保存 NZCV / VFP 并保持栈八字节对齐。
#[unsafe(naked)]
unsafe extern "C" fn style_confirm_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "mrs r12, cpsr",
        "push {{r12, lr}}",
        "vpush {{d0-d15}}",
        // 非 4G 行使用现场原表；其它 NRO 新增的风格编号不查本插件六项数组。
        "cmp r0, #6",
        "moveq r2, #0",
        "ldrbne r2, [r1, r0, lsl #2]",
        "mov r1, r6",
        "bl {helper}",
        "mov r7, r0",
        "vpop {{d0-d15}}",
        "pop {{r12, lr}}",
        "msr cpsr_f, r12",
        "pop {{r0-r3, r12, lr}}",
        "movw r0, #0x13C4",
        "push {{r12, lr}}",
        "ldr r12, 2f",
        "3:", "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4", "2:", ".word {slot} - (3b + 8)",
        helper = sym commit_row, slot = sym CONFIRM_CONTINUATION,
    );
}

/// r1=光标行号、r0=玩家；恢复后补回 ldr r1,[r0]，原函数继续虚调用读取实际风格。
#[unsafe(naked)]
unsafe extern "C" fn style_compare_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "mrs r12, cpsr",
        "push {{r12, lr}}",
        "vpush {{d0-d15}}",
        "cmp r1, #6",
        "moveq r3, #0",
        "ldrne r3, [r2, r1, lsl #2]",
        "mov r2, r0", "mov r0, r1", "mov r1, r2",
        "mov r2, r3",
        "bl {helper}",
        "mov r7, r0",
        "vpop {{d0-d15}}",
        "pop {{r12, lr}}",
        "msr cpsr_f, r12",
        "pop {{r0-r3, r12, lr}}",
        "ldr r1, [r0]",
        "push {{r12, lr}}",
        "ldr r12, 2f",
        "3:", "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4", "2:", ".word {slot} - (3b + 8)",
        helper = sym compare_row, slot = sym COMPARE_CONTINUATION,
    );
}

crate::cb_hook! { 0x00637958,
fn hook_style_labels(ui: u32) -> u32, |original| {
    let result = unsafe { original(ui) };
    if unsafe { is_charge_blade() } {
        unsafe { text(ui, STYLE_NODES[6], 2, FOUR_G_NAME) };
    }
    result
}
}

crate::cb_hook! { 0x00637B28,
fn hook_style_panel(ui: u32, state: u32) -> u32, |original| {
    let old_state = unsafe { memory::read_u32(ui + 92) };
    let result = unsafe { original(ui, state) };
    if old_state != state && matches!(state, 1 | 2) && unsafe { is_charge_blade() } {
        // 只在原菜单切换时设置。八行节点和控件已存在，不在每帧重建 UI。
        let cursor = unsafe { memory::read_u32(ui + 80) };
        let initialize: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
            unsafe { mem::transmute(crate::game_base() + 0x0054EDEC) };
        let style = unsafe { player_style(local_player()) };
        let row = if unsafe { selected_for_player() } { 6 }
            else if style < 6 { STYLE_ORDER[style as usize] }
            else { (unsafe { memory::read_u8(cursor + 13) }) as u32 };
        // GU 游标 +20 为 u16 总条数；保留前一个入口钩子已经扩展的菜单。
        let rows = (unsafe { memory::read_u16(cursor + 20) } as u32).max(7);
        let vtable = unsafe { memory::read_u32(cursor) };
        let select: unsafe extern "C" fn(u32, u32) -> u32 =
            unsafe { mem::transmute(memory::read_u32(vtable + 60)) };
        let count: unsafe extern "C" fn(u32, u32) -> u32 =
            unsafe { mem::transmute(crate::game_base() + 0x005C5500) };
        unsafe {
            initialize(cursor, rows, 1, 0);
            select(cursor, row);
            count(memory::read_u32(ui + 88), rows);
            text(ui, STYLE_NODES[6], 2, FOUR_G_NAME);
            current_highlight(ui);
        }
    }
    result
}
}

crate::cb_hook! { 0x00638C20,
fn hook_style_equipment_info(ui: u32, weapon: u32, style: u32, unused: u32, argument: u32) -> u32, |original| {
    let result = unsafe { original(ui, weapon, style, unused, argument) };
    if weapon == 14 { unsafe { current_highlight(ui) }; }
    result
}
}

crate::cb_hook! { 0x00638D00,
fn hook_style_highlight(ui: u32) -> u32, |original| {
    let result = unsafe { original(ui) };
    unsafe { current_highlight(ui) };
    result
}
}

crate::cb_hook! { 0x00638D34,
fn hook_style_current_name(ui: u32, player: u32) -> u32, |original| {
    let result = unsafe { original(ui, player) };
    if unsafe { selected_for_player() } {
        unsafe { text(ui, 0x55, 1, FOUR_G_NAME) };
    }
    result
}
}

crate::cb_hook! { 0x006394B8,
fn hook_style_description(ui: u32, kind: u32, row: u32, page: u32) -> u32, |original| {
    if kind == 0 && row == 6 && unsafe { is_charge_blade() } {
        // 原名称／说明表只有六项；第七项完全在此处理，不进入原表。
        unsafe {
            memory::write_u32(ui + 76, kind);
            memory::write_u32(ui + 80, row);
            text(ui, 2, 2, FOUR_G_NAME);
            text(ui, 3, 2, FOUR_G_DESCRIPTION);
            let visible: unsafe extern "C" fn(u32, u32, u32) -> u32 =
                mem::transmute(crate::game_base() + 0x0056E528);
            visible(ui, 4, 0);
            visible(ui, 5, 0);
            let show: unsafe extern "C" fn(u32) -> u32 =
                mem::transmute(crate::game_base() + 0x0056DE60);
            show(memory::read_u32(ui + 88))
        }
    } else {
        unsafe { original(ui, kind, row, page) }
    }
}
}

pub fn collect(branches: &mut Vec<BranchPatch>) {
    branches.push(BranchPatch {
        offset: CONFIRM_SITE,
        original: 0xE7D17100,
        cheat_call: 0xE7D17100,
        second: 0xE30103C4,
        target: style_confirm_bridge as *const () as u32,
        continuation: ptr::addr_of_mut!(CONFIRM_CONTINUATION),
    });
    branches.push(BranchPatch {
        offset: COMPARE_SITE,
        original: 0xE7927101,
        cheat_call: 0xE7927101,
        second: 0xE5901000,
        target: style_compare_bridge as *const () as u32,
        continuation: ptr::addr_of_mut!(COMPARE_CONTINUATION),
    });
}

pub fn enable(base: u32) {
    forge::install_hook!(base, hook_style_labels);
    forge::install_hook!(base, hook_style_panel);
    forge::install_hook!(base, hook_style_equipment_info);
    forge::install_hook!(base, hook_style_highlight);
    forge::install_hook!(base, hook_style_current_name);
    forge::install_hook!(base, hook_style_description);
}
