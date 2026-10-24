//! 盾斧风格／派生／超解／攻击数据整合；公共资源常驻，战斗功能读取启动INI。
#![no_std]
extern crate alloc;

mod config;
mod config_file;
#[path = "../../shared/config_io.rs"]
mod config_io;
mod guard;
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
mod style;
#[path = "../../shared/settings_menu.rs"]
mod settings_menu;
mod ui;

use core::sync::atomic::{AtomicU32, Ordering};
static GAME_BASE: AtomicU32 = AtomicU32::new(0);
pub fn game_base() -> u32 { GAME_BASE.load(Ordering::Relaxed) }

#[forge::entry]
fn main() {
    let config = config_file::initialize();
    let game_base = forge::mem::text_addr();
    GAME_BASE.store(game_base, Ordering::Relaxed);
    style::initialize(config.style_selected);
    if unsafe { patches::install(game_base, config) } {
        hooks::enable(game_base, config);
    }
}
