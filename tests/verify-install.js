// 实际配置解析、2048组选项选择与完整补丁事务／失败恢复。
const {fs, path, root, modulePath, hostBridge, run} = require('./host');
const fixtures = require('./fixtures');
const shared = fs.readFileSync(path.join(root, 'shared/patch.rs'), 'utf8');
const types = shared.slice(shared.indexOf('pub type WordPatch'), shared.indexOf('/// libnx'));
const install = shared.slice(shared.indexOf('pub unsafe fn install'))
  .replace(/ptr::read_volatile\((.+?) as \*const u32\)/g, 'read_main($1)');
const rows = list => list.map(([site, value]) => '(' + site + ',' + value + ')').join(',');
const source = `
#![allow(dead_code, unused_parens)]
extern crate alloc;
use std::{cell::RefCell, collections::BTreeMap, mem, ptr};
mod saed {
    #[path = ${JSON.stringify(path.join(root, 'Moveset-to-4G/src/saed/patches.rs').replaceAll('\\\\', '/'))}] pub mod patches;
}
${modulePath('Moveset-to-4G/src/config.rs', 'config')}
${modulePath('Moveset-to-4G/src/patches.rs', 'patches')}
${hostBridge('Moveset-to-4G/src/moveset.rs', 'moveset')}
${hostBridge('Moveset-to-4G/src/guard.rs', 'guard')}
mod style {
    use cb_game::player::ChargeBladePlayer;
    pub enum Feature { GpRecoil, GuardPerformance, ShieldThrustYellow, GuardRecoilYellow }
    pub unsafe fn enabled_for(_:ChargeBladePlayer,_:Feature)->bool {false}
    static mut SLOTS:[u32;3]=[0;3];
    pub fn collect(v:&mut Vec<crate::patch::BranchPatch>) {
        for (i,(offset,original,second)) in [(0x011D5C64,0xE35000A9,0x8A00030B),(0x0054924C,0xE7D17100,0xE30103C4),(0x00549784,0xE7927101,0xE5901000)].into_iter().enumerate() {
            v.push(crate::patch::BranchPatch {offset,original,second,cheat_call:original,target:0x73300000+i as u32*4,continuation:unsafe{(&raw mut SLOTS).cast::<u32>().add(i)}});
        }
    }
}
mod config_file{pub fn get()->&'static crate::config::Config{&crate::config::Config::DEFAULT}}
thread_local! {
    static MEMORY: RefCell<BTreeMap<u32,u8>> = const { RefCell::new(BTreeMap::new()) };
    static CALLS: RefCell<Vec<(u32,u32)>> = const { RefCell::new(Vec::new()) };
    static LOGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static FAIL_AT: RefCell<usize> = const { RefCell::new(usize::MAX) };
}
macro_rules! error { ($($arg:tt)*) => { LOGS.with(|v| v.borrow_mut().push(format!($($arg)*))) }; }
fn get_bytes(addr:u32,len:usize)->Vec<u8> { MEMORY.with(|v| { let v=v.borrow(); (0..len).map(|i| v[&(addr+i as u32)]).collect() }) }
fn put_bytes(addr:u32,bytes:&[u8]) { MEMORY.with(|v| { let mut v=v.borrow_mut(); for (i,b) in bytes.iter().enumerate() { v.insert(addr+i as u32,*b); } }); }
fn put_word(addr:u32,value:u32) { put_bytes(addr,&value.to_le_bytes()); }
unsafe fn read_main(addr:u32)->u32 { u32::from_le_bytes(get_bytes(addr,4).try_into().unwrap()) }
struct Patch { address:u32, original:Vec<u8>, replacement:Vec<u8>, active:bool }
impl Patch {
    fn new(address:u32,bytes:&[u8],enabled:bool)->Self { assert!(!enabled); Self { address,original:get_bytes(address,bytes.len()),replacement:bytes.to_vec(),active:false } }
    fn addr(&self)->u32 { self.address }
    fn len(&self)->u32 { self.replacement.len() as u32 }
    fn enabled(&self)->bool { self.active }
    fn enable(&mut self) { put_bytes(self.address,&self.replacement); self.active=true; }
    fn disable(&mut self) { if self.active { put_bytes(self.address,&self.original); self.active=false; } }
}
impl Drop for Patch { fn drop(&mut self) { self.disable(); } }
unsafe fn flush_code(addr:u32,len:u32)->u32 {
    let index=CALLS.with(|v| { let mut v=v.borrow_mut(); let n=v.len(); v.push((addr,len)); n });
    FAIL_AT.with(|v| if *v.borrow()==index { 0x1234 } else { 0 })
}
mod patch { use super::*; ${types} ${install} }
const BASE:u32=0x60000000;
fn reset(plan:&patches::Plan,cheated:bool) {
    MEMORY.with(|v| v.borrow_mut().clear()); CALLS.with(|v| v.borrow_mut().clear()); LOGS.with(|v| v.borrow_mut().clear()); FAIL_AT.with(|v| *v.borrow_mut()=usize::MAX);
    for &(site,old,new) in &plan.words { put_word(BASE+site,if cheated { new } else { old }); }
    for branch in &plan.branches {
        put_word(BASE+branch.offset,if cheated { branch.cheat_call } else { branch.original });
        put_word(BASE+branch.offset+4,branch.second);
        unsafe { ptr::write_volatile(branch.continuation,0) };
    }
    for offset in (0x013F1EEC..0x013F1FA0).step_by(4) { put_word(BASE+offset,0xDEADC0DE); }
}
fn snapshot()->BTreeMap<u32,u8> { MEMORY.with(|v| v.borrow().clone()) }
#[test]
fn all_feature_combinations() {
    let groups:&[&[(u32,u32)]]=&[
        &[], &[${rows(fixtures.moveset.slice(7, 9))}], &[${rows(fixtures.moveset.slice(9))}], &[], &[],
        &[${rows(fixtures.saed)}], &[${rows(fixtures.overpowered)}], &[], &[], &[], &[],
    ];
    for mask in 0..2048u16 {
        let on=|bit:usize| mask&(1u16<<bit)!=0;
        let mut c=config::Config::DEFAULT;
        let styles=|on|if on {config::StyleSet::FOUR_G}else{config::StyleSet::NONE};
        c.styles.yellow_shield_aed=styles(on(0)); c.moveset.shield_multiplier=on(1); c.moveset.aed_camera_shake=on(2); c.styles.gp_recoil=styles(on(3)); c.moveset.j_guard_aed=on(4);
        c.styles.shield_thrust_yellow=styles(on(8));
        c.styles.guard_performance=styles(on(9));
        c.styles.guard_recoil_yellow=styles(on(10));
        c.saed.enabled=on(5); c.styles.aed_followups=styles(on(6)); c.hitdata=on(7);
        let p=unsafe { patches::selected(BASE,&c) };
        let expected:Vec<_>=groups.iter().enumerate().filter(|(i,_)| on(*i)).flat_map(|(_,g)| g.iter().copied()).collect();
        assert_eq!(p.words.iter().map(|&(o,_,v)| (o,v)).collect::<Vec<_>>(),expected,"mask={mask}");
        let branches:Vec<_>=[(true,0x011D5C64),(true,0x0054924C),(true,0x00549784),(on(3),0x011DDE74),(on(4)||on(8)||on(10),0x002EA87C),(on(9),0x011DFB0C),(on(9),0x002F0800)].into_iter().filter(|&(enabled,_)| enabled).map(|(_,o)|o).collect();
        assert_eq!(p.branches.iter().map(|b|b.offset).collect::<Vec<_>>(),branches);
        assert!(c.needs_hitdata()); assert_eq!(c.needs_phial(),on(5));
        c.enabled=false;
        let disabled=unsafe { patches::selected(BASE,&c) };
        assert!(disabled.words.is_empty()); assert_eq!(disabled.branches.len(),3); assert!(c.needs_hitdata()&&!c.needs_phial());
    }
}
#[test]
fn actual_ini_defaults_and_parsing() {
    assert_eq!(core::mem::size_of::<config::StyleSet>(),1);
    let text=include_str!(${JSON.stringify(path.join(root, 'conf/cb_moveset_to_4g.ini').replaceAll('\\\\', '/'))});
    assert_eq!(config::Config::parse(text).unwrap(),config::Config::DEFAULT);
    assert!(config::Config::DEFAULT.styles.aed_followups.is_empty());
    assert!(config::Config::DEFAULT.styles.shield_thrust_yellow.is_empty());
    assert!(config::Config::DEFAULT.styles.guard_recoil_yellow.is_empty());
    assert!(config::Config::DEFAULT.moveset.j_guard_aed);
    for id in 0..=255 {
        for (index,set) in [config::Config::DEFAULT.styles.yellow_shield_aed,config::Config::DEFAULT.styles.gp_recoil,config::Config::DEFAULT.styles.guard_performance].into_iter().enumerate() {
            assert_eq!(set.contains(id),id==6||(index==0&&id==3),"AED defaults to Adept/4G; GP and guard remain 4G only");
        }
    }
    assert!(config::Config::parse("[moveset]\\nguard_performance=false").unwrap().styles.guard_performance.is_empty());
    assert!(text.find("[styles]").unwrap()<text.find("[saed]").unwrap());
    assert!(text.find("[moveset]").unwrap()<text.find("[saed]").unwrap());
    assert!(text.find("aed_followups = none").unwrap()<text.find("[saed]").unwrap());
    assert!(text.find("[saed]").unwrap()<text.find("[values]").unwrap());
    assert!(text.find("[values]").unwrap()<text.find("[hitdata]").unwrap());
    let independent=config::Config::parse("[moveset]\\nj_guard_aed=false\\n[styles]\\nshield_thrust_yellow=7").unwrap();
    assert!(!independent.moveset.j_guard_aed);assert!(independent.styles.shield_thrust_yellow.contains(7));
    assert!(text.len()<8192);
    let c=config::Config::parse("\\u{feff}; comment\\r\\n[plugin]\\r\\nenabled=false # no hooks\\r\\n[saed]\\r\\nenabled=false\\r\\neffect_gear=4\\r\\n").unwrap();
    assert!(!c.enabled); assert!(!c.saed.enabled); assert_eq!(c.saed.effect_gear,4);
    assert_eq!(config::Config::parse("[saed]\\neffect_gear=0").unwrap_err().line,2);
    for (text,line) in [("[other]",1),("enabled=false",1),("[saed]\\nenabled=1",2),("[saed]\\neffect_gear=5",2),("[saed]\\neffect_gear=-1",2),("[saed]\\neffect_gear=2.5",2),("[moveset]\\naed_shake_strength=256",2),("[moveset]\\noverpowered=true",2),("[plugin]oops",1)] {
        assert_eq!(config::Config::parse(text).unwrap_err().line,line,"{text}");
    }
    // 移除的键不能再偷偷影响某一子功能，SAED 只有整组开关与特效档位。
    for (key,value) in [("third_hit","true"),("third_hit_motion_value","100"),("third_hit_duration","8"),("phial_damage","true"),("phial_damage_multiplier","1.35"),("phial_sequence","true"),("ground_snap","true"),("effect_override","true"),("impact_first_delay","0"),("impact_interval","9"),("impact_first_forward","270"),("impact_step","100"),("element_first_delay","46"),("element_first_forward","300"),("element_forward_scale","1"),("element_side_scale","1"),("end_delay","9")] {
        assert_eq!(config::Config::parse(&format!("[saed]\\n{key}={value}")).unwrap_err().line,2);
    }
}
#[test]
fn configured_values_and_unified_saed_switch() {
    for gear in 1..=4 {
        let mut c=config::Config::parse(&format!("[moveset]\\naed_shake_strength=31\\n[saed]\\neffect_gear={gear}")).unwrap();
        c.styles.aed_followups=config::StyleSet::ALL;
        let p=unsafe { patches::selected(BASE,&c) };
        let value=|site|p.words.iter().find(|w|w.0==site).unwrap().2;
        assert_eq!(value(0x0048859C),0xE3A01014); assert_eq!(value(0x00489078),0x3FACCCCC); assert_eq!(value(0x011DE830),0xE3A09000|gear);
        c.saed.enabled=false;
        let p=unsafe { patches::selected(BASE,&c) };
        assert_eq!(p.words.len(),3); assert!(!c.needs_phial()); assert!(c.needs_hitdata());
        assert!(p.words.iter().all(|w|!(${JSON.stringify(fixtures.saed.map(w => w[0]))}).contains(&w.0)));
        c.hitdata=false; assert!(c.needs_hitdata());
    }
}
#[test]
fn style_lists_and_legacy_keys_roundtrip() {
    for id in 0..8u16 {
        let mut c=config::Config::DEFAULT;
        c.styles.gp_recoil=config::StyleSet::parse(&format!("all,!{id}")).unwrap();
        c.styles.yellow_shield_aed=config::StyleSet::parse(&format!("{id}")).unwrap();
        assert!(!c.styles.gp_recoil.contains(id));
        assert!(c.styles.yellow_shield_aed.contains(id));
        for other in 0..=255u16 {
            assert_eq!(c.styles.yellow_shield_aed.contains(other),other==id);
            assert_eq!(c.styles.gp_recoil.contains(other),other<8&&other!=id);
        }
        let mut out=String::new();c.write_ini(&mut out).unwrap();
        assert_eq!(config::Config::parse(&out).unwrap(),c);
        assert!(!out.contains("aed_shake_strength"));assert!(!out.contains("[overpowered]"));
    }
    for invalid in ["8","255","256","-1","3-2","all,","!all","1.5","7-12"] {
        assert!(config::Config::parse(&format!("[styles]\\ngp_recoil={invalid}")).is_err());
    }
    let old=config::Config::parse("[overpowered]\\nenabled=true\\n[moveset]\\naed_shake_strength=77").unwrap();
    assert!(old.styles.aed_followups.is_empty());assert!(old.styles.gp_recoil.contains(6));
}
#[test]
fn legacy_masters_migrate_without_overriding_explicit_lists() {
    let sets=|c:config::Config|[c.styles.yellow_shield_aed,c.styles.gp_recoil,c.styles.guard_performance];
    for (i,key) in ["yellow_shield_aed","gp_recoil","guard_performance"].into_iter().enumerate() {
        for value in [false,true] {
            let master=format!("[moveset]\\n{key}={value}\\n");
            let c=config::Config::parse(&master).unwrap();
            assert_eq!(sets(c)[i],if value{sets(config::Config::DEFAULT)[i]}else{config::StyleSet::NONE});
            for list in ["none","6","0,2,4,6","all","all,!3"] {
                let styles=format!("[styles]\\n{key}={list}\\n");
                for text in [format!("{master}{styles}"),format!("{styles}{master}")] {
                    let c=config::Config::parse(&text).unwrap();
                    assert_eq!(sets(c)[i],config::StyleSet::parse(list).unwrap(),"explicit lists win in either section order");
                    let mut out=String::new();c.write_ini(&mut out).unwrap();
                    assert!(!out.contains(&format!("{key} = true")));assert!(!out.contains(&format!("{key} = false")));
                    assert_eq!(config::Config::parse(&out).unwrap(),c);
                }
            }
        }
    }
    let old=config::Config::parse("[moveset]\\nshield_multiplier=false\\naed_camera_shake=false").unwrap();
    let new=config::Config::parse("[values]\\nshield_multiplier=false\\naed_camera_shake=false").unwrap();
    assert_eq!(old,new);let mut out=String::new();old.write_ini(&mut out).unwrap();
    assert!(out.contains("[values]"));assert_eq!(config::Config::parse(&out).unwrap(),new);
    for (old,key) in [("[moveset]\\nbushi_shield_thrust_aed=true\\n","shield_thrust_yellow"),("[aed_followups]\\nenabled=true\\n","aed_followups"),("[overpowered]\\nenabled=true\\n","aed_followups")] {
        let c=config::Config::parse(old).unwrap();assert!(c.styles.shield_thrust_yellow.is_empty()&&c.styles.aed_followups.is_empty());
        for text in [format!("{old}[styles]\\n{key}=7\\n"),format!("[styles]\\n{key}=7\\n{old}")] {
            let c=config::Config::parse(&text).unwrap();let set=if key=="aed_followups"{c.styles.aed_followups}else{c.styles.shield_thrust_yellow};assert_eq!(set,config::StyleSet::parse("7").unwrap());
        }
    }
}
#[test]
fn every_preflight_field_and_cache_failure_rolls_back() {
    let mut c=config::Config::DEFAULT;c.styles.aed_followups=config::StyleSet::ALL; let p=unsafe { patches::selected(BASE,&c) };
    assert_eq!((p.words.len(),p.branches.len()),(24,7));
    let offsets:Vec<_>=p.words.iter().map(|w|w.0).chain(p.branches.iter().flat_map(|b|[b.offset,b.offset+4])).collect();
    for cheated in [false,true] {
        reset(&p,cheated); assert!(unsafe { patches::install(BASE,&c) });
        for &(offset,_,replacement) in &p.words { assert_eq!(unsafe{read_main(BASE+offset)},replacement); }
        for b in &p.branches { assert_eq!(unsafe{read_main(BASE+b.offset)},0xE51FF004); assert_eq!(unsafe{read_main(BASE+b.offset+4)},b.target); assert_eq!(unsafe{ptr::read_volatile(b.continuation)},BASE+b.offset+8); }
        for offset in (0x013F1EEC..0x013F1FA0).step_by(4) { assert_eq!(unsafe{read_main(BASE+offset)},0xDEADC0DE); }
        assert_eq!(CALLS.with(|v|v.borrow().len()),31);
        for failed in 0..31 {
            reset(&p,cheated); FAIL_AT.with(|v|*v.borrow_mut()=failed); let before=snapshot();
            assert!(!unsafe{patches::install(BASE,&c)}); assert_eq!(snapshot(),before); assert_eq!(CALLS.with(|v|v.borrow().len()),(failed+1)*2);
        }
    }
    for offset in offsets { reset(&p,false); put_word(BASE+offset,0xBAD0BAD0); let before=snapshot(); assert!(!unsafe{patches::install(BASE,&c)}); assert_eq!(snapshot(),before); assert!(CALLS.with(|v|v.borrow().is_empty())); }
}
`;
run('verify-config-install', source, ['--test-threads=1'], true);
console.log('PASS: INI/defaults/errors, eight style IDs and out-of-range rejection, fixed AED shake, legacy key migration, 2048 feature combinations and 62 transaction rollback cases; public actions/HDP stay registered');
