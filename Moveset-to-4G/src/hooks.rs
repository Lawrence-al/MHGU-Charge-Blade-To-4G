// 唯一的入口钩子安装点；关闭功能时不挂钩，HDP 合并为一次原函数调用。
use crate::{config::Config, config_file, hitdata, overpowered, saed::phial};
use cb_game::{Game, player::ChargeBladePlayer, shell::SaedShell};

crate::cb_hook! { 0x00270C18,
fn hook_hitdata_load(resource: u32, stream: u32) -> u32, |original| {
    let result = unsafe { original(resource, stream) };
    if result != 0 {
        unsafe { hitdata::after_load(resource, config_file::get()) };
    }
    result
}
}

crate::cb_hook! { 0x00488C0C,
fn hook_saed_shell_init(shell: u32, setup: u32) -> u32, |original| {
    let result = unsafe { original(shell, setup) };
    if result == 1 {
        unsafe {
            phial::after_init(
                SaedShell::from_address(shell),
                Game::new(forge::mem::text_addr()),
            )
        };
    }
    result
}
}

crate::cb_hook! { 0x00488DE4,
fn hook_saed_shell_update(shell: u32) -> u32, |original| {
    if let Some(result) = unsafe { phial::on_frame(SaedShell::from_address(shell)) } {
        result
    } else {
        unsafe { original(shell) }
    }
}
}

crate::cb_hook! { 0x00488BDC,
fn hook_saed_shell_destruct(shell: u32) -> u32, |original| {
    phial::forget(SaedShell::from_address(shell));
    unsafe { original(shell) }
}
}

// 原初始化完成后，仅为大解的原链追加后摇命令；原返回寄存器也保留。
crate::cb_hook! { 0x002E9B7C,
fn hook_move_chain_init(player: u32, chain: u32, start_frame: i32) -> u32, |original| {
    let result = unsafe { original(player, chain, start_frame) };
    unsafe { overpowered::after_chain_init(ChargeBladePlayer::from_address(player), chain) };
    result
}
}

pub fn enable(game_base: u32, config: &Config) {
    if config.overpowered {
        overpowered::init();
        forge::install_hook!(game_base, hook_move_chain_init);
    }
    if config.needs_hitdata() {
        forge::install_hook!(game_base, hook_hitdata_load);
    }
    if config.needs_phial() {
        phial::init();
        forge::install_hook!(game_base, hook_saed_shell_init);
        forge::install_hook!(game_base, hook_saed_shell_update);
        forge::install_hook!(game_base, hook_saed_shell_destruct);
    }
    forge::log::prelude::error!(
        "moveset-to-4g: Hitdata={}, SAED={}, effect_gear={}, overpowered={}",
        config.hitdata,
        config.saed.enabled,
        config.saed.effect_gear,
        config.overpowered
    );
    forge::log::prelude::error!(
        "moveset-to-4g: yellow_AED={}, shield_multiplier={}, GP_recoil={}, J_guard_AED={}, Bushi_shield_thrust_AED={}, AED_shake={}/{}",
        config.moveset.yellow_shield_aed,
        config.moveset.shield_multiplier,
        config.moveset.gp_recoil,
        config.moveset.j_guard_aed,
        config.moveset.bushi_shield_thrust_aed,
        config.moveset.aed_camera_shake,
        config.moveset.aed_shake_strength
    );
}
