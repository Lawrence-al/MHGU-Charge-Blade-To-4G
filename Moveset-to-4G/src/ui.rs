use crate::{
    config::Config,
    config_file, config_io,
    settings_menu::{self, Menu},
};
use forge::{imgui, os::sync::OnceLock, sys::imgui::ImGuiSliderFlags};

static MENU: OnceLock<Menu<Config>> = OnceLock::new();
static STYLE_ID: OnceLock<core::sync::atomic::AtomicI32> = OnceLock::new();

#[forge::imgui_render]
fn draw_settings() {
    imgui::push_id(imgui::Id::Str(c"cb_moveset_to_4g"));
    if imgui::tree_node(c"CB Moveset to 4G") {
        MENU.get_or_init(|| Menu::new(*config_file::get()))
            .with(|edit| {
                let c = &mut edit.draft;
                imgui::checkbox(c"Enable Moveset plugin", &mut c.enabled);
                imgui::separator_text(c"Moveset");
                use core::sync::atomic::Ordering;
                let selected = STYLE_ID.get_or_init(|| core::sync::atomic::AtomicI32::new(6));
                let mut id = selected.load(Ordering::Relaxed);
                if imgui::slider_int(c"Style ID", &mut id, 0, 7, ImGuiSliderFlags::None) {
                    selected.store(id, Ordering::Relaxed);
                }
                imgui::text(match id {
                    0 => "0: Guild",
                    1 => "1: Striker",
                    2 => "2: Aerial",
                    3 => "3: Adept",
                    4 => "4: Alchemy",
                    5 => "5: Valor",
                    6 => "6: 4G",
                    _ => "7: Reserved",
                });
                for (label, set) in [
                    (
                        c"Yellow shield becomes normal AED",
                        &mut c.styles.yellow_shield_aed,
                    ),
                    (c"Use GP recoil in this style", &mut c.styles.gp_recoil),
                    (
                        c"4G guard performance for GP and red shield",
                        &mut c.styles.guard_performance,
                    ),
                    (
                        c"Shield thrust X+A to yellow shield",
                        &mut c.styles.shield_thrust_yellow,
                    ),
                    (
                        c"Guard recoil (small/medium) X+A to yellow shield",
                        &mut c.styles.guard_recoil_yellow,
                    ),
                    (c"Additional AED follow-ups", &mut c.styles.aed_followups),
                ] {
                    let mut enabled = set.contains(id as u16);
                    if imgui::checkbox(label, &mut enabled) {
                        set.set(id as u8, enabled);
                    }
                }
                imgui::text("All six settings apply to the selected style (0-7).");
                imgui::text("Defaults: yellow shield to AED in styles 3 and 6.");
                imgui::text("GP recoil and guard in style 6; the other three settings are off.");
                imgui::text(
                    "Yellow-shield routes need 'Yellow shield becomes normal AED' for normal AED.",
                );
                imgui::checkbox(c"J guard X+A to yellow shield", &mut c.moveset.j_guard_aed);
                imgui::separator_text(c"SAED");
                imgui::checkbox(c"Enable 4G SAED", &mut c.saed.enabled);
                let mut gear = i32::from(c.saed.effect_gear);
                if imgui::slider_int(c"SAED effect gear", &mut gear, 1, 4, ImGuiSliderFlags::None) {
                    c.saed.effect_gear = gear as u8;
                }
                imgui::separator_text(c"Values");
                imgui::checkbox(
                    c"Red / blue shield multiplier 1.2",
                    &mut c.moveset.shield_multiplier,
                );
                imgui::checkbox(c"AED phial camera shake", &mut c.moveset.aed_camera_shake);
                imgui::checkbox(c"4G hitdata", &mut c.hitdata);
                settings_menu::footer(
                    edit,
                    Config::DEFAULT,
                    &config_io::MOVESET,
                    config_file::save,
                );
            });
        imgui::tree_pop();
    }
    imgui::pop_id();
}
