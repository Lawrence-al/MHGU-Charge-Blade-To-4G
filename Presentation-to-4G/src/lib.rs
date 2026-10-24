//! 表现力变 4G：按动作帧提交 SE、创建盾牌特效，并修改加载后的 SRQ／PTS。
//! 游戏对象和原生调用由 cb-game 提供；本 NRO 的缓存、任务及计时独立管理。
#![no_std]

mod action;
mod config;
mod config_file;
#[path = "../../shared/config_io.rs"]
mod config_io;
#[cfg(feature = "debug-logs")]
mod debug;
mod effect;
mod hooks;
mod gaxe;
mod phial;
#[path = "../../shared/patch.rs"]
mod patch;
mod preview;
mod resource;
#[path = "../../shared/runtime_hook.rs"]
mod runtime_hook;
#[path = "../../shared/settings_menu.rs"]
mod settings_menu;
mod sound;
mod state;
mod tasks;
mod ui;

#[forge::entry]
fn main() {
    let config = config_file::initialize();
    let game_base = forge::mem::text_addr();
    state::STATE.initialize(game_base);
    forge::log::prelude::error!("presentation-to-4g: loaded, game base = 0x{game_base:08X}");
    if !config.enabled {
        return;
    }
    hooks::enable(game_base);
    #[cfg(feature = "debug-logs")]
    debug::enable(game_base);
    forge::log::prelude::error!("presentation-to-4g: hooks installed");
}
