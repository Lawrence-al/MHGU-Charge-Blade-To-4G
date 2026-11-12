//! SE 提交和 MSS 拦截规则；原生音序字段及播放池调用在 cb-game。
use crate::{action, state::STATE};
use cb_game::{
    player::ChargeBladePlayer,
    sound::{HitSoundSource, MssEvent, SeRequest, SoundSequence, submit_common_hit},
};

/// 仅001808CC桥接的公共血声层进入这里；保留D8分支落点，安装时判断独立开关。
/// 按直接来源的Shell002/003识别盾斧瓶爆，支持玩家动作结束后的迟发命中。
pub unsafe extern "C" fn submit_hit_blood(
    manager: u32, request: u32, position: u32, room: u32, attack: u32,
) -> u32 {
    let game = STATE.game();
    if (0x82..=0x84).contains(&request)
        && unsafe { HitSoundSource::from_attack(attack).is_charge_blade_phial(game) }
    {
        return 0;
    }
    unsafe { submit_common_hit(game, manager, request, position, room) }
}

pub unsafe fn play_se_ex(request_id: u16, bank: u16, attach_index: i32, pool_path: u32) -> bool {
    let owner = unsafe { STATE.resolve_owner() };
    if owner.address() == 0 {
        return false;
    }
    let played = unsafe {
        owner.play(
            STATE.game(),
            SeRequest {
                request_id,
                bank,
                attach_index,
                pool_path,
            },
        )
    };
    if played {
        STATE.count_play();
    }
    played
}

/// 记录 MSS 发声归属，命中规则时由入口钩子跳过当前事件。
pub unsafe fn should_block_event(sequence: SoundSequence, command_bytes: *const u8) -> bool {
    let owner = unsafe { sequence.owner() };
    if owner.address() != 0 {
        STATE.observe_sequence(sequence, owner);
        let is_player = unsafe { owner.is_charge_blade_player(STATE.game()) };
        if is_player {
            STATE.set_owner(owner);
            if !command_bytes.is_null() {
                let event = MssEvent::from_address(command_bytes as u32);
                // 盾突本体0x1400的bank0请求0x44对应gaxe_imp。
                if unsafe { event.opcode() } == 0x66
                    && unsafe { event.bank() } == 0
                    && unsafe { event.request_id() } == 0x44
                    && unsafe { ChargeBladePlayer::from_address(owner.address()).animation_id() } == 0x1400
                {
                    return true;
                }
            }
        }
        if is_player && unsafe { action::find_block_rule(sequence, command_bytes) }.is_some() {
            return true;
        }
    }
    false
}
