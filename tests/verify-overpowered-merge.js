// 运行真实合并代码：原资源决定期望顺序和窗口，内存／互斥锁由宿主替身提供。
const {fs,path,root,dir,rustPath,modulePath,run}=require('./host');
const gameModules=require('./game-host');
const styles=require('./command-resources')(process.argv[2]);
const fixtures=[];
function cmd(c){
  const f=c.fields||{st:0xEE,no:0xFF,param:0x1234,type:8,args:[0x12345678,0x87654321,4,7]};
  const words=[f.st|(f.no<<16),(f.param|(f.type<<16))>>>0,...f.args,(c.start|(c.end<<16))>>>0,(c.pre|0xA5A50000)>>>0];
  return '['+words.join(',')+']';
}
for(const style of styles){
  const axe=style.tables.find(t=>t.chain===0x7DC).commands;
  for(const chain of [0x7F4,0x7F5,0x839,0x83A]){
    const original=style.tables.find(t=>t.chain===chain).commands;
    fixtures.push(`Fixture{style:${style.style},chain:${chain},original:&[${original.map(cmd)}],axe:&[${axe.map(cmd)}]}`);
  }
}
const adapter=path.join(dir,'overpowered-merge-host.rs');
fs.writeFileSync(adapter,fs.readFileSync(path.join(root,'Moveset-to-4G/src/overpowered.rs'),'utf8')+`
// 仅宿主适配器导出快照，生产源不增加测试接口。
pub fn test_snapshot(player:u32)->Option<(u32,TableHeader,Vec<u32>,Vec<(u32,u32)>)>{
 with_tables(|actors|{
  let a=actors.iter().find(|a|a.player.address()==player)?;
  let header=a.table().address();let mut words=Vec::new();
  for i in 0..7{words.push((header+i*4,unsafe{*((&raw const a.header).cast::<u32>().add(i as usize))}));}
  for i in 0..MAX_COMMANDS{words.push((a.records.as_ptr() as u32+i as u32*4,a.records[i]));}
  Some((header,a.header,a.records[..a.header.count as usize].to_vec(),words))
 })
}
pub fn test_slots()->usize{with_tables(|actors|actors.len())}
`);
run('verify-overpowered-merge',`
#![allow(dead_code)]
extern crate alloc;
extern crate self as forge;
use std::{cell::RefCell,collections::BTreeMap};
${gameModules()}
#[path=${rustPath(adapter)}] mod overpowered;
thread_local!{static FIELDS:RefCell<BTreeMap<u32,u32>>=const{RefCell::new(BTreeMap::new())};static LOGS:RefCell<Vec<String>>=const{RefCell::new(Vec::new())};}
#[macro_export] macro_rules! error{($($arg:tt)*)=>{crate::LOGS.with(|l|l.borrow_mut().push(format!($($arg)*)))};}
pub mod log{pub mod prelude{pub use crate::error;}}
pub mod os{pub mod sync{
 pub use std::sync::LazyLock;
 pub struct Mutex(std::sync::Mutex<()>);impl Mutex{pub fn new()->Self{Self(std::sync::Mutex::new(()))}}
 pub struct ScopedLock<'a>(#[allow(dead_code)] std::sync::MutexGuard<'a,()>);impl<'a> ScopedLock<'a>{pub fn new(m:&'a Mutex)->Self{Self(m.0.lock().unwrap())}}
}}
mod memory{
 pub unsafe fn read_u32(a:u32)->u32{crate::FIELDS.with(|f|*f.borrow().get(&a).unwrap_or_else(||panic!("unmapped {a:X}")))}
 pub unsafe fn read_u8(a:u32)->u8{unsafe{read_u32(a) as u8}}
 pub unsafe fn read_u16(a:u32)->u16{unsafe{read_u32(a) as u16}}
 pub unsafe fn read_f32(a:u32)->f32{f32::from_bits(unsafe{read_u32(a)})}
 pub use read_u32 as load_u32;pub use read_u8 as load_u8;pub use read_f32 as load_f32;
 pub unsafe fn write_u32(a:u32,v:u32){crate::FIELDS.with(|f|{f.borrow_mut().insert(a,v);});}
 pub unsafe fn write_u8(a:u32,v:u8){unsafe{write_u32(a,v as u32)}}
 pub(crate) unsafe fn method<T:Copy>(_:u32,_:u32)->T{unreachable!()}
}
mod native{pub unsafe fn function<T:Copy>(_:u32)->T{unreachable!()}}
struct Fixture{style:u32,chain:u32,original:&'static[[u32;8]],axe:&'static[[u32;8]]}
const FIXTURES:&[Fixture]=&[${fixtures.join(',\n')}];
fn put(a:u32,v:u32){unsafe{memory::write_u32(a,v)}}
fn get(a:u32)->u32{unsafe{memory::read_u32(a)}}
struct Native{player:u32,original:u32,axe:u32,orig_refs:Vec<u32>,axe_refs:Vec<u32>}
fn setup(f:&Fixture,slot:u32,player:u32,weapon:u32)->Native{
 let base=0x40000000+slot*0x10000;let equip=base+0x1000;let root=base+0x2000;let tables=base+0x3000;
 let original=base+0x4000;let oa=base+0x5000;let axe=base+0x6000;let aa=base+0x7000;
 let mut refs=[Vec::new(),Vec::new()];
 for (which,(list,array,table)) in [(f.original,oa,original),(f.axe,aa,axe)].into_iter().enumerate(){
  for(i,&v)in [0xABCDEF01,0x12345678,list.len() as u32,0x65432100,0x76543210,array,0x87654321].iter().enumerate(){put(table+i as u32*4,v);}
  for(i,record)in list.iter().enumerate(){let ptr=base+0x8000+which as u32*0x1000+i as u32*32;put(array+i as u32*4,ptr);refs[which].push(ptr);for(j,&v)in record.iter().enumerate(){put(ptr+j as u32*4,v);}}
 }
 put(player+0x13C4,equip);put(equip+0x4D4,weapon);put(equip+0x4D9,f.style);
 put(player+0x24A8,original);put(player+0x24AC,f.chain);put(player+0x24F0,root);put(root+0x74,tables);put(tables+(0x7DC-2000)*4,axe);
 for i in 0..12{put(player+0x24B0+i*4,0x9ABC0000+i);}
 Native{player,original,axe,orig_refs:refs[0].clone(),axe_refs:refs[1].clone()}
}
fn original_memory_preserved(before:BTreeMap<u32,u32>,player:u32){FIELDS.with(|f|{let after=f.borrow();for(a,v)in before{if a!=player+0x24A8{assert_eq!(after[&a],v,"changed native memory {a:X}");}}});}
fn merged(n:&Native,f:&Fixture)->(u32,Vec<u32>){
 let(p,h,refs,words)=overpowered::test_snapshot(n.player).unwrap();assert_eq!(get(n.player+0x24A8),p);assert_eq!(get(n.player+0x24AC),f.chain);
 let expected:Vec<_>=n.orig_refs.iter().copied().chain(n.axe_refs.iter().skip(1).copied()).collect();assert_eq!(refs,expected);
 let mut header=unsafe{command::CommandTable::from_address(n.original).header()};header.count=expected.len() as u32;header.records=h.records;assert_eq!(h,header);
 if let Some(first)=n.axe_refs.first(){assert!(!refs.contains(first),"axe bounce record appended");}
 // 用表实际指针和数量走与游戏相同的读取路径，验证不是只返回一份期望 Vec。
 for(a,v)in words{put(a,v);}
 let view=command::CommandTable::from_address(p);assert_eq!(unsafe{view.count()},expected.len() as u32);
 for(i,&r)in expected.iter().enumerate(){assert_eq!(unsafe{view.record(i)},r);let payload:Vec<_>=(0..8).map(|j|get(r+j*4)).collect();let data=if i<f.original.len(){f.original[i]}else{f.axe[i-f.original.len()+1]};assert_eq!(payload,data);}
 if f.chain==0x7F4{assert_eq!(get(refs[0])>>16,0x39);assert_eq!(get(refs[0]+8),5);assert_eq!(get(refs[0]+24),28<<16);assert_eq!(get(refs[0]+28)&0xFFFF,0);}
 (p,refs)
}
fn main(){
 overpowered::init();let mut total=0;let mut first=None;
 for(i,f)in FIXTURES.iter().enumerate(){for repeat in 0..3{
  let actor=0x10000000+i as u32*0x10000;let n=setup(f,i as u32*3+repeat,actor,14);let before=FIELDS.with(|f|f.borrow().clone());
  unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(actor),f.chain)};
  original_memory_preserved(before,actor);let state=merged(&n,f);
  if repeat==0{if i==0{first=Some((actor,state.0));}}else{assert_eq!(overpowered::test_slots(),i+1);}
  let slots=overpowered::test_slots();unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(actor),f.chain)};
  assert_eq!(overpowered::test_slots(),slots);assert_eq!(merged(&n,f),state);total+=1;
 }}
 let(actor,p)=first.unwrap();assert_eq!(get(actor+0x24A8),p,"Vec growth moved an active player's table");
 // 同一玩家改风格后重用表地址，但重新读取当前风格命令。
 for (style,f) in FIXTURES.iter().filter(|f|f.chain==0x7F4).enumerate(){
  let n=setup(f,110+style as u32,actor,14);let slots=overpowered::test_slots();
  unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(actor),f.chain)};
  assert_eq!(merged(&n,f).0,p);assert_eq!(overpowered::test_slots(),slots);
 }
 // 离开大解由原初始化写回游戏表；回调不接管其它链。
 put(actor+0x24A8,0x77880000);put(actor+0x24AC,0x7DC);
 unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(actor),0x7DC)};
 assert_eq!(get(actor+0x24A8),0x77880000);
 let f=&FIXTURES[0];let n=setup(f,100,0x12000000,9);let slots=overpowered::test_slots();unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(n.player),f.chain)};assert_eq!(get(n.player+0x24A8),n.original);assert_eq!(overpowered::test_slots(),slots);
 let n=setup(f,101,0x12010000,14);put(n.player+0x24AC,0x7DC);unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(n.player),f.chain)};assert_eq!(get(n.player+0x24A8),n.original);
 // 非大解链快速返回，不访问未初始化的武器字段，也不接管原斧竖劈的弹刀。
 for chain in [0,0x7DC,0x81A,0x7FA,9999]{unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(0x13000000),chain)};}
 let full:Vec<[u32;8]>=vec![[0x5555;8];48];let one:[[u32;8];2]=[[0x6666;8];2];
 // 动态测试数据仅在此进程中泄漏为静态 fixture，生产数据不分配这些 Vec。
 let large=Fixture{style:5,chain:0x7F4,original:Box::leak(full.into_boxed_slice()),axe:Box::leak(Box::new(one))};
 let n=setup(&large,102,0x12020000,14);unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(n.player),large.chain)};assert_eq!(get(n.player+0x24A8),n.original);LOGS.with(|l|assert!(l.borrow().last().unwrap().contains("48+1")));
 let empty=Fixture{style:5,chain:0x839,original:&[],axe:&[]};let n=setup(&empty,103,0x12030000,14);unsafe{overpowered::after_chain_init(player::ChargeBladePlayer::from_address(n.player),empty.chain)};assert_eq!(get(n.player+0x24A8),n.original);
 println!("PASS: {total} real resource merge/reload cases, six styles / four AED chains; original red-shield/input/windows/header retained, axe entry 0 excluded, stable per-player storage, style changes/chain exit, no duplicate append, no native resource writes; rejected/non-CB/normal chains and capacity bound");
}
`);
