// 执行实际统一加载函数／入口回调，主机仅适配指针宽度及原游戏分配器。
const {fs, path, root, dir, rustPath, modulePath, run} = require('./host');
const assert = require('assert/strict');
const {loadPlugin} = require('./arm');
const dump = process.argv[2];
if (!dump) throw Error('用法：node tests/verify-hitdata.js "MHGU dump 目录"');
const originalFile = path.join(dump, 'romfs/nativeNX/arc/player/quest/w14-arc/player/hit/pl_we14_hitdata.4E2EF008');
const original = fs.readFileSync(originalFile);

const source = fs.readFileSync(path.join(root, 'Moveset-to-4G/src/hitdata/mod.rs'), 'utf8');
const split = source.indexOf('#[cfg(target_arch = "arm")]');
let adapter = source.slice(0, split)
  .replace('pub mod records;', modulePath('Moveset-to-4G/src/hitdata/records.rs', 'records'))
  .replace('"../../../shared/hitdata_path.rs"', rustPath(path.join(root, 'shared/hitdata_path.rs')));
adapter += source.slice(split).replace('#[cfg(target_arch = "arm")]', '')
  .replace(/\bu32\b/g, 'usize').replace('accepts(path, count, config)', 'accepts(path, count as u32, config)');
const resourceFile = path.join(dir, 'resource-host.rs');
fs.writeFileSync(resourceFile, fs.readFileSync(path.join(root,'game-api/src/resource.rs'),'utf8').replace(/\bu32\b/g,'usize'));
const adaptedFile = path.join(dir, 'hitdata-host.rs');
fs.writeFileSync(adaptedFile, adapter);
const hooks = fs.readFileSync(path.join(root, 'Moveset-to-4G/src/hooks.rs'), 'utf8');
const callback = hooks.match(/crate::cb_hook!\s*\{\s*0x00270C18,\s*(fn hook_hitdata_load[\s\S]*?\n})\n}/)[1]
  .replace('resource: u32, stream: u32', 'resource: usize, stream: usize')
  .replace('-> u32, |original| {', '-> u32 {').replace('original(resource, stream)', 'original!(resource, stream)');
run('verify-unified-hitdata', `
#![allow(dead_code, unused_unsafe)]
extern crate self as forge;
extern crate self as cb_game;
#[derive(Clone,Copy)] pub struct Game(usize);
impl Game {
    pub const fn new(base:usize)->Self {Self(base)}
    pub const fn address(self,offset:usize)->usize {self.0+offset}
    pub(crate) unsafe fn function<T:Copy>(self,offset:usize)->T {unsafe{native::function(self.address(offset))}}
}
pub mod mem {pub fn text_addr()->usize {0}}
mod memory {
    pub use super::native::read_u32;
    pub use read_u32 as load_u32;
    pub use super::native::write_u32;
    pub(crate) unsafe fn method<T:Copy>(object:usize,offset:usize)->T {unsafe{super::native::method(object,offset)}}
}
#[path=${rustPath(resourceFile)}] pub mod resource;
use std::{cell::RefCell, collections::BTreeMap, ptr, sync::atomic::{AtomicPtr, Ordering}};
#[macro_export] macro_rules! error { ($($arg:tt)*) => { let _=format!($($arg)*); }; }
pub mod log { pub mod prelude { pub use crate::error; } }
${modulePath('Moveset-to-4G/src/config.rs', 'config')}
#[path=${rustPath(adaptedFile)}] mod hitdata;
mod config_file {
    use super::*;
    static CONFIG:AtomicPtr<config::Config>=AtomicPtr::new(ptr::null_mut());
    pub fn set(c:config::Config) { CONFIG.store(Box::into_raw(Box::new(c)),Ordering::Release); }
    pub fn get()->&'static config::Config { unsafe{&*CONFIG.load(Ordering::Acquire)} }
}
mod native {
    use super::*;
    pub struct State { pub fields:BTreeMap<usize,usize>, pub buffers:Vec<Box<[u8]>>, pub original:usize, pub loads:usize, pub allocations:usize, pub frees:usize }
    thread_local! { pub static STATE:RefCell<State>=RefCell::new(State{fields:BTreeMap::new(),buffers:Vec::new(),original:0,loads:0,allocations:0,frees:0}); }
    pub fn address(offset:usize)->usize { offset }
    pub unsafe fn read_u32(address:usize)->usize {
        STATE.with(|s| if let Some(value)=s.borrow().fields.get(&address) { *value } else { (unsafe{ptr::read_unaligned(address as *const u32)}) as usize })
    }
    pub unsafe fn write_u32(address:usize,value:usize) { STATE.with(|s|{s.borrow_mut().fields.insert(address,value);}); }
    unsafe extern "C" fn get_allocator(_:usize)->usize { 0x101 }
    unsafe extern "C" fn allocate(allocator:usize,size:usize,align:usize)->usize {
        assert_eq!((allocator,size,align),(0x101,4552,16));
        let data=vec![0xCD;size].into_boxed_slice(); let pointer=data.as_ptr() as usize;
        STATE.with(|s| { let mut s=s.borrow_mut(); s.allocations+=1; s.buffers.push(data); }); pointer
    }
    unsafe extern "C" fn free(allocator:usize,data:usize) {
        assert_eq!(allocator,0x101);
        STATE.with(|s|{let mut s=s.borrow_mut();let index=s.buffers.iter().position(|b|b.as_ptr() as usize==data).unwrap();s.buffers.remove(index);s.frees+=1;});
    }
    pub unsafe fn function<T:Copy>(offset:usize)->T { assert_eq!(offset,0x007AA47C); let pointer=get_allocator as *const (); unsafe{std::mem::transmute_copy(&pointer)} }
    pub unsafe fn method<T:Copy>(allocator:usize,offset:usize)->T {
        assert_eq!(allocator,0x101);let pointer=match offset {0x1C=>allocate as *const (),0x34=>free as *const (),_=>panic!("unknown method")};unsafe{std::mem::transmute_copy(&pointer)}
    }
    pub unsafe fn load_original(resource:usize,result:usize)->u32 {
        STATE.with(|s|{let mut s=s.borrow_mut();s.loads+=1;if result!=0 {let data=s.original;s.fields.insert(resource+0x64,data);s.fields.insert(resource+0x68,data+16);}});result as u32
    }
}
macro_rules! original { ($resource:expr,$stream:expr)=>{native::load_original($resource,$stream)}; }
${callback}
fn main() {
    let original=std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let original_4f=&original[16+79*56..];
    let mut cases=0;
    for input in [original.as_slice(),hitdata::records::ASSET.as_slice()] {
    for path in ["player\\\\hit\\\\pl_we14_hitdata","HIT/PL_WE14_HITDATA.hdp","player/hit/pl_we13_hitdata"] {
    for count in [79u32,80,81] { for asset in [false,true] { for third in [false,true] { for enabled in [false,true] {
    for result in [0,1,7] { for _reload in 0..2 {
        let mut c=config::Config::DEFAULT; c.hitdata=asset;c.saed.enabled=third;c.enabled=enabled;config_file::set(c);
        let mut loaded=input.to_vec();if count==81 { loaded.extend_from_slice(include_bytes!(${rustPath(path.join(root, 'tests/data/saed-hdp50.bin'))})); } loaded[8..12].copy_from_slice(&count.to_le_bytes());
        let data=loaded.clone().into_boxed_slice();let old=data.as_ptr() as usize;
        native::STATE.with(|s| *s.borrow_mut()=native::State{fields:BTreeMap::new(),buffers:vec![data],original:old,loads:0,allocations:0,frees:0});
        let mut resource=[0u8;128];resource[8..8+path.len()].copy_from_slice(path.as_bytes());let address=resource.as_mut_ptr() as usize;
        assert_eq!(unsafe{hook_hitdata_load(address,result)},result as u32);
        let accepted=enabled&&(asset||third)&&!path.contains("we13")&&count>=80&&result!=0;
        let extended=accepted&&third&&count==80;
        let mut expected=loaded;
        if extended {expected.extend_from_slice(include_bytes!(${rustPath(path.join(root, 'tests/data/saed-hdp50.bin'))}));expected[8..12].copy_from_slice(&81u32.to_le_bytes());}
        if accepted&&asset {expected[16..16+79*56].copy_from_slice(&hitdata::records::ASSET[16..16+79*56]);expected[16+79*56..4496].copy_from_slice(original_4f);}
        if accepted&&third {let offset=4496;expected[offset..4552].copy_from_slice(include_bytes!(${rustPath(path.join(root, 'tests/data/saed-hdp50.bin'))}));}
        native::STATE.with(|s|{let s=s.borrow();assert_eq!(s.loads,1);assert_eq!(s.allocations,usize::from(extended));assert_eq!(s.frees,usize::from(extended));assert_eq!(s.buffers.len(),1);assert_eq!(s.buffers[0].as_ref(),expected,"path={path} count={count} asset={asset} third={third} enabled={enabled} result={result}");if result!=0 {let current=s.fields[&(address+0x64)];assert_eq!(current,s.buffers[0].as_ptr() as usize);assert_eq!(s.fields[&(address+0x68)],current+16);}});
        cases+=1;
    } } } } } }
    } }
    println!("PASS: {cases} actual unified HDP loader/callback cases; original called once, both input files, independent Hitdata/SAED switches, fixed 0x50, 0x4F, return values, reload, allocator ownership and both resource pointers");
}
`, [originalFile]);
const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
const embedded = fs.readFileSync(path.join(root, 'assets/romfs/nativeNX/arc/player/quest/w14.arc/hit/pl_we14_hitdata.4E2EF008'));
assert.ok(plugin.nro.includes(embedded.subarray(16, 16 + 79 * 56)), 'NRO 未包含完整 asset 0x00..0x4E');
assert.ok(plugin.nro.includes(original.subarray(16 + 79 * 56)), 'NRO 未包含完整原版 0x4F');
console.log('PASS: merged ARM NRO contains full asset records 0x00..0x4E and original 0x4F snapshot');
