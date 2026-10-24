//! 共用动作和菜单始终安装；六项派生／防御修改按当前风格查询固定配置。
pub mod actions;
pub mod menu;
pub mod selection;

use cb_game::{Game, player::ChargeBladePlayer};

pub const FOUR_G: u16 = 6;

#[derive(Clone, Copy)]
pub enum Feature {
    YellowShieldAed,
    GpRecoil,
    GuardPerformance,
    ShieldThrustYellow,
    GuardRecoilYellow,
    AedFollowups,
}

/// 4G 菜单选择在本机公会资源上运行；其余编号按装备原字段完整读取。
pub unsafe fn style_id(player: ChargeBladePlayer) -> u16 {
    let equipment = unsafe { player.equipment() };
    let actual = unsafe { equipment.style() } as u16;
    if actual == 0
        && selection::selected()
        && unsafe { Game::new(crate::game_base()).local_player() == player }
    {
        FOUR_G
    } else {
        actual
    }
}

pub unsafe fn enabled_for(player: ChargeBladePlayer, feature: Feature) -> bool {
    let config = crate::config_file::get();
    if !config.enabled || unsafe { player.equipment().weapon_type() } != 14 {
        return false;
    }
    let id = unsafe { style_id(player) };
    match feature {
        Feature::YellowShieldAed => config.styles.yellow_shield_aed.contains(id),
        Feature::GpRecoil => config.styles.gp_recoil.contains(id),
        Feature::GuardPerformance => config.styles.guard_performance.contains(id),
        Feature::ShieldThrustYellow => config.styles.shield_thrust_yellow.contains(id),
        Feature::GuardRecoilYellow => config.styles.guard_recoil_yellow.contains(id),
        Feature::AedFollowups => config.styles.aed_followups.contains(id),
    }
}

pub fn initialize(selected: bool) {
    selection::initialize(selected);
}

pub fn collect(branches: &mut alloc::vec::Vec<crate::patch::BranchPatch>) {
    actions::collect(branches);
    menu::collect(branches);
}

pub fn enable(base: u32) {
    actions::enable(base);
    menu::enable(base);
}
