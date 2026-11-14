//! 大解结束后留在同一动作的state2收斧；红盾原0..28帧及原派生记录完全不改。
use crate::{
    config_file,
    patch::BranchPatch,
    style::{self, Feature},
};
use alloc::vec::Vec;
use cb_game::{
    Game,
    motion::{CommandClock, PlayerMotion},
    player::ChargeBladePlayer,
};
use core::{
    arch::{global_asm, naked_asm},
    cell::UnsafeCell,
};
use forge::os::sync::{LazyLock, Mutex, ScopedLock};

const RECOVERY_STATE: u32 = 2;
// 原盾斧分发末尾：add sp,#8; pop {r4,lr}; b 011DABC8。
// 只替换前两条；恢复原栈后尾跳NRO，再由NRO调用原入口，保留Presentation钩子。
const DISPATCH_SITE: u32 = 0x011D6914;
static mut DISPATCH_CONTINUATION: u32 = 0;
const END_SITE: u32 = 0x011DAD48;
static mut END_CONTINUATION: u32 = 0;
global_asm!(".hidden {slot}", slot = sym END_CONTINUATION);

#[derive(Clone, Copy)]
struct Recovery {
    player: ChargeBladePlayer,
    action: u16,
    end: f32,
}
struct Recoveries {
    mutex: Mutex,
    actors: UnsafeCell<Vec<Recovery>>,
}
unsafe impl Send for Recoveries {}
unsafe impl Sync for Recoveries {}
static RECOVERIES: LazyLock<Recoveries> = LazyLock::new(|| Recoveries {
    mutex: Mutex::new(),
    actors: UnsafeCell::new(Vec::new()),
});

fn with_actors<R>(f: impl FnOnce(&mut Vec<Recovery>) -> R) -> R {
    let states = &*RECOVERIES;
    let _lock = ScopedLock::new(&states.mutex);
    f(unsafe { &mut *states.actors.get() })
}
fn forget(player: ChargeBladePlayer) {
    with_actors(|actors| actors.retain(|state| state.player != player));
}
unsafe fn current(player: ChargeBladePlayer) -> Option<Recovery> {
    if unsafe { player.action_state() } != RECOVERY_STATE {
        return None;
    }
    let action = unsafe { player.action_id() };
    if !matches!(action, 0x2D | 0x2E | 0x80 | 0x81) {
        return None;
    }
    with_actors(|actors| {
        actors
            .iter()
            .find(|state| state.player == player && state.action == action)
            .copied()
    })
}

#[inline(never)]
unsafe extern "C" fn dispatch(actor: u32, variant: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    let game = Game::new(crate::game_base());
    let motion = PlayerMotion::new(player);
    // 仅0/1/6/7原本会接0x2A；黄盾特殊动作4/5及超解2/3/8/9完全走原入口。
    if !matches!(variant, 0 | 1 | 6 | 7) {
        return unsafe { motion.update_discharge(game, variant) };
    }
    let active = unsafe { current(player) }.is_some();
    if !active && !unsafe { style::enabled_for(player, Feature::AedRecovery) } {
        return unsafe { motion.update_discharge(game, variant) };
    }
    match unsafe { player.action_state() } {
        0 => forget(player),
        RECOVERY_STATE if active => {
            if unsafe { motion.blend_remaining() } <= 0.0 {
                forget(player);
                return unsafe { motion.return_to_neutral(game) };
            }
        }
        _ => {}
    }
    // state2下原函数只做共通动作维护；不会再初始化HDP、扣瓶或处理攻击事件。
    unsafe { motion.update_discharge(game, variant) }
}

/// 仅原生普通大解结束的0x2A分支调用。先让Presentation完成当帧任务，再进入恢复。
#[inline(never)]
unsafe extern "C" fn begin(actor: u32) -> u32 {
    let player = ChargeBladePlayer::from_address(actor);
    if !unsafe { style::enabled_for(player, Feature::AedRecovery) } {
        return 0;
    }
    let motion = PlayerMotion::new(player);
    let state = Recovery {
        player,
        action: unsafe { player.action_id() },
        end: unsafe { motion.end_frame() },
    };
    with_actors(|actors| {
        actors.retain(|old| old.player != player);
        actors.push(state);
    });
    unsafe {
        motion.stop_attack_controllers();
        motion.set_action_state(RECOVERY_STATE as u8);
        motion.blend_to_axe_idle(
            Game::new(crate::game_base()),
            config_file::get().moveset.aed_recovery_frames,
        );
    }
    1
}

/// 此处原大解仅压栈{r4,r5,r6,lr}；启用时直接完成该帧返回，关闭时补回两条MOV。
#[unsafe(naked)]
unsafe extern "C" fn end_bridge() {
    naked_asm!(
        ".arm",
        "push {{r0-r3, r12, lr}}",
        "mov r0, r4",
        "bl {begin}",
        "cmp r0, #0",
        "pop {{r0-r3, r12, lr}}",
        "beq 2f",
        "mov r0, #0",
        "pop {{r4-r6, pc}}",
        "2:",
        "mov r0, r4",
        "mov r1, #16",
        "push {{r12, lr}}",
        "ldr r12, 3f",
        "4:",
        "add r12, pc, r12",
        "ldr r12, [r12]",
        "str r12, [sp, #4]",
        "pop {{r12, pc}}",
        ".balign 4",
        "3:",
        ".word {continuation} - (4b + 8)",
        begin = sym begin,
        continuation = sym END_CONTINUATION,
    );
}

#[unsafe(naked)]
unsafe extern "C" fn dispatch_bridge() {
    naked_asm!(
        ".arm",
        "add sp, sp, #8",
        "pop {{r4, lr}}",
        "b {dispatch}",
        dispatch = sym dispatch,
    );
}

// 输入查询临时使用原大解终帧，结束后恢复真实收斧动画时钟。
// 记录、红盾窗口及逐记录输入缓存不改；原命令决定派生是否合法。
crate::cb_hook! { 0x002E9D54,
fn hook_command_clock(actor: u32) -> u32, |original| {
    let player = ChargeBladePlayer::from_address(actor);
    if let Some(state) = unsafe { current(player) } {
        let motion = PlayerMotion::new(player);
        let clip = unsafe { player.motion_id() };
        let saved = unsafe { motion.clock() };
        unsafe { motion.set_clock(CommandClock::finished(state.end)) };
        let result = unsafe { original(actor) };
        // 原查询可能提交新动作；若已创建其它动画，不覆盖它的新时钟。
        if unsafe { player.motion_id() } == clip {
            unsafe { motion.set_clock(saved) };
        }
        result
    } else {
        unsafe { original(actor) }
    }
}
}

pub fn collect(branches: &mut Vec<BranchPatch>) {
    branches.push(BranchPatch {
        offset: DISPATCH_SITE,
        original: 0xE28DD008,
        second: 0xE8BD4010,
        cheat_call: 0xE28DD008,
        target: dispatch_bridge as *const () as u32,
        continuation: &raw mut DISPATCH_CONTINUATION,
        call_original: false,
    });
    branches.push(BranchPatch {
        offset: END_SITE,
        original: 0xE1A00004,
        second: 0xE3A01010,
        cheat_call: 0xE1A00004,
        target: end_bridge as *const () as u32,
        continuation: &raw mut END_CONTINUATION,
        call_original: false,
    });
}

pub fn enable(base: u32) {
    LazyLock::force(&RECOVERIES);
    forge::install_hook!(base, hook_command_clock);
}
