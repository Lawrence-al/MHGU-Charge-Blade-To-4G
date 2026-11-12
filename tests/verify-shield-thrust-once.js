// 实际盾突模块和资源/声音API的主机回归；只替换原生函数。
const {fs,path,root,dir,modulePath,rustPath,run} = require('./host');
const gameModules = require('./game-host');
module.exports = function verifyAssets() {
  const assert=require('assert/strict'),zlib=require('zlib'),crypto=require('crypto');
  const file=path.join(root,'assets/ported/cb_shield_thrust_4g.arc');
  const archive=fs.readFileSync(file),manifest=JSON.parse(fs.readFileSync(file.replace('.arc','.manifest.json')));
  const digest=data=>crypto.createHash('sha256').update(data).digest('hex');
  assert.equal(digest(archive),manifest.archive.sha256);assert.equal(archive.readUInt16LE(6),3);
  const resources=new Map(),soundPath='sound\\cb4g\\shield_thrust';
  for(let i=0;i<3;i++){
    const at=12+80*i,name=archive.subarray(at,at+64).toString().split('\0')[0];
    const kind=archive.readUInt32LE(at+64),offset=archive.readUInt32LE(at+76),length=archive.readUInt32LE(at+68);
    const data=zlib.inflateSync(archive.subarray(offset,offset+length));
    assert.equal(name,soundPath);assert.equal(digest(data),manifest.entries[i].sha256);
    resources.set(kind,data);
  }
  assert.deepEqual([...resources.keys()],[0x79C47B59,0x15D782FB,0x1BCC4966]);
  const request=resources.get(0x1BCC4966),bank=resources.get(0x15D782FB),wave=resources.get(0x79C47B59);
  assert.equal(request.toString('ascii',0,4),'SRQR');assert.equal(request.readUInt32LE(8),1);
  assert.deepEqual([...request.subarray(0x34,0x38)],[0x55,0,0x55,0x7F]);
  assert.equal(request.readUInt16LE(0x34+0x1E),0);assert.equal(request.readUInt32LE(0x34+8),1);
  assert.equal(manifest.volume_boost_db,6);
  assert.equal(request.readFloatLE(0x34+0x24),4);assert.equal(request.readFloatLE(0x34+0x28),4);
  const bankPath=request.readUInt32LE(request.readUInt32LE(0x20));
  assert.equal(request.toString('ascii',bankPath,request.indexOf(0,bankPath)),soundPath);
  assert.equal(bank.toString('ascii',0,4),'SBKR');assert.equal(bank.readUInt32LE(8),1);assert.equal(bank.readUInt32LE(12),1);
  assert.equal(bank.readUInt16LE(0x14),0);assert.equal(bank.readUInt16LE(0x16),0x0802);
  const end=bank.indexOf(0,0x20),params=end+5;
  assert.equal(bank.toString('ascii',0x20,end),soundPath);assert.equal(bank.readUInt32LE(end+1),0x79C47B59);
  assert.deepEqual([...bank.subarray(params+71,params+75)],[0x55,0x55,0x55,0]);
  assert.equal(wave.readUInt32LE(0),2);assert.equal(wave.readUInt32LE(0x60),18200);assert.equal(wave.readUInt32LE(0x64),16000);
  console.log('PASS: isolated three-resource ARC closure, manifest hashes, SRQR request 0x55 / cue 0 / key 0x55 / +6dB boost, SBKR sample selector and GU ADPCM header');
  const source=fs.readFileSync(path.join(root,'Presentation-to-4G/src/shield_thrust.rs'),'utf8')
    .replace(/include_bytes!\("([^"\n]+)"\)/g,(_,file)=>`include_bytes!(${rustPath(path.resolve(root,'Presentation-to-4G/src',file))})`);
  const shieldFile=path.join(dir,'shield-thrust-host.rs');fs.writeFileSync(shieldFile,source);
  const effectFile=path.join(dir,'thrust-effect-host.rs');
  fs.writeFileSync(effectFile,fs.readFileSync(path.join(root,'game-api/src/effect.rs'),'utf8')+
    '\npub unsafe fn track_lifetime(_:EffectNode,_:f32) {unreachable!()}\n'+
    'pub unsafe fn create_timed_on_shield(_:crate::player::ChargeBladePlayer,_:u16,_:u16,_:f32)->EffectNode{unreachable!()}\n');
  const modules=gameModules().replace(modulePath('game-api/src/effect.rs','effect'),`#[path=${rustPath(effectFile)}] pub mod effect;`);
  const harness=String.raw`
#![allow(dead_code)]
extern crate self as forge;
use std::{cell::RefCell,ffi::{CStr,c_char},sync::atomic::{AtomicUsize,Ordering}};
${modules}
const BASE:u32=0x21000000;const MANAGER:u32=0x30000000;
const ARCHIVE:u32=0x31000000;const REQUEST:u32=0x32000000;
#[derive(Default)]struct Probe {fail:u8,loads:usize,finds:usize,releases:Vec<u32>,plays:usize,stream:usize,sound_manager:bool}
thread_local!{static PROBE:RefCell<Probe>=RefCell::new(Probe::default());}
#[macro_export]macro_rules! error{($($args:tt)*)=>{{let _=format!($($args)*);}};}
pub mod log{pub mod prelude{pub use crate::error;}}
pub mod os{pub mod sync{pub use std::sync::OnceLock;}}
mod state {pub struct State;pub static STATE:State=State;
    impl State{pub fn game(&self)->crate::Game{crate::Game::new(crate::BASE)}}}
mod config_file {
    use super::*;
    pub struct Config{pub effects:bool,pub sounds:bool,pub shield_thrust_duration:f32,pub shield_thrust_4g:bool}
    static MODE:AtomicUsize=AtomicUsize::new(0);
    static CONFIG:[Config;4]=[
        Config{effects:false,sounds:false,shield_thrust_duration:30.0,shield_thrust_4g:false},
        Config{effects:true,sounds:false,shield_thrust_duration:30.0,shield_thrust_4g:false},
        Config{effects:false,sounds:true,shield_thrust_duration:30.0,shield_thrust_4g:false},
        Config{effects:true,sounds:true,shield_thrust_duration:30.0,shield_thrust_4g:true}];
    pub fn get()->&'static Config{&CONFIG[MODE.load(Ordering::Relaxed)]}
    pub fn set(mode:usize){MODE.store(mode,Ordering::Relaxed);}
}
mod gaxe {
    use crate::effect::{EffectCreateParams,EffectNode,WeaponEffectShell};
    pub fn pel()->Option<crate::effect::PelHandle>{None}
    pub unsafe fn create_visual(_:WeaponEffectShell,_:u16,_:u32,_:&EffectCreateParams)->Option<EffectNode>{unreachable!()}
}
#[path=${rustPath(shieldFile)}] mod shield_thrust;
mod memory {
    use super::*;
    pub unsafe fn read_u32(address:u32)->u32{
        match address {
            a if a==BASE+0x02126A64=>MANAGER,
            a if a==BASE+0x01896F5C=>PROBE.with(|p|if p.borrow().sound_manager{MANAGER}else{0}),
            _=>panic!("unexpected read {address:#x}")
        }
    }
    pub unsafe fn read_u16(address:u32)->u16{unsafe{read_u32(address) as u16}}
    pub unsafe fn read_u8(address:u32)->u8{unsafe{read_u32(address) as u8}}
    pub unsafe fn read_f32(address:u32)->f32{f32::from_bits(unsafe{read_u32(address)})}
    pub use read_u32 as load_u32;pub use read_u8 as load_u8;pub use read_f32 as load_f32;
    pub unsafe fn write_u32(_:u32,_:u32){unreachable!()}
    pub unsafe fn write_u8(_:u32,_:u8){unreachable!()}
    pub(crate) unsafe fn method<T:Copy>(object:u32,offset:u32)->T{
        assert_eq!((object,offset),(MANAGER,0x30));
        let pointer=find as *const () as usize;unsafe{std::mem::transmute_copy(&pointer)}
    }
}
mod native {
    use super::*;
    pub unsafe fn function<T:Copy>(address:u32)->T{
        let pointer=match address-BASE{
            0x008385E8=>construct as *const () as usize,
            0x00838634=>destruct as *const () as usize,
            0x00BCA65C=>load as *const () as usize,
            0x00887660=>release as *const () as usize,
            0x004F0668=>play as *const () as usize,
            _=>panic!("unexpected native {address:#x}")
        };unsafe{std::mem::transmute_copy(&pointer)}
    }
}
unsafe extern "C" fn construct(stream:*mut u32,input:*const u8,size:u32,flags:u32)->u32{
    assert_eq!(input as usize%16,0);assert_eq!(flags,1);
    let bytes=unsafe{std::slice::from_raw_parts(input,size as usize)};
    assert_eq!(&bytes[..4],b"ARC\0");assert_eq!(u16::from_le_bytes(bytes[6..8].try_into().unwrap()),3);
    PROBE.with(|p|p.borrow_mut().stream=stream as usize);0
}
unsafe extern "C" fn destruct(stream:*mut u32)->u32{
    PROBE.with(|p|assert_eq!(p.borrow().stream,stream as usize));0
}
unsafe extern "C" fn load(manager:u32,stream:*mut u32,dti:u32,path:*const c_char,quality:u32)->u32{
    assert_eq!((manager,dti,quality),(MANAGER,BASE+0x02124DF0,4));
    assert_eq!(unsafe{CStr::from_ptr(path)},c"arc\\forge\\cb_shield_thrust_4g");
    PROBE.with(|p|{let mut p=p.borrow_mut();assert_eq!(p.stream,stream as usize);p.loads+=1;
        if p.fail==1{0}else{ARCHIVE}})
}
unsafe extern "C" fn find(manager:u32,dti:u32,path:*const c_char,flags:u32)->u32{
    assert_eq!((manager,dti,flags),(MANAGER,BASE+0x02125BEC,0x80));
    assert_eq!(unsafe{CStr::from_ptr(path)},c"sound\\cb4g\\shield_thrust");
    PROBE.with(|p|{let mut p=p.borrow_mut();p.finds+=1;if p.fail==2{0}else{REQUEST}})
}
unsafe extern "C" fn release(resource:u32){PROBE.with(|p|p.borrow_mut().releases.push(resource));}
unsafe extern "C" fn play(manager:u32,resource:u32,request:u32,event:u32,position:*const f32)->u32{
    assert_eq!((manager,resource,request,event),(MANAGER,REQUEST,0x55,1));
    assert_eq!(unsafe{std::slice::from_raw_parts(position,3)},[1.5,2.75,-3.0]);
    PROBE.with(|p|p.borrow_mut().plays+=1);0
}
fn main(){
    let fail=std::env::args().nth(1).unwrap().parse().unwrap();
    PROBE.with(|p|p.borrow_mut().fail=fail);
    for mode in [0,1]{config_file::set(mode);shield_thrust::preload();}
    PROBE.with(|p|assert_eq!(p.borrow().loads,0));
    config_file::set(3);for _ in 0..3{shield_thrust::preload();}
    PROBE.with(|p|{let p=p.borrow();assert_eq!(p.loads,1);assert_eq!(p.finds,usize::from(fail!=1));
        assert_eq!(p.releases,if fail==2{vec![ARCHIVE]}else{vec![]});});
    let resource=resource::Resource::from_address(REQUEST);
    assert!(!unsafe{sound::play_resource_at(Game::new(BASE),resource,0x55,[1.5,2.75,-3.0])});
    PROBE.with(|p|p.borrow_mut().sound_manager=true);
    assert!(unsafe{sound::play_resource_at(Game::new(BASE),resource,0x55,[1.5,2.75,-3.0])});
    PROBE.with(|p|assert_eq!(p.borrow().plays,1));
    println!("PASS: shield-thrust preload/cache/native API case {fail}; aligned ARC, sound DTI, isolated SRQR, load/failure lifetime and spatial submit");
}
`;
  for(const mode of ['0','1','2'])run('verify-shield-thrust-assets',harness,[mode]);
};
if(require.main===module) module.exports();
