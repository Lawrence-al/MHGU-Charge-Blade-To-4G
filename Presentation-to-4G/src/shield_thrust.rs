//! 红盾/蓝盾盾突；视觉可选GU闪电或4G105，声音使用独立4G资源。
use cb_game::{
    effect::EffectCreateParams,
    player::{ChargeBladePlayer, PhialKind},
    resource::{Resource, find_loaded_sound_request, load_embedded_archive},
    sound::play_resource_at,
};
use forge::os::sync::OnceLock;
use crate::{config_file, effect, gaxe, state::STATE};

pub const REQUEST_ID: u16 = 0x55;
const BYTES: &[u8] = include_bytes!("../../assets/ported/cb_shield_thrust_4g.arc");
#[repr(align(16))]
struct AlignedArc([u8; BYTES.len()]);
static ARC: AlignedArc = AlignedArc(*include_bytes!("../../assets/ported/cb_shield_thrust_4g.arc"));

struct Assets {
    _archive: Resource,
    request: Resource,
}
static ASSETS: OnceLock<Option<Assets>> = OnceLock::new();

fn sound_assets() -> Option<Resource> {
    ASSETS.get_or_init(|| {
        let game = STATE.game();
        let archive = unsafe { load_embedded_archive(game, c"arc\\forge\\cb_shield_thrust_4g", &ARC.0) };
        if archive.address() == 0 {
            forge::log::prelude::error!("[cb4g-thrust] sound ARC load failed");
            return None;
        }
        let request = unsafe { find_loaded_sound_request(game, c"sound\\cb4g\\shield_thrust") };
        if request.address() == 0 {
            unsafe { archive.release(game) };
            forge::log::prelude::error!("[cb4g-thrust] sound ARC has no SRQR");
            return None;
        }
        Some(Assets { _archive: archive, request })
    }).as_ref().map(|assets| assets.request)
}

/// 盾斧任务初始化已建立原生资源管理器和武器shell，在此预载，不播放任何事件。
pub fn preload() {
    let config = config_file::get();
    if config.effects && config.shield_thrust_4g { let _ = gaxe::pel(); }
    if config.sounds { let _ = sound_assets(); }
}

/// 原盾突初始化完成后调用一次；红盾/蓝盾空挥也执行，声音和视觉分别服从原有总开关。
pub unsafe fn on_start(player: ChargeBladePlayer) {
    if !matches!(unsafe { player.shield_state() }, 2 | 3) { return; }
    let config = config_file::get();
    let game = STATE.game();
    if config.effects && config.shield_thrust_4g {
        let mask = if unsafe { player.phial_kind() } == PhialKind::Element { 2 } else { 1 };
        let mut params = EffectCreateParams::zeroed();
        unsafe { params.construct(game) };
        params.attach_to(unsafe { player.shield().reference() }, 0);
        let node = unsafe { gaxe::create_visual(player.effect_shell(), 105, mask, &params) };
        unsafe { params.destruct(game) };
        if let Some(node) = node {
            unsafe { effect::track_lifetime(node, config.shield_thrust_duration) };
        }
    } else if config.effects {
        unsafe { effect::create_timed_on_shield(player, 0x258, 0x259, config.shield_thrust_duration) };
    }
    if config.sounds {
        if let Some(request) = sound_assets() {
            unsafe { play_resource_at(game, request, REQUEST_ID, player.position()) };
        }
    }
}
