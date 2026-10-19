// hooks.rs
// SDK 钩子管理 - 与游戏函数的对接（只做转发，逻辑放在各自模块里）

// use forge::log::prelude::error;

use crate::{action, effect, resource};
use cb_game::{effect::EffectNode, player::ChargeBladePlayer, sound::SoundSequence};

// ============================================================================
// 当前启用的钩子
// ============================================================================

// 在原函数建立音库引用前，修改已读入的盾斧 common SRQR。
crate::cb_hook! { 0x00B67AE4,
fn hook_srq_resource_bind(sound_request: u32) -> u32, |original| {
    unsafe { resource::before_srq_bind(sound_request, crate::config_file::get().volume_4a) };
    unsafe { original(sound_request) }
}
}

// PTS 经 XFS 解析后，修改运行时的伴生音效记录；失败时保持原返回值。
crate::cb_hook! { 0x004D923C,
fn hook_pts_resource_load(pel_tied_se: u32, stream: u32) -> u32, |original| {
    let result = unsafe { original(pel_tied_se, stream) };
    if result != 0 {
        unsafe { resource::after_pts_load(pel_tied_se) };
    }
    result
}
}

// SeCommand_Dispatch 钩子（main+0x004D74F8）
//
// 每个 mss 事件都会进这里；记录 SE 归属对象，供 `play_se*` 使用。
// 以后要"删掉游戏自己那一声"，也是在这里命中目标事件时不调用 `original!`。
crate::cb_hook! { 0x004D74F8,
fn hook_se_command_dispatch(
    sequence_slot: u32,
    sound_seq_ctrl: u32,
    command_bytes: *const u8,
) -> u32, |original| {
    if unsafe {
        crate::sound::should_block_event(SoundSequence::from_address(sound_seq_ctrl), command_bytes)
    } {
        return 0;
    }

    unsafe { original(sequence_slot, sound_seq_ctrl, command_bytes) }
}
}

// sub_611DABC8 钩子（main+0x011DABC8）
//
// 盾斧"瓶爆类"动作每帧更新函数，签名 `(player_actor, variant)`。
// 判定与执行都在 [`action::on_phial_action_frame`] 里，这里只读 actionState 后转发。
crate::cb_hook! { 0x011DABC8,
fn hook_cb_phial_action(player_actor: u32, variant: u32) -> u32, |original| {
    let player = ChargeBladePlayer::from_address(player_actor);
    let action_state = unsafe { player.action_state() };
    unsafe { action::on_phial_action_frame(player, variant, action_state) };

    unsafe { original(player_actor, variant) }
}
}

// sub_611DA204 钩子（main+0x011DA204）
//
// 盾斧"瓶爆子动作"的每帧更新函数，签名 `(player_actor, variant)`；
// 判定与执行都在 [`action::on_burst_action_frame`] 里（对应表 [`crate::tasks::BURST_ACTION_TASKS`]）。
crate::cb_hook! { 0x011DA204,
fn hook_cb_burst_action(player_actor: u32, variant: u32) -> u32, |original| {
    let player = ChargeBladePlayer::from_address(player_actor);
    let action_state = unsafe { player.action_state() };
    unsafe { action::on_burst_action_frame(player, variant, action_state) };

    unsafe { original(player_actor, variant) }
}
}

// 盾突起手有红盾就创建闪电；先让原函数初始化动画，再创建盾牌特效。
crate::cb_hook! { 0x011DB280,
fn hook_cb_shield_thrust_action(player_actor: u32, variant: u32) -> u32, |original| {
    let player = ChargeBladePlayer::from_address(player_actor);
    let starting = unsafe { player.action_state() } == 0;
    let result = unsafe { original(player_actor, variant) };
    if starting {
        unsafe { effect::on_shield_thrust_start(player, crate::config_file::get().shield_thrust_duration) };
    }
    result
}
}

// 在特效自己的更新链上累计持续帧数，离开动作后仍继续计时。
crate::cb_hook! { 0x00043224,
fn hook_effect_core_update(node: u32) -> u32, |original| {
    unsafe { effect::on_effect_update(EffectNode::from_address(node)) };
    unsafe { original(node) }
}
}

// 游戏回收节点前撤销记录，避免节点地址复用后匹配到旧动作。
crate::cb_hook! { 0x0004201C,
fn hook_effect_core_destruct(node: u32) -> u32, |original| {
    effect::forget_lifetime(EffectNode::from_address(node));
    unsafe { original(node) }
}
}

// 按需求停用攻击命中清理，保留钩子实现。
/*
// 原生命中计算完成后结束该玩家的盾牌特效；返回值和全部参数原样传递。
#[forge::hook(offset = 0x00170968)]
fn hook_attack_hit_monster(
    hitcheck: u32,
    hit_attack: u32,
    monster: u32,
    contact: u32,
    body_contact: u32,
    attack_contact: u32,
    hit_part: u32,
    shape_index: u32,
) -> u32 {
    let result = unsafe {
        original!(
            hitcheck,
            hit_attack,
            monster,
            contact,
            body_contact,
            attack_contact,
            hit_part,
            shape_index
        )
    };
    unsafe { effect::on_attack_hit(hit_attack) };
    result
}
*/

// ============================================================================
// 已停用的钩子（旧方案：靠 SeState_Update 数帧发声）
// ============================================================================
//
// 旧的"每帧数 tick + 到帧播音效"方案已按需求停用，配置也已清空（见 action.rs 的注释）。
// 如果以后要恢复，把它们从 git 历史里取回来，并在 enable() 里重新安装即可：
//
// #[forge::hook(offset = 0x00281998)]
// fn hook_action_and_efx_init(player_actor: u32, action_key: u32) -> u32 { ... }
//
// #[forge::hook(offset = 0x004D5E90)]
// fn hook_se_state_update(sound_seq_ctrl: u32) -> u32 { ... }

// ============================================================================
// 启用钩子
// ============================================================================

// 安装当前启用的钩子
//
// 参数：`game_base` 游戏模块基址
//
// 注意：#\[forge::hook\] 的 offset 必须是字面量，因此入口偏移直接写在钩子属性中；
// 游戏对象字段和原生调用分别由 cb-game 的对应对象管理。
pub fn enable(game_base: u32) {
    let config = crate::config_file::get();
    if config.patch_srq { forge::install_hook!(game_base, hook_srq_resource_bind); }
    if config.patch_pts { forge::install_hook!(game_base, hook_pts_resource_load); }
    if config.sounds { forge::install_hook!(game_base, hook_se_command_dispatch); }
    if config.sounds || config.effects {
        forge::install_hook!(game_base, hook_cb_phial_action);
        forge::install_hook!(game_base, hook_cb_burst_action);
    }
    if config.effects {
        effect::init_lifetimes();
        forge::install_hook!(game_base, hook_effect_core_update);
        forge::install_hook!(game_base, hook_effect_core_destruct);
        if config.shield_thrust { forge::install_hook!(game_base, hook_cb_shield_thrust_action); }
    }
    // forge::install_hook!(game_base, hook_attack_hit_monster);
}
