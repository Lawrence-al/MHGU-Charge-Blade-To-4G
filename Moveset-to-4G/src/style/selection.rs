//! 4G 是原版菜单第七个选择；引擎继续以公会 0 读取原六套资源。
use core::sync::atomic::{AtomicBool, Ordering};

static SELECTED: AtomicBool = AtomicBool::new(false);

pub fn initialize(selected: bool) {
    SELECTED.store(selected, Ordering::Relaxed);
}
pub fn selected() -> bool {
    SELECTED.load(Ordering::Relaxed)
}

pub fn set_selected(value: bool) {
    if SELECTED.swap(value, Ordering::Relaxed) == value {
        return;
    }
    if let Err(error) = crate::config_file::save_selection() {
        forge::log::prelude::warn!("moveset-to-4g: menu selection could not be saved: {error}");
    }
}
