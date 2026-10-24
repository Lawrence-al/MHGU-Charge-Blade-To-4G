//! 4G后段爆炸及瓶爆视觉路由。只接管创建节点，不改伤害、瓶数或等待帧。
extern crate alloc;

use alloc::vec::Vec;
use core::cell::UnsafeCell;
use forge::os::sync::{LazyLock, Mutex, ScopedLock};
use cb_game::{
    effect::{EffectCreateParams, EffectNode, PelHandle, WeaponEffectShell},
    player::{ChargeBladePlayer, PhialKind},
    shell::SaedShell,
    surface,
};
use crate::{config_file, gaxe, state::STATE};

#[path = "phial_bridges.rs"]
mod bridges;

struct ActionVisuals {
    player: ChargeBladePlayer,
    action: u16,
    motion: u16,
    aed: bool,
    hit: bool,
    ground: Option<EffectNode>,
    saed_pending: bool,
}
struct Visuals {
    mutex: Mutex,
    entries: UnsafeCell<Vec<ActionVisuals>>,
}
// 同一锁内读写去重记录；创建资源和游戏函数均在解锁之后执行。
unsafe impl Send for Visuals {}
unsafe impl Sync for Visuals {}
static VISUALS: LazyLock<Visuals> = LazyLock::new(|| Visuals {
    mutex: Mutex::new(),
    entries: UnsafeCell::new(Vec::new()),
});

impl ActionVisuals {
    unsafe fn new(player: ChargeBladePlayer) -> Self {
        Self { player, action: unsafe { player.action_id() }, motion: unsafe { player.motion_id() },
            aed: false, hit: false, ground: None, saed_pending: false }
    }
    unsafe fn matches(&self, player: ChargeBladePlayer) -> bool {
        self.player == player && self.action == unsafe { player.action_id() }
            && self.motion == unsafe { player.motion_id() }
    }
}

fn with_visuals<T>(f: impl FnOnce(&mut Vec<ActionVisuals>) -> T) -> T {
    let visuals = &*VISUALS;
    let _lock = ScopedLock::new(&visuals.mutex);
    f(unsafe { &mut *visuals.entries.get() })
}

/// 在原动作更新后登记：state0此时已经建立新动画；原音效/闪电的更新顺序不动。
pub unsafe fn on_action_frame(player: ChargeBladePlayer, variant: u32, before: u32) {
    let pending = with_visuals(|entries| {
        if before == 0 {
            let mut fresh = unsafe { ActionVisuals::new(player) };
            fresh.aed = matches!(variant, 1 | 7);
            if let Some(entry) = entries.iter_mut().find(|entry| entry.player == player) {
                *entry = fresh;
            } else { entries.push(fresh); }
        }
        entries.iter().any(|entry| unsafe { entry.matches(player) } && entry.saed_pending)
    });
    // 必须使用原IS_ack跨帧判断；同次更新可跨过发瓶和111帧，不以当前帧>=111代替。
    if pending && unsafe { player.crossed_frame(STATE.game(), 111.0) } {
        let fire = with_visuals(|entries| {
            if let Some(entry) = entries.iter_mut().find(|entry| unsafe { entry.matches(player) }) {
                let fire = entry.saed_pending;
                entry.saed_pending = false;
                fire
            } else { false }
        });
        if fire { unsafe { create_saed_ground(player) }; }
    }
}

/// 原CreateShell03调用前捕获真实瓶数；返回值只是visual-suppress XOR1，不能当创建成功。
/// 地面在111帧出后段；空战state2已通过真正落地判定，直接在落地事件出一次。
pub unsafe fn on_saed_created(player: ChargeBladePlayer, before: u32, had_phial: bool) {
    if !had_phial || !config_file::get().four_g.saed_explosion { return; }
    if before == 2 {
        unsafe { create_saed_ground(player) };
    } else if before == 1 {
        with_visuals(|entries| {
            if let Some(entry) = entries.iter_mut().find(|entry| unsafe { entry.matches(player) }) {
                entry.saed_pending = true;
            } else {
                let mut entry = unsafe { ActionVisuals::new(player) };
                entry.saed_pending = true;
                entries.push(entry);
            }
        });
    }
}

unsafe fn create_saed_ground(player: ChargeBladePlayer) {
    let (position, rotation) = unsafe { player.saed_ground_pose(STATE.game()) };
    unsafe { gaxe::create_world_explosion(player, position, rotation) };
}

/// 原PSL的实际尘土请求：补入原PEL(-50,0,250)/mode1/bone-1，不沿用尘土0.35尺度。
pub unsafe fn on_sequence_created(player: ChargeBladePlayer, handle: u32, kind: u32, id: u32,
    params: &EffectCreateParams, original_node: u32) {
    if !config_file::get().four_g.aed_explosion || original_node == 0 || kind != 1 || id != 0x78 { return; }
    let game = STATE.game();
    if !unsafe { player.is_instance(game) } || handle != unsafe { player.sequence_pel().address() } { return; }
    let eligible = with_visuals(|entries| entries.iter().any(|entry|
        unsafe { entry.matches(player) } && entry.aed && !entry.hit && entry.ground.is_none()));
    if !eligible { return; }
    let mut visual = unsafe { core::ptr::read(params) };
    if !unsafe { visual.inherit_pel_transform(game, PelHandle::from_address(handle), kind, id) } { return; }
    let mask = if unsafe { player.phial_kind() } == PhialKind::Element { 2 } else { 1 };
    if let Some(node) = unsafe { gaxe::create_visual(player.effect_shell(), 0x1005, mask, &visual) } {
        with_visuals(|entries| {
            if let Some(entry) = entries.iter_mut().find(|entry| unsafe { entry.matches(player) }) {
                entry.ground = Some(node);
            }
        });
    }
}

/// type2/3是已经捕获怪物接触的AED三爆；继承进程的真实命中部位，而不是剑根。
unsafe fn on_aed_contact(shell: WeaponEffectShell, kind: u8, params: &EffectCreateParams) {
    if !config_file::get().four_g.aed_explosion || !matches!(kind, 2 | 3) { return; }
    let player = unsafe { shell.owner() };
    let ground = with_visuals(|entries| {
        if let Some(entry) = entries.iter_mut().find(|entry| unsafe { entry.matches(player) }) {
            entry.hit = true;
            entry.ground.take()
        } else { None }
    });
    if let Some(node) = ground { unsafe { node.stop_immediately(STATE.game()) }; }
    let _ = unsafe { gaxe::create_visual(shell, 0x1005, if kind == 2 { 1 } else { 2 }, params) };
}

pub fn forget_node(node: u32) {
    with_visuals(|entries| {
        for entry in entries {
            if entry.ground.map(EffectNode::address) == Some(node) { entry.ground = None; }
        }
    });
}

/// 首爆函数仍执行原伤害/清槽；只把本插件002的软停止补为立即关闭已有粒子。
pub unsafe fn first_blast_attachment(shell: WeaponEffectShell) -> Option<EffectNode> {
    unsafe { shell.attachment() }.filter(|node| unsafe { node.effect_id() } == 2)
}

/// 4G按完整u64元素值分支；混合或未知值不猜成某一种属性，回退原GU视觉。
fn element_mask(flags: u64) -> Option<u32> {
    match flags {
        16 => Some(1),
        64 => Some(2),
        32 => Some(4),
        512 => Some(8),
        128 => Some(16),
        _ => None,
    }
}

/// 来自00488358：所有四种CreateShell02第一阶段都是瓶附加002。
pub unsafe extern "C" fn create_attachment(
    shell: u32,
    definition: *const u32,
    params: &mut EffectCreateParams,
) -> u32 {
    let shell = WeaponEffectShell::from_address(shell);
    let kind = unsafe { shell.process_type() };
    unsafe { on_aed_contact(shell, kind, params) };
    if config_file::get().four_g.phial_attachment && kind <= 3 {
        let mask = if kind & 1 == 0 { 1 } else { 2 };
        if let Some(node) = unsafe { gaxe::create_visual(shell, 2, mask, params) } {
            unsafe { companion(shell, definition, node) };
            // 交回原slot5596；首爆0048874C会停止附加并清槽，不能绕开这次停止。
            return node.address();
        }
    }
    unsafe { shell.create_defined(STATE.game(), definition, params).address() }
}

/// 来自004886EC：这里只改变延迟瓶爆的资源及分组，原STR/等待节点逻辑继续执行。
pub unsafe extern "C" fn create_blast(
    shell: u32,
    definition: *const u32,
    params: &mut EffectCreateParams,
) -> u32 {
    let shell = WeaponEffectShell::from_address(shell);
    let config = &config_file::get().four_g;
    let kind = unsafe { shell.process_type() };
    let replacement = match kind {
        0 if config.ed_blast && unsafe { shell.selector() } == 0 => Some((3, 1)),
        1 if config.ed_blast && unsafe { shell.selector() } == 0 =>
            element_mask(unsafe { shell.element_flags() }).map(|mask| (8, mask)),
        2 if config.aed_blast => Some((6, if unsafe { shell.remaining_phials() } == 1 { 1 << 10 } else { 1 })),
        3 if config.aed_blast => element_mask(unsafe { shell.element_flags() }).map(|mask| {
            (9, if unsafe { shell.remaining_phials() } == 1 { mask << 10 } else { mask })
        }),
        _ => None,
    };
    if let Some((id, mask)) = replacement {
        // 原212/213的cm_001 PEP随机旋转Z；从原PEL借用资源，不改原始栈参数。
        let mut visual = unsafe { core::ptr::read(params) };
        if kind >= 2 {
            let list = unsafe { *definition };
            let old_id = unsafe { *definition.add(1) };
            if let Some(script) = unsafe { shell.param_script(STATE.game(), list, old_id) } {
                unsafe { visual.borrow_param_script(script) };
            }
        }
        if let Some(node) = unsafe { gaxe::create_visual(shell, id, mask, &visual) } {
            // GU原先只创建一次组合EFL，后两爆重建4G粒子时不能重复原PTS音效。
            if unsafe { shell.action_state() } == 0 {
                unsafe { companion(shell, definition, node) };
            }
            return node.address();
        }
    }
    // state2本来根本不会创建GU组合EFL；新增单次视觉失败也不能重放它及其PTS。
    if config.aed_blast && unsafe { shell.action_state() } == 2 && matches!(kind, 2 | 3) {
        return 0;
    }
    unsafe { shell.create_defined(STATE.game(), definition, params).address() }
}

/// 原生state0与2均已生成伤害，只有state0会创建组合EFL。
/// 4G006/009是单次爆炸，因此AED后两次也通过视觉创建门；不写真实state。
pub unsafe extern "C" fn visual_state(shell: u32, state: u32) -> u32 {
    let shell = WeaponEffectShell::from_address(shell);
    if state == 2 && config_file::get().four_g.aed_blast
        && matches!(unsafe { shell.process_type() }, 2 | 3)
    {
        0
    } else {
        state
    }
}

/// 004A1E50入口的窄类型过滤，同时覆盖GU原生和Moveset直接提交的SAED瓶爆。
/// None表示原函数照常执行；Some只替换本次视觉，不增加瓶爆调用。
pub unsafe fn replace_saed(
    shell: u32,
    definition: *const u32,
    params: &mut EffectCreateParams,
) -> Option<u32> {
    let game = STATE.game();
    let shell = SaedShell::from_address(shell);
    if !unsafe { shell.is_instance(game) } {
        return None;
    }
    if !config_file::get().four_g.saed_blast {
        return None;
    }
    let setup = unsafe { shell.params() };
    let last = unsafe { setup.phial_count() } == 1;
    let mask = if unsafe { setup.kind() } == PhialKind::Impact {
        if last { 2 } else { 1 }
    } else {
        let base = element_mask(unsafe { shell.element_flags() })? << 2;
        if last { base << 8 } else { base }
    };
    let effect_shell = WeaponEffectShell::from_address(shell.address());
    let mut visual = unsafe { core::ptr::read(params) };
    visual.set_auto_surface(false);
    let node = unsafe { gaxe::create_visual(effect_shell, 107, mask, &visual) }?;
    unsafe {
        surface::apply_at_shell(game, shell, node);
        companion(effect_shell, definition, node);
    }
    Some(node.address())
}

/// 新PEL没有音表；仍用原定义的ID和PTS，只提交原本那一声。
unsafe fn companion(shell: WeaponEffectShell, definition: *const u32, node: EffectNode) {
    let group = unsafe { *definition };
    let old_id = unsafe { *definition.add(1) };
    unsafe { node.play_companion(STATE.game(), shell.sound_table(group), old_id) };
}

pub fn enable(base: u32) {
    let config = &config_file::get().four_g;
    LazyLock::force(&VISUALS);
    bridges::install(base, config.phial_attachment || config.aed_explosion, config.ed_blast || config.aed_blast, config.aed_blast);
}
