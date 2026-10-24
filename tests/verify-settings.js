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
// 本夹具只验证持久配置；实时特效菜单由verify-gaxe-preview单独执行验证。
${presentation ? 'mod preview {pub fn draw() {}}' : ''}
${presentation ? '' : `mod style { pub mod selection {
    static SELECTED:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
    pub fn selected()->bool{SELECTED.load(std::sync::atomic::Ordering::Relaxed)}
    pub fn set(v:bool){SELECTED.store(v,std::sync::atomic::Ordering::Relaxed);}
} }`}
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
    assert_eq!(config::Config::DEFAULT.four_g,config::FourGEffects{
        aed_explosion:true,saed_explosion:true,phial_attachment:true,ed_blast:true,aed_blast:true,saed_blast:false});
    for bits in 0..64 {
        let mut c=config::Config::DEFAULT;c.four_g=config::FourGEffects{
            aed_explosion:bits&1!=0,saed_explosion:bits&2!=0,phial_attachment:bits&4!=0,
            ed_blast:bits&8!=0,aed_blast:bits&16!=0,saed_blast:bits&32!=0};
        check_roundtrip(c);
    }
    custom.four_g=config::FourGEffects{aed_explosion:false,saed_explosion:false,phial_attachment:false,
        ed_blast:false,aed_blast:false,saed_blast:false};
    for value in ["true","false"] {
        let text=format!("[effects]\\nuse_4g_sequences={value}\\naed_4g_duration=82.5\\nsaed_4g_duration=0\\n[impacts]\\naed_hit_4g=true\\nsaed_ground_4g=true");
        let legacy=config::Config::parse(&text).unwrap();assert_eq!(legacy,config::Config::DEFAULT);
        let mut serialized=String::new();legacy.write_ini(&mut serialized).unwrap();
        assert!(serialized.contains("[4g_effects]"));
        for old in ["[impacts]","aed_hit_4g","saed_ground_4g","use_4g_sequences","aed_4g_duration","saed_4g_duration"]{assert!(!serialized.contains(old));}
    }
    for key in ["aed_explosion","saed_explosion","phial_attachment","ed_blast","aed_blast","saed_blast"] {
        assert!(config::Config::parse(&format!("[4g_effects]\\n{key}=1")).is_err());
        assert!(config::Config::parse(&format!("[effects]\\n{key}=true")).is_err());
    }
    for t in custom.action.iter_mut().chain(custom.burst.iter_mut()) {t.enabled=false;t.frame+=3.5;if t.duration_frames>0.0 {t.duration_frames=11.5;}}
    for bad in ["[action.17]", "[sounds]\\nvolume_4a=NaN", "[effects]\\nshield_thrust_duration=0", "[burst.0]\\nduration_frames=1", "[action.0]\\nframe=-1", "[action.0]\\nduration_frames=0", "[effects]\\naed_4g_duration=-1", "[effects]\\nsaed_4g_duration=NaN"] {assert!(config::Config::parse(bad).is_err(),"{bad}");}
    ` : `
    custom.styles.yellow_shield_aed=config::StyleSet::NONE; custom.moveset.shield_multiplier=false;
    custom.styles.guard_performance=config::StyleSet::NONE;
    custom.moveset.j_guard_aed=false; custom.moveset.aed_camera_shake=false;
    custom.saed.enabled=false; custom.saed.effect_gear=4; custom.hitdata=false;
    custom.styles.gp_recoil=config::StyleSet::parse("0,6,7").unwrap();
    custom.styles.shield_thrust_yellow=config::StyleSet::parse("1,7").unwrap();
    custom.styles.guard_recoil_yellow=config::StyleSet::parse("2,6").unwrap();
    custom.styles.aed_followups=config::StyleSet::parse("3,5").unwrap();
    for bad in ["[saed]\\neffect_gear=0", "[saed]\\nthird_hit=false", "[moveset]\\naed_shake_strength=256"] {assert!(config::Config::parse(bad).is_err());}
    `}
    check_roundtrip(custom);
    // 关闭插件仍能打开菜单；编辑和写盘均不修改当前战斗的配置。
    file_bytes(b"[plugin]\\nenabled=false");
    let running=*config_file::initialize(); assert!(!running.enabled); ui::draw_settings();
    ${presentation ? `CONTROLS.with(|s|{
        let s=s.borrow();let at=|label:&str|s.drawn.iter().position(|v|v==label).unwrap();
        assert!(at("4G explosions and phial effects")<at("GU lightning"));
        assert!(at("GU lightning")<at("Sound"));
        for(prefix,tasks)in [("action",tasks::ACTION_TASKS),("burst",tasks::BURST_ACTION_TASKS)]{
            for(i,task)in tasks.iter().enumerate(){
                let control=at(&format!("{prefix}.{i}/Enable task"));
                if matches!(&task.action,tasks::TaskAction::PlaySe{..}){assert!(control>at("Sound"));}
                else{assert!(control>at("GU lightning")&&control<at("Sound"));}
            }
        }
        for old in ["4G impact effects","4G AED hit effect (gaxe_002)","4G SAED ground effect (gaxe_104)","Use 4G sequences (sword)","4G AED sequence duration","4G SAED sequence duration"]{assert!(!s.drawn.iter().any(|v|v==old));}
    });` : ''}
    ${presentation ? '' : `
    CONTROLS.with(|s| {
        let s=s.borrow();let at=|label|s.drawn.iter().position(|v|v==label).unwrap();
        assert_eq!(s.ranges["Style ID"],(0,7));
        assert!(at("Moveset")<at("Style ID"));assert!(at("4G guard performance for GP and red shield")<at("SAED"));
        assert!(at("Additional AED follow-ups")<at("SAED"));assert!(at("SAED")<at("Values"));
        assert!(at("Shield thrust X+A to yellow shield")<at("SAED"));
        assert!(at("Guard recoil (small/medium) X+A to yellow shield")<at("SAED"));
        assert!(at("Values")<at("Red / blue shield multiplier 1.2"));assert!(at("Values")<at("4G hitdata"));
        for old in ["Yellow shield routes to AED","GP recoil","4G guard performance (GP / red shield)"] {
            assert!(!s.drawn.iter().any(|v|v==old),"no redundant master checkboxes");
        }
    });
    `}
    CONTROLS.with(|s| {
        let mut s=s.borrow_mut();
        s.bools.insert("Enable ${presentation?'Presentation':'Moveset'} plugin".into(),true);
        ${presentation ? `
        s.floats.insert("Request 0x4A volume".into(),-3.9);s.floats.insert("Thrust lightning duration".into(),21.0);
        s.floats.insert("action.0/Trigger frame".into(),6.0);s.floats.insert("action.0/Duration frames".into(),17.0);
        s.bools.insert("burst.0/Enable task".into(),false);
        for label in ["AED hit / ground explosion (gaxe_005 tail)","SAED ground explosion with phials (gaxe_103 tail)",
            "Phial attachment (gaxe_002)","ED I / II phial bursts (gaxe_003 / 008)",
            "AED phial bursts (gaxe_006 / 009)","SAED phial bursts (gaxe_107)"] {
            s.bools.insert(label.into(),false);
        }
        ` : `
        s.ints.insert("SAED effect gear".into(),3);
        s.ints.insert("Style ID".into(),7);
        s.bools.insert("Use GP recoil in this style".into(),true);
        s.bools.insert("Shield thrust X+A to yellow shield".into(),true);
        s.bools.insert("Guard recoil (small/medium) X+A to yellow shield".into(),true);
        s.bools.insert("Additional AED follow-ups".into(),true);
        s.bools.insert("4G hitdata".into(),false);
        `}
    });
    ui::draw_settings(); assert_eq!(*config_file::get(),running);
    ${presentation ? '' : `
    CONTROLS.with(|s| {
        let mut s=s.borrow_mut();s.ints.insert("Style ID".into(),6);
        s.bools.insert("4G guard performance for GP and red shield".into(),false);
    });
    ui::draw_settings();
    `}
    STORAGE.with(|s|assert_eq!(s.borrow().writes,0));
    click("Save INI");let saved=saved_config();assert!(saved.enabled);
    ${presentation ? `
    assert_eq!(saved.volume_4a,-3.9);assert_eq!(saved.shield_thrust_duration,21.0);
    assert_eq!(saved.action[0].frame,6.0);assert_eq!(saved.action[0].duration_frames,17.0);assert!(!saved.burst[0].enabled);
    assert_eq!(saved.four_g,custom.four_g);
    ` : `
    assert_eq!(saved.saed.effect_gear,3);assert!(!saved.hitdata);
    assert!(saved.styles.gp_recoil.contains(7));
    assert!(saved.styles.gp_recoil.contains(6));
    for set in [saved.styles.shield_thrust_yellow,saved.styles.guard_recoil_yellow,saved.styles.aed_followups]{assert!(set.contains(7));assert!(!set.contains(6));}
    assert!(saved.moveset.j_guard_aed);
    assert!(saved.styles.guard_performance.is_empty());
    `}
    assert_eq!(*config_file::initialize(),running);
    assert_eq!(LOCATION.load(config::Config::DEFAULT,config::Config::parse),saved);
    click("Defaults");click("Discard edits");click("Save INI");assert_eq!(saved_config(),saved);
    click("Defaults");STORAGE.with(|s|s.borrow_mut().write_error=true);click("Save INI");
    CONTROLS.with(|s|assert!(s.borrow().text.last().unwrap().contains("Could not write")));
    click("Discard edits");STORAGE.with(|s|s.borrow_mut().write_error=false);click("Save INI");assert_eq!(saved_config(),saved);
    click("Defaults");click("Save INI");assert_eq!(saved_config(),config::Config::DEFAULT);
    assert_eq!(*config_file::get(),running);
    ${presentation ? '' : `
    // 原版菜单保存选择不能覆盖Forge菜单刚保存、尚待重启的其它选项。
    let before=saved_config(); style::selection::set(true);config_file::save_selection().unwrap();
    let mut expected=before;expected.style_selected=true;assert_eq!(saved_config(),expected);
    let mut stale=before;stale.saed.effect_gear=4;config_file::save(&stale).unwrap();
    expected.saed.effect_gear=4;assert_eq!(saved_config(),expected);
    style::selection::set(false);config_file::save_selection().unwrap();
    expected.style_selected=false;assert_eq!(saved_config(),expected);
    `}
    println!("PASS: ${project} real menu editing/save/defaults/discard/error; INI roundtrip, same SD read/write path, read/write failures and handle cleanup; active config fixed at startup");
}
`);
}
