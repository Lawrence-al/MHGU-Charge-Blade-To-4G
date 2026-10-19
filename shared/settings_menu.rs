//! 编辑草稿与战斗中生效的不可变配置分开；只在点击保存时写存档根目录。
use crate::config_io::Location;
use core::cell::UnsafeCell;
use forge::{
    imgui,
    os::sync::{Mutex, ScopedLock},
};

pub struct Menu<C> {
    lock: Mutex,
    state: UnsafeCell<Edit<C>>,
}
unsafe impl<C: Send> Send for Menu<C> {}
unsafe impl<C: Send> Sync for Menu<C> {}

pub struct Edit<C> {
    pub draft: C,
    pub saved: C,
    pub message: &'static str,
}
impl<C: Copy> Menu<C> {
    pub fn new(value: C) -> Self {
        Self {
            lock: Mutex::new(),
            state: UnsafeCell::new(Edit {
                draft: value,
                saved: value,
                message: "",
            }),
        }
    }
    pub fn with<R>(&self, f: impl FnOnce(&mut Edit<C>) -> R) -> R {
        let _lock = ScopedLock::new(&self.lock);
        f(unsafe { &mut *self.state.get() })
    }
}

pub fn footer<C: Copy + PartialEq>(
    edit: &mut Edit<C>,
    defaults: C,
    location: &Location,
    save: impl FnOnce(&C) -> Result<(), &'static str>,
) {
    imgui::separator();
    imgui::text("Save changes to INI, then restart the game to apply.");
    imgui::text(location.display);
    if edit.draft != edit.saved {
        imgui::text("Unsaved changes.");
    }
    if imgui::button(c"Save INI", Default::default()) {
        edit.message = match save(&edit.draft) {
            Ok(()) => {
                edit.saved = edit.draft;
                "Saved. Restart the game to apply."
            }
            Err(message) => message,
        };
    }
    imgui::same_line(0.0, -1.0);
    if imgui::button(c"Discard edits", Default::default()) {
        edit.draft = edit.saved;
        edit.message = "Edits discarded.";
    }
    imgui::same_line(0.0, -1.0);
    if imgui::button(c"Defaults", Default::default()) {
        edit.draft = defaults;
        edit.message = "Defaults selected. Save INI to keep them.";
    }
    imgui::text(edit.message);
}
