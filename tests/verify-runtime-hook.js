// 项目内 Hook 返回区覆盖当前 24 字节与旧 76 字节实现，原函数参数／结果仍直接传递。
const {modulePath,run} = require('./host');
run('verify-runtime-hook', `
#![allow(dead_code)]
use std::{cell::Cell,ffi::c_void};
${modulePath('shared/runtime_hook.rs','runtime_hook')}
thread_local! {static BYTES:Cell<usize>=const{Cell::new(24)};}
unsafe extern "C" fn game(x:u32,y:u32)->u32 {assert_eq!((x,y),(17,23));0x12345678}
#[unsafe(export_name="forge_hook_create")]
unsafe extern "C" fn create(out:*mut runtime_hook::HookStorage,target:*const c_void,detour:*const c_void,original:*mut *const c_void)->*mut runtime_hook::HookStorage {
    assert_eq!(target as usize,0x60000020);
    assert_eq!(detour,probe::__detour as *const c_void);
    let data=unsafe{std::slice::from_raw_parts_mut(out.cast::<u8>(),76)};
    assert!(data.iter().all(|v|*v==0));
    BYTES.with(|n|data[..n.get()].fill(0x5A));
    unsafe{*original=game as *const c_void};out
}
crate::cb_hook! {0x20,
fn probe(x:u32,y:u32)->u32,|original| {unsafe{original(x,y)}}
}
fn main() {
    assert_eq!(std::mem::size_of::<runtime_hook::HookStorage>(),76);
    for length in [24,76] {
        BYTES.with(|n|n.set(length));unsafe{probe::__install(0x60000000)};
        assert_eq!(unsafe{probe::__detour(17,23)},0x12345678);
        let mut slot=std::ptr::null();
        let value=unsafe{runtime_hook::create(0x60000020,probe::__detour as *const c_void,&raw mut slot)};
        let data=unsafe{std::slice::from_raw_parts((&raw const value).cast::<u8>(),76)};
        assert!(data[..length].iter().all(|v|*v==0x5A));
        assert!(data[length..].iter().all(|v|*v==0));
        assert_eq!(slot,game as *const c_void);
    }
    println!("PASS: local Hook buffer initialized for 24/76-byte Forge returns; original slot, target/detour, parameters and result preserved");
}
`);
