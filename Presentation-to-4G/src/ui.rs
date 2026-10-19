use crate::{
    config::{Config, TaskSettings, ini::Text},
    config_file, config_io,
    settings_menu::{self, Menu},
    tasks::{ACTION_TASKS, ActionTask, BURST_ACTION_TASKS, TaskAction},
};
use core::{ffi::CStr, fmt::Write};
use forge::{
    imgui,
    os::sync::OnceLock,
    sys::imgui::{ImGuiInputTextFlags, ImGuiSliderFlags},
};

static MENU: OnceLock<Menu<Config>> = OnceLock::new();

#[forge::imgui_render]
fn draw_settings() {
    imgui::push_id(imgui::Id::Str(c"cb_presentation_to_4g"));
    if imgui::tree_node(c"CB Presentation to 4G") {
        MENU.get_or_init(|| Menu::new(*config_file::get()))
            .with(|edit| {
                let c = &mut edit.draft;
                imgui::checkbox(c"Enable Presentation plugin", &mut c.enabled);
                imgui::separator_text(c"Sound");
                imgui::checkbox(
                    c"Custom timed sounds / block duplicate MSS sounds",
                    &mut c.sounds,
                );
                imgui::checkbox(c"Patch common SRQ", &mut c.patch_srq);
                imgui::checkbox(c"Patch effect companion sounds (PTS)", &mut c.patch_pts);
                imgui::slider_float(
                    c"Request 0x4A volume",
                    &mut c.volume_4a,
                    -96.0,
                    12.0,
                    ImGuiSliderFlags::None,
                );
                imgui::separator_text(c"Shield lightning");
                imgui::checkbox(c"Enable shield lightning", &mut c.effects);
                imgui::checkbox(c"Red-shield thrust lightning", &mut c.shield_thrust);
                frame_input(
                    c"Thrust lightning duration",
                    &mut c.shield_thrust_duration,
                    0.1,
                );
                imgui::text(
                    "Effect IDs and attachment stay on the shield, selected by phial type.",
                );
                task_group(
                    c"AED / SAED action tasks",
                    "action",
                    ACTION_TASKS,
                    &mut c.action,
                );
                task_group(
                    c"AED preparation tasks",
                    "burst",
                    BURST_ACTION_TASKS,
                    &mut c.burst,
                );
                settings_menu::footer(
                    edit,
                    Config::DEFAULT,
                    &config_io::PRESENTATION,
                    config_file::save,
                );
            });
        imgui::tree_pop();
    }
    imgui::pop_id();
}

fn frame_input(label: &CStr, value: &mut f32, min: f32) {
    if imgui::input_float(label, value, 1.0, 10.0, ImGuiInputTextFlags::None) {
        *value = value.clamp(min, 10000.0);
    }
}

fn task_group(label: &CStr, prefix: &str, tasks: &[ActionTask], values: &mut [TaskSettings]) {
    if !imgui::tree_node(label) {
        return;
    }
    for (i, (task, value)) in tasks.iter().zip(values).enumerate() {
        let mut title = Text::<128>::new();
        match &task.action {
            TaskAction::PlaySe { request_id, .. } => {
                write!(title, "{prefix}.{i}: SE 0x{request_id:02X}").unwrap()
            }
            TaskAction::CreateShieldEffect {
                effect_id,
                element_effect_id,
                ..
            } => write!(
                title,
                "{prefix}.{i}: lightning 0x{effect_id:X} / 0x{element_effect_id:X}"
            )
            .unwrap(),
        }
        if imgui::tree_node(title.as_c_str()) {
            let mut details = Text::<128>::new();
            write!(
                details,
                "actionState={}  variant mask=0x{:X}",
                task.action_state, task.variant_mask
            )
            .unwrap();
            imgui::text(details.as_str());
            imgui::checkbox(c"Enable task", &mut value.enabled);
            frame_input(c"Trigger frame", &mut value.frame, 0.0);
            if matches!(&task.action, TaskAction::CreateShieldEffect { .. }) {
                frame_input(c"Duration frames", &mut value.duration_frames, 0.1);
            }
            imgui::tree_pop();
        }
    }
    imgui::tree_pop();
}
