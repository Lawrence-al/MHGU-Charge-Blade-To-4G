// 两个插件都运行真实菜单的编辑／保存流程，运行时配置必须保持启动快照。
const {fs,path,root,dir,modulePath,rustPath,run} = require('./host');
for (const [project, location] of [['Presentation-to-4G','PRESENTATION'], ['Moveset-to-4G','MOVESET']]) {
  const ui = path.join(dir, project + '-ui-host.rs');
  fs.writeFileSync(ui, fs.readFileSync(path.join(root, project, 'src/ui.rs'),'utf8')
    .replace('#[forge::imgui_render]', '').replace('fn draw_settings()', 'pub fn draw_settings()'));
  const presentation = location === 'PRESENTATION';
  run('verify-settings-' + location.toLowerCase(), `
#![allow(dead_code, non_snake_case, non_upper_case_globals)]
extern crate self as forge;
${modulePath(project + '/src/config.rs','config')}
${modulePath(project + '/src/config_file.rs','config_file')}
${modulePath('shared/config_io.rs','config_io')}
${modulePath('shared/settings_menu.rs','settings_menu')}
${presentation ? modulePath('Presentation-to-4G/src/tasks.rs','tasks') : ''}
#[path=${rustPath(ui)}] mod ui;
use config_io::${location} as LOCATION;
${fs.readFileSync(path.join(__dirname,'settings-host.rs'),'utf8')}
fn main() {
    check_reader(); check_storage(); check_roundtrip(config::Config::DEFAULT);
    let mut custom = config::Config::DEFAULT; custom.enabled=false;
    ${presentation ? `
    assert_eq!(core::mem::offset_of!(config::Config, shield_thrust_duration),12);
    custom.sounds=false; custom.patch_srq=false; custom.patch_pts=false; custom.volume_4a=-3.9;
    custom.effects=false; custom.shield_thrust=false; custom.shield_thrust_duration=19.5;
    for t in custom.action.iter_mut().chain(custom.burst.iter_mut()) {t.enabled=false;t.frame+=3.5;if t.duration_frames>0.0 {t.duration_frames=11.5;}}
    for bad in ["[action.17]", "[sounds]\\nvolume_4a=NaN", "[effects]\\nshield_thrust_duration=0", "[burst.0]\\nduration_frames=1", "[action.0]\\nframe=-1"] {assert!(config::Config::parse(bad).is_err(),"{bad}");}
    ` : `
    custom.moveset.yellow_shield_aed=false; custom.moveset.shield_multiplier=false; custom.moveset.gp_recoil=false;
    custom.moveset.j_guard_aed=false; custom.moveset.bushi_shield_thrust_aed=false; custom.moveset.aed_camera_shake=false; custom.moveset.aed_shake_strength=255;
    custom.saed.enabled=false; custom.saed.effect_gear=4; custom.hitdata=false; custom.overpowered=false;
    for bad in ["[saed]\\neffect_gear=0", "[saed]\\nthird_hit=false", "[moveset]\\naed_shake_strength=256"] {assert!(config::Config::parse(bad).is_err());}
    `}
    check_roundtrip(custom);
    // 关闭插件仍能打开菜单；编辑和写盘均不修改当前战斗的配置。
    file_bytes(b"[plugin]\\nenabled=false");
    let running=*config_file::initialize(); assert!(!running.enabled); ui::draw_settings();
    CONTROLS.with(|s| {
        let mut s=s.borrow_mut();
        s.bools.insert("Enable ${presentation?'Presentation':'Moveset'} plugin".into(),true);
        ${presentation ? `
        s.floats.insert("Request 0x4A volume".into(),-3.9);s.floats.insert("Thrust lightning duration".into(),21.0);
        s.floats.insert("action.0/Trigger frame".into(),6.0);s.floats.insert("action.0/Duration frames".into(),17.0);
        s.bools.insert("burst.0/Enable task".into(),false);
        ` : `
        s.ints.insert("SAED effect gear".into(),3);s.ints.insert("AED shake strength".into(),80);
        s.bools.insert("Adept shield thrust X+A routes to AED".into(),false);
        s.bools.insert("4G hitdata".into(),false);
        `}
    });
    ui::draw_settings(); assert_eq!(*config_file::get(),running);
    STORAGE.with(|s|assert_eq!(s.borrow().writes,0));
    click("Save INI");let saved=saved_config();assert!(saved.enabled);
    ${presentation ? `
    assert_eq!(saved.volume_4a,-3.9);assert_eq!(saved.shield_thrust_duration,21.0);
    assert_eq!(saved.action[0].frame,6.0);assert_eq!(saved.action[0].duration_frames,17.0);assert!(!saved.burst[0].enabled);
    ` : `
    assert_eq!(saved.saed.effect_gear,3);assert_eq!(saved.moveset.aed_shake_strength,80);assert!(!saved.hitdata);
    assert!(!saved.moveset.bushi_shield_thrust_aed);assert!(saved.moveset.j_guard_aed);
    `}
    assert_eq!(*config_file::initialize(),running);
    assert_eq!(LOCATION.load(config::Config::DEFAULT,config::Config::parse),saved);
    click("Defaults");click("Discard edits");click("Save INI");assert_eq!(saved_config(),saved);
    click("Defaults");STORAGE.with(|s|s.borrow_mut().write_error=true);click("Save INI");
    CONTROLS.with(|s|assert!(s.borrow().text.last().unwrap().contains("Could not write")));
    click("Discard edits");STORAGE.with(|s|s.borrow_mut().write_error=false);click("Save INI");assert_eq!(saved_config(),saved);
    click("Defaults");click("Save INI");assert_eq!(saved_config(),config::Config::DEFAULT);
    assert_eq!(*config_file::get(),running);
    println!("PASS: ${project} real menu editing/save/defaults/discard/error; INI roundtrip, same SD read/write path, read/write failures and handle cleanup; active config fixed at startup");
}
`);
}
