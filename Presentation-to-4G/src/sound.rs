//! SE 提交和 MSS 拦截规则；原生音序字段及播放池调用在 cb-game。
use crate::{action, state::STATE};
use cb_game::sound::{SeRequest, SoundSequence};

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
        }
        if is_player && unsafe { action::find_block_rule(sequence, command_bytes) }.is_some() {
            return true;
        }
    }
    false
}
