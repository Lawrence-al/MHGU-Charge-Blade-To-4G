use crate::{
    config::Config,
    config_file, config_io,
    settings_menu::{self, Menu},
};
use forge::{imgui, os::sync::OnceLock, sys::imgui::ImGuiSliderFlags};

static MENU: OnceLock<Menu<Config>> = OnceLock::new();

#[forge::imgui_render]
fn draw_settings() {
    imgui::push_id(imgui::Id::Str(c"cb_moveset_to_4g"));
    if imgui::tree_node(c"CB Moveset to 4G") {
        MENU.get_or_init(|| Menu::new(*config_file::get()))
            .with(|edit| {
                let c = &mut edit.draft;
                imgui::checkbox(c"Enable Moveset plugin", &mut c.enabled);
                imgui::separator_text(c"Moveset");
                imgui::checkbox(
                    c"Yellow shield routes to AED",
                    &mut c.moveset.yellow_shield_aed,
                );
                imgui::checkbox(
                    c"Red / blue shield multiplier 1.2",
                    &mut c.moveset.shield_multiplier,
                );
                imgui::checkbox(
                    c"GP recoil for Guild / Aerial / Alchemy",
                    &mut c.moveset.gp_recoil,
                );
                imgui::checkbox(c"Adept guard X+A routes to AED", &mut c.moveset.j_guard_aed);
                imgui::checkbox(
                    c"Adept shield thrust X+A routes to AED",
                    &mut c.moveset.bushi_shield_thrust_aed,
                );
                imgui::checkbox(c"AED phial camera shake", &mut c.moveset.aed_camera_shake);
                let mut strength = i32::from(c.moveset.aed_shake_strength);
                if imgui::slider_int(
                    c"AED shake strength",
                    &mut strength,
                    0,
                    255,
                    ImGuiSliderFlags::None,
                ) {
                    c.moveset.aed_shake_strength = strength as u8;
                }
                imgui::separator_text(c"SAED");
                imgui::checkbox(c"Enable 4G SAED", &mut c.saed.enabled);
                let mut gear = i32::from(c.saed.effect_gear);
                if imgui::slider_int(c"SAED effect gear", &mut gear, 1, 4, ImGuiSliderFlags::None) {
                    c.saed.effect_gear = gear as u8;
                }
                imgui::checkbox(c"4G hitdata", &mut c.hitdata);
                imgui::checkbox(c"Overpowered AED recovery", &mut c.overpowered);
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
