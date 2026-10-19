// 执行实际盾突条件、瓶型读取及选择函数；仅替换粒子创建和持续时间登记。
const {fs,path,root,dir,modulePath,rustPath,run} = require('./host');
const gameModules = require('./game-host');
module.exports = function verifyStart() {
  const source = fs.readFileSync(path.join(root,'Presentation-to-4G/src/effect.rs'),'utf8');
  const timed = source.slice(source.indexOf('pub unsafe fn create_timed_on_shield('),source.indexOf('/// 创建 UNIQUE 特效'));
  const shield = source.slice(source.indexOf('pub unsafe fn on_shield_thrust_start('));
  const effectFile=path.join(dir,'effect-selector-host.rs');
  fs.writeFileSync(effectFile,fs.readFileSync(path.join(root,'game-api/src/effect.rs'),'utf8') + `
use crate::player::{ChargeBladePlayer,PhialKind};
${timed}
${shield}
unsafe fn create_on_shield(player:ChargeBladePlayer,id:u16)->EffectNode {
  crate::EVENTS.lock().unwrap().push((player.address(),id,0.0));
  EffectNode::from_address(player.address()+0x10000)
}
unsafe fn track_lifetime(node:EffectNode,duration:f32) {
  let mut events=crate::EVENTS.lock().unwrap();
  assert_eq!(events.last().unwrap().0+0x10000,node.address());
  events.last_mut().unwrap().2=duration;
}
`);
  const modules=gameModules().replace(modulePath('game-api/src/effect.rs','effect'),`#[path=${rustPath(effectFile)}] pub mod effect;`);
  run('verify-shield-selector',`
#![allow(dead_code)]
use std::{sync::{Mutex,LazyLock},collections::BTreeMap};
static FIELDS:LazyLock<Mutex<BTreeMap<u32,u32>>>=LazyLock::new(||Mutex::new(BTreeMap::new()));
static EVENTS:LazyLock<Mutex<Vec<(u32,u16,f32)>>>=LazyLock::new(||Mutex::new(Vec::new()));
${modules}
mod memory {
  pub unsafe fn read_u32(address:u32)->u32 {crate::FIELDS.lock().unwrap()[&address]}
  pub unsafe fn read_u16(address:u32)->u16 {unsafe{read_u32(address) as u16}}
  pub unsafe fn read_u8(address:u32)->u8 {unsafe{read_u32(address) as u8}}
  pub unsafe fn read_f32(address:u32)->f32 {f32::from_bits(unsafe{read_u32(address)})}
  pub use read_u32 as load_u32;pub use read_u8 as load_u8;pub use read_f32 as load_f32;
  pub unsafe fn write_u32(_:u32,_:u32) {unreachable!()}
  pub unsafe fn write_u8(_:u32,_:u8) {unreachable!()}
  pub(crate) unsafe fn method<T:Copy>(_:u32,_:u32)->T {unreachable!()}
}
mod native {pub unsafe fn function<T:Copy>(_:u32)->T {unreachable!()}}
fn main() {
  let mut count=0;
  for actor in [0x20000000,0x22000000] {for shield in [0,1,2,3] {for element in [false,true] {
    let equip=actor+0x10000;
    {let mut fields=FIELDS.lock().unwrap();fields.insert(actor+0x3344,shield);
      fields.insert(actor+0x13C4,equip);fields.insert(equip+0x356,u32::from(element));}
    EVENTS.lock().unwrap().clear();
    unsafe {effect::on_shield_thrust_start(player::ChargeBladePlayer::from_address(actor),30.0)};
    assert_eq!(*EVENTS.lock().unwrap(),if shield==2 {vec![(actor,if element {0x259}else{0x258},30.0)]} else {vec![]});
    count+=1;
  }}}
  println!("PASS: {count} actual typed player/equipment/particle selector cases; red shield only, impact/element, 30-frame duration");
}
`);
};
if(require.main===module) module.exports();
