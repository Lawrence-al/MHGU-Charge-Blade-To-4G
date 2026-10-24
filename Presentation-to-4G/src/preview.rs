//! 4G盾斧特效逐个预览；菜单只排队，创建/停止由本地盾斧的游戏更新执行。
use cb_game::{
    effect::{EffectCreateParams, EffectNode},
    player::ChargeBladePlayer,
};
use core::{cell::UnsafeCell, fmt::Write, sync::atomic::{AtomicBool, AtomicU32, Ordering}};
use forge::{imgui, os::sync::{Mutex, OnceLock, ScopedLock}, sys::imgui::{ImGuiComboFlags, ImGuiInputTextFlags, ImGuiSelectableFlags, ImGuiSliderFlags}};

use crate::{config::ini::Text, effect, gaxe::{self, EFFECTS}, state::STATE};

#[derive(Clone, Copy)]
struct Settings {
    index: usize,
    all_groups: bool,
    group: i32,
    surface: i32,
    shield: bool,
    bone: i32,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
    duration: i32,
    delay: i32,
}

impl Settings {
    const DEFAULT: Self = Self {
        index: 0, all_groups: false, group: 0, surface: 1, shield: false, bone: -1,
        position: [0.0, 40.0, 160.0], rotation: [0.0; 3], scale: 1.0,
        duration: 180, delay: 60,
    };

    fn mask(self) -> u32 {
        if self.all_groups { u32::MAX } else { 1 << self.group }
    }
}

#[derive(Clone, Copy)]
enum Request {
    Play(Settings),
    Stop,
}
struct Panel {
    settings: Settings,
    request: Option<Request>,
    message: &'static str,
}
struct SharedPanel {
    mutex: Mutex,
    inner: UnsafeCell<Panel>,
}
// 菜单与游戏线程均通过with_panel访问；原生调用在解锁后执行。
unsafe impl Send for SharedPanel {}
unsafe impl Sync for SharedPanel {}
static PANEL: OnceLock<SharedPanel> = OnceLock::new();
static PENDING: AtomicBool = AtomicBool::new(false);
static ACTIVE_NODE: AtomicU32 = AtomicU32::new(0);

fn with_panel<R>(f: impl FnOnce(&mut Panel) -> R) -> R {
    let panel = PANEL.get_or_init(|| SharedPanel {
        mutex: Mutex::new(),
        inner: UnsafeCell::new(Panel { settings: Settings::DEFAULT, request: None, message: "" }),
    });
    let _lock = ScopedLock::new(&panel.mutex);
    f(unsafe { &mut *panel.inner.get() })
}

/// 011D3F04是盾斧对象的通用Move更新，站立时也执行。仅有排队请求才读取本地玩家。
pub unsafe fn on_player_frame(player: ChargeBladePlayer) {
    if !PENDING.load(Ordering::Acquire) || player != unsafe { STATE.game().local_player() } {
        return;
    }
    let request = with_panel(|panel| {
        if let Some(Request::Play(settings)) = &mut panel.request {
            if settings.delay > 0 {
                settings.delay -= 1;
                if settings.delay > 0 { return None; }
            }
        }
        PENDING.store(false, Ordering::Release);
        panel.request.take()
    });
    match request {
        Some(Request::Stop) => {
            unsafe { stop_active() };
            with_panel(|panel| panel.message = "Stopped and cleared the preview.");
        }
        Some(Request::Play(settings)) => {
            unsafe { stop_active(); play(player, settings) };
        }
        None => {}
    }
}

unsafe fn stop_active() {
    let node = ACTIVE_NODE.swap(0, Ordering::AcqRel);
    if node != 0 {
        let node = EffectNode::from_address(node);
        effect::forget_lifetime(node);
        unsafe { node.stop_immediately(STATE.game()) };
    }
}

unsafe fn play(player: ChargeBladePlayer, settings: Settings) {
    let Some(pel) = gaxe::pel() else {
        with_panel(|panel| panel.message = "Preview resource load failed; check the game log.");
        forge::log::prelude::error!("[cb4g-preview] embedded resource load failed");
        return;
    };
    let game = STATE.game();
    let (id, name) = EFFECTS[settings.index];
    let mut params = EffectCreateParams::zeroed();
    unsafe { params.construct(game) };
    params.set_priority(3);
    let reference = if settings.shield { unsafe { player.shield().reference() } }
                    else { unsafe { player.reference() } };
    params.attach_to(reference, settings.bone as u16);
    params.set_local_position(settings.position);
    params.set_rotation(settings.rotation);
    params.set_scale([settings.scale; 3]);
    params.set_unit_mask(settings.mask());
    // 地面/水面是第二个mask，不能拿group0/1代替；强制模式同时关闭父对象自动覆盖。
    params.set_auto_surface(settings.surface == 0);
    if settings.surface != 0 {
        params.set_surface_mask(if settings.surface == 1 { cb_game::surface::LAND } else { cb_game::surface::WATER });
    }
    let node = unsafe { player.effect_shell().create_from_pel(game, pel, id, &params) };
    unsafe { params.destruct(game) };
    forge::log::prelude::error!(
        "[cb4g-preview] play {name} id={id} mask=0x{:08X} surface={} shield={} bone={} pos={:?} rot={:?} scale={} duration={} node=0x{:08X}",
        settings.mask(), settings.surface, settings.shield, settings.bone, settings.position,
        settings.rotation, settings.scale, settings.duration, node.address(),
    );
    if node.address() == 0 {
        with_panel(|panel| panel.message = "Effect creation failed; check the game log.");
        return;
    }
    ACTIVE_NODE.store(node.address(), Ordering::Release);
    unsafe { effect::track_lifetime(node, settings.duration as f32) };
    with_panel(|panel| panel.message = "Played. Selected name and group are recorded in the game log.");
}

/// 原生析构发生时撤销节点地址，防止后续Stop误停止复用该地址的其它特效。
pub fn on_destruct(node: u32) {
    let _ = ACTIVE_NODE.compare_exchange(node, 0, Ordering::AcqRel, Ordering::Relaxed);
}

pub fn draw() {
    if !imgui::tree_node(c"4G gaxe effect preview (29)") { return; }
    with_panel(|panel| {
        let settings = &mut panel.settings;
        let mut name = Text::<48>::new();
        write!(name, "{} ({}/29)", EFFECTS[settings.index].1, settings.index + 1).unwrap();
        if imgui::begin_combo(c"Effect file", name.as_c_str(), ImGuiComboFlags::None) {
            for (index, (_, effect)) in EFFECTS.iter().enumerate() {
                let mut label = Text::<32>::new();
                write!(label, "{effect}").unwrap();
                if imgui::selectable(label.as_c_str(), settings.index == index, ImGuiSelectableFlags::None, Default::default()) {
                    settings.index = index;
                }
            }
            imgui::end_combo();
        }
        if imgui::button(c"Previous", Default::default()) {
            settings.index = (settings.index + EFFECTS.len() - 1) % EFFECTS.len();
        }
        imgui::same_line(0.0, -1.0);
        if imgui::button(c"Next", Default::default()) {
            settings.index = (settings.index + 1) % EFFECTS.len();
        }
        imgui::checkbox(c"All groups together", &mut settings.all_groups);
        imgui::slider_int(c"Group bit", &mut settings.group, 0, 31, ImGuiSliderFlags::None);
        let mut mask = Text::<80>::new();
        write!(mask, "Group mask: 0x{:08X}. Bit 0 = 1; bit 1 = 2.", settings.mask()).unwrap();
        imgui::text(mask.as_str());
        if imgui::begin_combo(c"Surface", [c"Auto", c"Ground", c"Water"][settings.surface as usize], ImGuiComboFlags::None) {
            for (value, label) in [c"Auto", c"Ground", c"Water"].iter().enumerate() {
                if imgui::selectable(label, settings.surface == value as i32, ImGuiSelectableFlags::None, Default::default()) {
                    settings.surface = value as i32;
                }
            }
            imgui::end_combo();
        }
        imgui::text("107: group 0 = impact, 1 = last impact; 2-6 = elements, 10-14 = last elements.");
        imgui::text("Surface is independent. Ground / Water overrides the parent surface cache.");
        if imgui::button(c"In front of player", Default::default()) {
            settings.shield = false; settings.bone = -1; settings.position = [0.0, 40.0, 160.0];
        }
        imgui::same_line(0.0, -1.0);
        if imgui::button(c"At player feet", Default::default()) {
            settings.shield = false; settings.bone = -1; settings.position = [0.0; 3];
        }
        imgui::same_line(0.0, -1.0);
        if imgui::button(c"On shield", Default::default()) {
            settings.shield = true; settings.bone = -1; settings.position = [0.0; 3];
        }
        imgui::checkbox(c"Use shield as parent", &mut settings.shield);
        if imgui::input_int(c"Bone (-1 = root)", &mut settings.bone, 1, 10, ImGuiInputTextFlags::None) {
            settings.bone = settings.bone.clamp(-1, 255);
        }
        imgui::input_float3(c"Local position (X/Y/Z)", &mut settings.position, ImGuiInputTextFlags::None);
        imgui::input_float3(c"Local rotation (degrees)", &mut settings.rotation, ImGuiInputTextFlags::None);
        imgui::slider_float(c"Preview scale", &mut settings.scale, 0.1, 10.0, ImGuiSliderFlags::None);
        if imgui::input_int(c"Stop after frames", &mut settings.duration, 30, 60, ImGuiInputTextFlags::None) {
            settings.duration = settings.duration.clamp(1, 3600);
        }
        if imgui::input_int(c"Delay before play", &mut settings.delay, 30, 60, ImGuiInputTextFlags::None) {
            settings.delay = settings.delay.clamp(0, 600);
        }
        if imgui::button(c"Play / Replay", Default::default()) {
            panel.request = Some(Request::Play(*settings));
            panel.message = "Queued. Close Forge menu and keep the charge blade equipped.";
            PENDING.store(true, Ordering::Release);
        }
        imgui::same_line(0.0, -1.0);
        if imgui::button(c"Stop preview", Default::default()) {
            panel.request = Some(Request::Stop);
            panel.message = "Stop queued.";
            PENDING.store(true, Ordering::Release);
        }
        imgui::text("Live preview only: no Save INI or restart needed for these controls.");
        imgui::text("No damage or companion sound. 050/051 are weapon trails; test while moving.");
        imgui::text(panel.message);
    });
    imgui::tree_pop();
}
