//! 4G盾斧资源常驻加载及GU特效编号映射；动作播放与预览共用同一份资源。
use cb_game::{
    effect::{EffectCreateParams, EffectNode, PelHandle, WeaponEffectShell},
    player::{ChargeBladePlayer, PhialKind},
    resource::{Resource, find_loaded_pel, load_embedded_archive},
};
use forge::os::sync::OnceLock;
use core::ffi::CStr;
use crate::state::STATE;

#[path = "gaxe_catalog.rs"]
mod catalog;
pub use catalog::EFFECTS;

const BYTES: &[u8] = include_bytes!("../../assets/ported/cb_gaxe_preview_4g.arc");
#[repr(align(16))]
struct AlignedArc([u8; BYTES.len()]);
static ARC: AlignedArc = AlignedArc(*include_bytes!("../../assets/ported/cb_gaxe_preview_4g.arc"));

const TAIL_BYTES: &[u8] = include_bytes!("../../assets/ported/cb_gaxe_tails_4g.arc");
#[repr(align(16))]
struct AlignedTails([u8; TAIL_BYTES.len()]);
static TAIL_ARC: AlignedTails = AlignedTails(*include_bytes!("../../assets/ported/cb_gaxe_tails_4g.arc"));

struct Assets {
    _archive: Resource,
    pel: PelHandle,
}
static ASSETS: OnceLock<Option<Assets>> = OnceLock::new();
static TAIL_ASSETS: OnceLock<Option<Assets>> = OnceLock::new();

fn load(bytes: &'static [u8], archive_path: &CStr, pel_path: &CStr) -> Option<Assets> {
    let game = STATE.game();
    let archive = unsafe { load_embedded_archive(game, archive_path, bytes) };
    if archive.address() == 0 {
        forge::log::prelude::error!("[cb4g-gaxe] embedded ARC load failed: {archive_path:?}");
        return None;
    }
    let resource = unsafe { find_loaded_pel(game, pel_path) };
    if resource.address() == 0 {
        forge::log::prelude::error!("[cb4g-gaxe] loaded ARC has no PEL: {pel_path:?}");
        unsafe { archive.release(game) };
        return None;
    }
    let pel = unsafe { PelHandle::register(game, resource) };
    unsafe { resource.release(game) };
    Some(Assets { _archive: archive, pel })
}

pub fn pel() -> Option<PelHandle> {
    ASSETS.get_or_init(|| load(&ARC.0, c"arc\\forge\\cb_gaxe_preview_4g", c"effect\\cb4gp\\gaxe_preview"))
        .as_ref().map(|assets| assets.pel)
}

fn tails() -> Option<PelHandle> {
    // overlay仅含两个新EFL和PEL，模型/贴图依赖由完整包常驻持有。
    let _ = pel()?;
    TAIL_ASSETS.get_or_init(|| load(&TAIL_ARC.0, c"arc\\forge\\cb_gaxe_tails_4g", c"effect\\cb4gt\\gaxe_tails"))
        .as_ref().map(|assets| assets.pel)
}

/// 借用原参数副本，仅覆盖视觉资源、分组和吸地；原栈参数及析构不变。
pub unsafe fn create_visual(
    shell: WeaponEffectShell,
    id: u16,
    mask: u32,
    params: &EffectCreateParams,
) -> Option<EffectNode> {
    let pel = if matches!(id, 0x1005 | 0x1103) { tails()? } else { pel()? };
    let game = STATE.game();
    let mut visual = unsafe { core::ptr::read(params) };
    // create_defined原本在提交前写priority=3，直建PEL时保持相同优先级。
    visual.set_priority(3);
    // 原GU已经给出命中部位/砸地坐标，不允许预览PEL再把世界Y强行改成地面。
    visual.set_ground_snap(false);
    visual.set_unit_mask(mask);
    let node = unsafe { shell.create_from_pel(game, pel, id, &visual) };
    if node.address() == 0 {
        forge::log::prelude::error!("[cb4g-gaxe] create failed: id=0x{id:X} mask=0x{mask:X}");
        None
    } else {
        Some(node)
    }
}

/// 有瓶SAED后段放在原生砸下点；独立世界基点不会再随剑/人物移动。
pub unsafe fn create_world_explosion(player: ChargeBladePlayer, position: [f32; 3], rotation: [f32; 3]) {
    let game = STATE.game();
    let mask = if unsafe { player.phial_kind() } == PhialKind::Element { 2 } else { 1 };
    let mut params = EffectCreateParams::zeroed();
    unsafe { params.construct(game) };
    params.set_priority(3);
    params.place_in_world(position);
    params.set_rotation(rotation);
    let _ = unsafe { create_visual(player.effect_shell(), 0x1103, mask, &params) };
    unsafe { params.destruct(game) };
}
