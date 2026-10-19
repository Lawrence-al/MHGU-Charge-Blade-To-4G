// 原资源合并、实际 ARM 回调以及按配置安装：三层各验证自己的输入和输出。
require('./verify-overpowered-merge');
const {fs,path,root,modulePath,run}=require('./host');
const assert=require('assert/strict');
const {Arm,loadPlugin}=require('./arm');
const plugin=loadPlugin(path.join(root,'Moveset-to-4G'),'moveset_to_4g');
const detour=plugin.symbol('hook_move_chain_init8___detour');
const slot=plugin.symbol('hook_move_chain_init10___ORIGINAL');
const helper=plugin.symbol('overpowered16after_chain_init');
let cases=0;
for(const base of [0x70000000,0x4C000000]){
  const original=0x61001000,end=0x21000000;
  for(const player of [0x22000000,0x23000000])for(const chain of [0x7F4,0x7F5,0x839,0x83A,0x7DC,0x81A,0]){
    for(const frame of [-9,0,2,3000])for(const result of [0,1,0x12345678]){
      const regs=Array.from({length:16},(_,i)=>(0x31000000+i*0x10101)>>>0);
      regs[0]=player;regs[1]=chain;regs[2]=frame>>>0;regs[13]=0x24001000;regs[14]=end;
      const cpu=new Arm(regs,[{base,data:plugin.nro}],[[base+slot.value,original]]);
      const savedVfp=[...cpu.s],calls=[];
      const clobber=machine=>{
        for(const r of [0,1,2,3,12])machine.r[r]=0xB0000000+r;
        for(let i=0;i<16;i++)machine.s[i]=0x3F800000+i;
      };
      cpu.externalCalls.set(original,machine=>{
        assert.deepEqual([...machine.r.slice(0,3)],[player,chain,frame>>>0]);
        assert.equal(machine.r[13]%8,0,'原函数调用栈对齐');
        calls.push('original');clobber(machine);machine.r[0]=result;
      });
      cpu.externalCalls.set(base+helper.value,machine=>{
        assert.deepEqual(calls,['original'],'先初始化游戏表，再合并');
        assert.deepEqual([...machine.r.slice(0,2)],[player,chain],'原函数破坏临时寄存器后仍保留输入');
        assert.equal(machine.r[13]%8,0,'合并回调栈对齐');
        calls.push('merge');clobber(machine);
      });
      cpu.run(base+detour.value,end,50);
      assert.deepEqual(calls,['original','merge']);assert.equal(cpu.r[0],result,'保留原 R0 返回值');
      for(let r=4;r<=11;r++)assert.equal(cpu.r[r],regs[r],'callee-saved r'+r);
      assert.equal(cpu.r[13],regs[13]);assert.deepEqual([...cpu.s.slice(16)],savedVfp.slice(16));
      assert.equal(cpu.svcs.length,0);cases++;
    }
  }
}
console.log('PASS: '+cases+' compiled InitMoveChain callbacks; original-first ordering, args/result, stack, callee-saved ARM/VFP registers and two load bases');

// 执行真实 enable 函数；仅安装宏和 init 函数用事件记录替身。
const source=fs.readFileSync(path.join(root,'Moveset-to-4G/src/hooks.rs'),'utf8');
const enable=source.slice(source.indexOf('pub fn enable('));
run('verify-overpowered-hook-selection',`
#![allow(dead_code)]
extern crate self as forge;
${modulePath('Moveset-to-4G/src/config.rs','config')}
use config::Config;
use std::cell::RefCell;
thread_local!{static EVENTS:RefCell<Vec<&'static str>>=const{RefCell::new(Vec::new())};}
fn event(v:&'static str){EVENTS.with(|e|e.borrow_mut().push(v));}
#[macro_export]macro_rules! install_hook{($base:expr,$name:ident)=>{{assert_eq!($base,0x60000000);crate::event(stringify!($name));}};}
#[macro_export]macro_rules! error{($($arg:tt)*)=>{let _=format!($($arg)*);};}
pub mod log{pub mod prelude{pub use crate::error;}}
mod overpowered{pub fn init(){crate::event("overpowered::init");}}
mod phial{pub fn init(){crate::event("phial::init");}}
${enable}
fn main(){
 for bits in 0..512{
  let mut c=Config::DEFAULT;
  c.enabled=bits&1!=0;c.overpowered=bits&2!=0;c.hitdata=bits&4!=0;c.saed.enabled=bits&8!=0;
  c.moveset.yellow_shield_aed=bits&16!=0;c.moveset.gp_recoil=bits&32!=0;c.moveset.j_guard_aed=bits&64!=0;
  c.moveset.bushi_shield_thrust_aed=bits&128!=0;c.moveset.aed_camera_shake=bits&256!=0;
  EVENTS.with(|e|e.borrow_mut().clear());if c.enabled{enable(0x60000000,&c);}
  let mut expected=Vec::new();
  if c.enabled&&c.overpowered{expected.extend(["overpowered::init","hook_move_chain_init"]);}
  if c.needs_hitdata(){expected.push("hook_hitdata_load");}
  if c.needs_phial(){expected.extend(["phial::init","hook_saed_shell_init","hook_saed_shell_update","hook_saed_shell_destruct"]);}
  EVENTS.with(|e|assert_eq!(*e.borrow(),expected,"bits={bits}"));
 }
 println!("PASS: 512 actual hook-selection combinations; Overpowered installs one InitMoveChain hook, disabled groups install none");
}
`);
