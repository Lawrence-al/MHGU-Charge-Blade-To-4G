// 新旧实际任务执行器在同一模拟对象字段上执行，对照发声／特效调用与 MSS 拦截。
const {fs,path,root,dir,modulePath,rustPath,run} = require('./host');
const gameModules = require('./game-host');
const effectFile=path.join(dir,'action-effect-host.rs');
fs.writeFileSync(effectFile,fs.readFileSync(path.join(root,'game-api/src/effect.rs'),'utf8') + `
impl From<u32> for crate::player::ChargeBladePlayer {
  fn from(address:u32)->Self {Self::from_address(address)}
}
pub unsafe fn create_timed_on_shield<P:Into<crate::player::ChargeBladePlayer>>(player:P,impact:u16,element:u16,duration:f32)->EffectNode {
  let player=player.into();
  let id=if unsafe{player.phial_kind()}==crate::player::PhialKind::Element {element}else{impact};
  crate::EVENTS.with(|v|v.borrow_mut().push(crate::Event::Effect(player.address(),id,duration)));
  EffectNode::from_address(player.address()+0x10000)
}
`);
const modules=gameModules().replace(modulePath('game-api/src/effect.rs','effect'),`#[path=${rustPath(effectFile)}] pub mod effect;`);
const soundFile=path.join(dir,'action-sound-host.rs');
fs.writeFileSync(soundFile,fs.readFileSync(path.join(root,'game-api/src/sound.rs'),'utf8') +
  '\npub use crate::presentation_sound::play_se_ex;\n');
run('verify-action-runner',`
#![allow(dead_code)]
extern crate self as forge;
use std::{cell::RefCell,collections::BTreeMap};
#[derive(Clone,Debug,PartialEq)] enum Event { Se(u32,u32,i32,u32,u32,i32),Effect(u32,u16,f32) }
thread_local! {
  static FIELDS:RefCell<BTreeMap<u32,u8>>=const {RefCell::new(BTreeMap::new())};
  static EVENTS:RefCell<Vec<Event>>=const {RefCell::new(Vec::new())};
}
#[macro_export] macro_rules! error {($($arg:tt)*)=>{{let _=format!($($arg)*);}};}
pub mod log {pub mod prelude {pub use crate::error;}}
${modules.replace(modulePath('game-api/src/sound.rs','sound'),`#[path=${rustPath(soundFile)}] pub mod sound;`)}
${modulePath('Presentation-to-4G/src/action.rs','action')}
${modulePath('Presentation-to-4G/src/state.rs','state')}
${modulePath('Presentation-to-4G/src/tasks.rs','tasks')}
${modulePath('Presentation-to-4G/src/config.rs','config')}
mod config_file {
  static mut CONFIG: crate::config::Config = crate::config::Config::DEFAULT;
  pub fn get()->&'static crate::config::Config { unsafe { &*(&raw const CONFIG) } }
  pub fn set(c:crate::config::Config) { unsafe { CONFIG=c; } }
}
${modulePath('Presentation-to-4G/src/sound.rs','presentation_sound')}
${modulePath('tests/data/presentation-reference.rs','reference')}
pub use reference::{old_action, constant};
mod runtime {pub fn set_master_player(address:u32) {crate::state::STATE.set_player(crate::player::ChargeBladePlayer::from_address(address));}}
mod memory {
  pub unsafe fn read_u8(address:u32)->u8 {crate::FIELDS.with(|v|v.borrow()[&address])}
  pub unsafe fn read_u16(address:u32)->u16 {u16::from_le_bytes(unsafe{[read_u8(address),read_u8(address+1)]})}
  pub unsafe fn read_u32(address:u32)->u32 {u32::from_le_bytes(core::array::from_fn(|i|unsafe{read_u8(address+i as u32)}))}
  pub unsafe fn read_f32(address:u32)->f32 {f32::from_bits(unsafe{read_u32(address)})}
  pub use read_u8 as load_u8;pub use read_u32 as load_u32;pub use read_f32 as load_f32;
  pub unsafe fn write_u8(address:u32,value:u8) {crate::FIELDS.with(|v|{v.borrow_mut().insert(address,value);});}
  pub unsafe fn write_u32(address:u32,value:u32) {for(i,b)in value.to_le_bytes().into_iter().enumerate(){unsafe{write_u8(address+i as u32,b)};}}
  pub(crate) unsafe fn method<T:Copy>(_:u32,_:u32)->T {unreachable!()}
}
mod native {
  unsafe extern "C" fn play(manager:u32,owner:u32,request:u32,attach:i32,path:u32,pool:u32,bank:i32)->u32 {
    assert_eq!(manager,0x30000000);assert_eq!(pool,4);
    crate::EVENTS.with(|v|v.borrow_mut().push(crate::Event::Se(owner,request,attach,path,pool,bank)));1
  }
  pub unsafe fn function<T:Copy>(offset:u32)->T {assert_eq!(offset,0x004F0BE8);let p=play as *const();unsafe{core::mem::transmute_copy(&p)}}
}
fn take()->Vec<Event>{EVENTS.with(|v|std::mem::take(&mut*v.borrow_mut()))}
fn four_g(bits:u32)->config::FourGEffects {config::FourGEffects{
  aed_explosion:bits&1!=0,saed_explosion:bits&2!=0,phial_attachment:bits&4!=0,
  ed_blast:bits&8!=0,aed_blast:bits&16!=0,saed_blast:bits&32!=0}}
fn main() {
  use player::ChargeBladePlayer;
  state::STATE.initialize(0);
  unsafe{memory::write_u32(0x01896F5C,0x30000000)};
  let mut cases=0;
  for actor in [0x20000000u32,0x21000000] {for element in [0u32,1] {for phial in [0u8,1] {for variant in 0..=32 {
    unsafe{memory::write_u32(actor+0x1280,actor);memory::write_u32(actor+0x13C4,actor+0x10000);
      memory::write_u32(actor+0x10000+0x356,element);memory::write_u8(actor+6,phial);}
    for burst in [false,true] {
      for (stage,frame) in [(0,0.0),(1,1.0),(1,10.0),(2,24.0),(2,26.0),(2,30.0),(1,31.0),(1,40.0),(2,50.0),(1,62.0),(1,66.0),(1,108.0),(1,109.0),(1,120.0),(1,120.0)] {
        unsafe{memory::write_u32(actor+0x500,(frame as f32).to_bits())};
        unsafe{if burst {old_action::on_burst_action_frame(actor,variant,stage)} else {old_action::on_phial_action_frame(actor,variant,stage)}};
        let expected=take();
        unsafe{if burst {action::on_burst_action_frame(ChargeBladePlayer::from_address(actor),variant,stage)} else {action::on_phial_action_frame(ChargeBladePlayer::from_address(actor),variant,stage)}};
        assert_eq!(take(),expected,"actor={actor:X} element={element} phial={phial} variant={variant} burst={burst} stage={stage} frame={frame}");
        cases+=1;
      }
    }
  }}}}
  let sequence=0x24000000;let command=0x25000000;
  for scans in 0..=3u8 {for(opcode,bank,request,frame)in [(0x66u8,0u32,0x43u32,62u32),(0x66,0,0x43,112),(0x66,0,0x4A,24),(0x66,1,0x43,62),(0x67,0,0x43,62)] {
    unsafe{memory::write_u8(command,opcode);memory::write_u8(command+2,scans);
      memory::write_u32(command+4+4*scans as u32,bank|(request<<16));
      memory::write_u32(sequence+0x90,frame);memory::write_u32(sequence+0xB0,frame-1);}
    assert_eq!(unsafe{action::find_block_rule(sound::SoundSequence::from_address(sequence),command as *const u8)},
      unsafe{old_action::find_block_rule(sequence,command as *const u8)});
  }}
  let actor=0x20000000;
  let player=ChargeBladePlayer::from_address(actor);
  let mut c=config::Config::DEFAULT;c.sounds=false;c.action[0].frame=8.0;c.action[0].duration_frames=17.0;
  config_file::set(c);
  unsafe {action::on_phial_action_frame(player,1,0);memory::write_u32(actor+0x500,7.0f32.to_bits());action::on_phial_action_frame(player,1,1);}
  assert!(take().is_empty());
  unsafe {memory::write_u32(actor+0x500,8.0f32.to_bits());action::on_phial_action_frame(player,1,1);}
  assert_eq!(take(),vec![Event::Effect(actor,0x259,17.0)]);
  unsafe {memory::write_u32(actor+0x500,120.0f32.to_bits());action::on_phial_action_frame(player,1,1);}
  assert!(take().is_empty());
  c.action[0].enabled=false;config_file::set(c);
  unsafe {action::on_phial_action_frame(player,1,0);action::on_phial_action_frame(player,1,1);}
  assert!(take().is_empty());
  c=config::Config::DEFAULT;c.effects=false;c.action[2].frame=14.0;c.action[3].enabled=false;c.action[5].enabled=false;config_file::set(c);
  unsafe {action::on_phial_action_frame(player,1,0);memory::write_u32(actor+0x500,13.0f32.to_bits());action::on_phial_action_frame(player,1,1);}
  assert!(take().is_empty());
  unsafe {memory::write_u32(actor+0x500,14.0f32.to_bits());action::on_phial_action_frame(player,1,1);}
  assert_eq!(take(),vec![Event::Se(actor,0x4A,2,0,4,0)]);
  assert_eq!(cases,7920);
  // 六项4G开关的全部组合都不进入GU执行器的资源/音效/时长选择。
  let mut independent=0;
  for bits in 0..64 {for element in [0,1] {for variant in [1,2,7,9] {
    unsafe{memory::write_u32(actor+0x10000+0x356,element);memory::write_u8(actor+6,1);}
    let mut c=config::Config::DEFAULT;c.four_g=four_g(bits);config_file::set(c);
    for burst in [false,true] {
      for (stage,frame) in [(0,0f32),(1,1.),(1,10.),(2,26.),(1,30.),(1,40.),(2,50.),(1,62.),(1,66.),(1,108.),(1,109.),(1,120.)] {
        unsafe{memory::write_u32(actor+0x500,frame.to_bits());
          if burst{old_action::on_burst_action_frame(actor,variant,stage)}else{old_action::on_phial_action_frame(actor,variant,stage)}};
        let expected=take();
        unsafe{if burst{action::on_burst_action_frame(player,variant,stage)}else{action::on_phial_action_frame(player,variant,stage)}};
        assert_eq!(take(),expected,"4G flags={bits} element={element} variant={variant} burst={burst} frame={frame}");
        independent+=1;
      }
    }
  }}}
  for element in [0,1] {for bits in 0..64 {for variant in [1,2,7,9] {
    unsafe{memory::write_u32(actor+0x10000+0x356,element);memory::write_u8(actor+6,1);}
    let mut c=config::Config::DEFAULT;c.sounds=false;c.four_g=four_g(bits);
    c.action[0].duration_frames=17.;c.action[1].duration_frames=19.;
    c.burst[3].duration_frames=23.;c.burst[4].duration_frames=27.;
    config_file::set(c);
    unsafe{action::on_phial_action_frame(player,variant,0);memory::write_u32(actor+0x500,1f32.to_bits());action::on_phial_action_frame(player,variant,1);}
    let saed=variant==2||variant==9;let id=if saed{0x262}else{0x258}+element as u16;
    let duration=if saed{19.}else{17.};
    assert_eq!(take(),vec![Event::Effect(actor,id,duration)]);
    for (v,frame,duration) in [(2,26f32,23f32),(1,50.,27.)] {
      unsafe{action::on_burst_action_frame(player,v,0);memory::write_u32(actor+0x500,frame.to_bits());action::on_burst_action_frame(player,v,2);}
      assert_eq!(take(),vec![Event::Effect(actor,0x258+element as u16,duration)]);
    }
  }}}
  println!("PASS: {cases} original GU action-runner cases plus {independent} six-flag independence cases match; GU IDs, sounds and custom main/preparation durations retained for all64 combinations");
}
`);
