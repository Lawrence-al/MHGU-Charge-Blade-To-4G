// effect.rs
// 主动创建盾牌上的特效，只按每个节点自定义的持续帧数停止。

extern crate alloc;

use ::core::cell::UnsafeCell;
use alloc::vec::Vec;

// use forge::log::prelude::error;
use forge::os::sync::{LazyLock, Mutex, ScopedLock};

use crate::state::STATE;
use cb_game::{
    effect::{EffectCreateParams, EffectNode},
    player::{ChargeBladePlayer, PhialKind},
};

/// 每个节点独立累计剩余持续帧数。
#[derive(Clone, Copy)]
struct EffectLifetime {
    node: EffectNode,
    remaining_frames: f32,
    // 旧的动作/动画清理字段暂时停用，保留供恢复。
    /*
    player: ChargeBladePlayer,
    action_id: u16,
    motion_id: u16,
    end_frame: f32,
    */
}

struct EffectLifetimes {
    mutex: Mutex,
    entries: UnsafeCell<Vec<EffectLifetime>>,
}

// 所有 entries 访问都在 with_lifetimes 的同一把锁内；游戏函数在解锁后调用。
unsafe impl Send for EffectLifetimes {}
unsafe impl Sync for EffectLifetimes {}

static LIFETIMES: LazyLock<EffectLifetimes> = LazyLock::new(|| EffectLifetimes {
    mutex: Mutex::new(),
    entries: UnsafeCell::new(Vec::new()),
});

fn with_lifetimes<T>(f: impl FnOnce(&mut Vec<EffectLifetime>) -> T) -> T {
    let lifetimes = &*LIFETIMES;
    let _lock = ScopedLock::new(&lifetimes.mutex);
    f(unsafe { &mut *lifetimes.entries.get() })
}

/// 安装特效更新钩子前初始化锁。
pub fn init_lifetimes() {
    LazyLock::force(&LIFETIMES);
}

/// 从节点创建起计时；duration_frames 是特效自身的持续帧数。
pub unsafe fn track_lifetime(node: EffectNode, duration_frames: f32) {
    let lifetime = EffectLifetime {
        node,
        remaining_frames: duration_frames,
        /*
        player,
        action_id: unsafe { player.action_id() },
        motion_id: unsafe { player.motion_id() },
        end_frame,
        */
    };
    with_lifetimes(|entries| entries.push(lifetime));
}

/// 节点析构或主动停止时移除记录，后续不再通过旧地址访问已回收的节点。
pub fn forget_lifetime(node: EffectNode) {
    with_lifetimes(|entries| entries.retain(|entry| entry.node != node));
}

/// 由特效更新钩子调用，使用原生特效的帧增量，与玩家动作帧无关。
pub unsafe fn on_effect_update(node: EffectNode) {
    let expired = with_lifetimes(|entries| {
        let index = entries.iter().position(|entry| entry.node == node)?;

        // 原生节点已收尾时撤销记录，不再对该节点发停止请求。
        if unsafe { node.is_finishing() } {
            entries.swap_remove(index);
            return None;
        }

        let delta = unsafe { node.delta_frame() };
        entries[index].remaining_frames -= delta;
        if entries[index].remaining_frames > 0.0 {
            None
        } else {
            Some(entries.swap_remove(index))
        }
    });

    if let Some(lifetime) = expired {
        unsafe { lifetime.node.stop(STATE.game()) };
    }
}

// 旧方案：按玩家动画绝对帧、动作/动画切换或攻击命中清理。按需求注释，保留实现。
/*
/// 由 uMHEffectCore 每帧更新钩子调用，受击离开瓶爆动作后仍然能检查。
/// 在到帧或切换动作/动画时停止；攻击命中由 on_attack_hit 立即处理。
pub unsafe fn on_effect_update(node: EffectNode) {
    let lifetime =
        with_lifetimes(|entries| entries.iter().find(|entry| entry.node == node).copied());
    let Some(lifetime) = lifetime else {
        return;
    };

    // 原生特效已进入收尾阶段时，只移除记录，让游戏继续处理剩余粒子。
    if unsafe { node.is_finishing() } {
        forget_lifetime(node);
        return;
    }

    let player = lifetime.player;
    let action_id = unsafe { player.action_id() };
    let motion_id = unsafe { player.motion_id() };
    let frame = unsafe { player.motion_frame() };
    let reason = if action_id != lifetime.action_id {
        Some("action_changed")
    } else if motion_id != lifetime.motion_id {
        Some("motion_changed")
    } else if frame >= lifetime.end_frame {
        Some("end_frame")
    } else {
        None
    };

    if let Some(reason) = reason {
        forget_lifetime(node);
        unsafe { finish_lifetime(lifetime, reason) };
    }
}

/// 怪物命中处理完成后调用，只结束使用该玩家两组武器攻击判定的节点。
/// MhHitAtk +0x06 为 0xFF 时没有命中结果，不能将 +0x05 的判定启用当成命中。
pub unsafe fn on_attack_hit(hit_attack: u32) {
    if unsafe { cb_game::memory::read_u8(hit_attack + 6) }
        == 0xFF
    {
        return;
    }

    loop {
        let lifetime = with_lifetimes(|entries| {
            let index = entries.iter().position(|entry| {
                hit_attack == entry.player.primary_hit_address()
                    || hit_attack == entry.player.secondary_hit_address()
            })?;
            Some(entries.swap_remove(index))
        });
        let Some(lifetime) = lifetime else {
            break;
        };

        // 先撤销跟踪再调原生收尾；同一攻击命中多个部位也只停止一次。
        unsafe { finish_lifetime(lifetime, "attack_hit") };
    }
}

unsafe fn finish_lifetime(lifetime: EffectLifetime, reason: &str) {
    let player = lifetime.player;
    let action_id = unsafe { player.action_id() };
    let motion_id = unsafe { player.motion_id() };
    let frame = unsafe { player.motion_frame() };
    let node = lifetime.node;
    let node_address = node.address();

    // PSL 的停止通知最终进入同一条原生收尾路径，不直接删除粒子对象。
    unsafe { node.stop(STATE.game()) };
    error!(
        "se-src-plugin: stop_shield_effect node=0x{node_address:08X} reason={reason} actionID=0x{:02X}->0x{action_id:02X} motion=0x{:04X}->0x{motion_id:04X} frame={frame:.2} end={:.2}",
        lifetime.action_id, lifetime.motion_id, lifetime.end_frame
    );
}
*/

/// 统一按装备瓶型选择盾牌特效，并记录特效自身的持续帧数。
pub unsafe fn create_timed_on_shield(
    player: ChargeBladePlayer,
    impact_effect: u16,
    element_effect: u16,
    duration_frames: f32,
) -> EffectNode {
    let effect_id = if unsafe { player.phial_kind() } == PhialKind::Element {
        element_effect
    } else {
        impact_effect
    };
    let node = unsafe { create_on_shield(player, effect_id) };
    unsafe { track_lifetime(node, duration_frames) };
    node
}

/// 创建 UNIQUE 特效，挂到当前玩家的盾牌骨骼 0。
///
/// 调用前提：动作中的盾牌对象、武器特效 shell 与其 PEL 已加载。
/// 参数和创建出的节点按有效指针直接使用；不额外提交伴生音效。
pub unsafe fn create_on_shield(player: ChargeBladePlayer, effect_id: u16) -> EffectNode {
    let shield = unsafe { player.shield() };
    let reference = unsafe { shield.reference() };
    let shell = unsafe { player.effect_shell() };
    let game = STATE.game();
    let mut params = EffectCreateParams::zeroed();
    unsafe { params.construct(game) };
    params.set_priority(3);
    params.attach_to(reference, 0);
    let node = unsafe { shell.create_unique(game, effect_id, &params) };
    unsafe { params.destruct(game) };
    node
}

/// 盾突起手初始化后有红盾就创建；不等待命中或瓶爆进程。
pub unsafe fn on_shield_thrust_start(player: ChargeBladePlayer, duration_frames: f32) {
    if unsafe { player.shield_state() } == 2 {
        unsafe { create_timed_on_shield(player, 0x258, 0x259, duration_frames) };
    }
}
