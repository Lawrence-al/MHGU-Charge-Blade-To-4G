//! 盾斧派生／超解／攻击数据／超模整合；启动 INI 决定补丁和钩子。
#![no_std]
extern crate alloc;

mod config;
mod config_file;
#[path = "../../shared/config_io.rs"]
mod config_io;
mod hitdata;
mod hooks;
mod moveset;
mod overpowered;
#[path = "../../shared/patch.rs"]
mod patch;
mod patches;
#[path = "../../shared/runtime_hook.rs"]
mod runtime_hook;
mod saed;
#[path = "../../shared/settings_menu.rs"]
mod settings_menu;
mod ui;

#[forge::entry]
fn main() {
    let config = config_file::initialize();
    let game_base = forge::mem::text_addr();
    if !config.enabled {
        forge::log::prelude::error!(
            "moveset-to-4g: disabled by [plugin]; no patches or hooks installed"
        );
        return;
    }
    if unsafe { patches::install(game_base, config) } {
        hooks::enable(game_base, config);
    }
}
