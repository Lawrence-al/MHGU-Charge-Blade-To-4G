//! 项目内的 ARM32 钩子入口；兼容 Forge 的 24 / 76 字节 Hook 返回结构。
use core::ffi::c_void;

// Hook 仅供 Forge 保存安装信息，本项目不解释其内部字段。
// AAPCS 通过 r0 传入结构体返回区，r1/r2/r3 分别是目标、替代函数和原函数槽。
// 显式传入清零的最大返回区，避免旧 Rust SDK 按 76 字节读取新版只写 24 字节的返回值。
#[repr(C)]
pub struct HookStorage([u32; 19]);
impl HookStorage {
    pub const ZERO: Self = Self([0; 19]);
}

unsafe extern "C" {
    #[link_name = "forge_hook_create"]
    fn create_raw(
        output: *mut HookStorage,
        target: *const c_void,
        detour: *const c_void,
        original: *mut *const c_void,
    ) -> *mut HookStorage;
}

pub unsafe fn create(
    target: u32,
    detour: *const c_void,
    original: *mut *const c_void,
) -> HookStorage {
    let mut hook = HookStorage::ZERO;
    unsafe { create_raw(&raw mut hook, target as *const c_void, detour, original) };
    hook
}

// 与 forge::install_hook! 保持相同的模块安装入口；这些钩子不使用 context。
#[macro_export]
macro_rules! cb_hook {
    ($offset:literal, fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty,
        |$original:ident| $body:block) => {
        pub mod $name {
            use super::*;
            static mut __ORIGINAL: *const core::ffi::c_void = core::ptr::null();
            static mut __HOOK: $crate::runtime_hook::HookStorage =
                $crate::runtime_hook::HookStorage::ZERO;

            pub unsafe extern "C" fn __detour($($arg: $ty),*) -> $ret {
                let $original: unsafe extern "C" fn($($ty),*) -> $ret =
                    unsafe { core::mem::transmute(__ORIGINAL) };
                $body
            }

            pub unsafe fn __install(base: u32) {
                unsafe {
                    __HOOK = $crate::runtime_hook::create(
                        base + $offset, __detour as *const core::ffi::c_void,
                        &raw mut __ORIGINAL,
                    );
                }
            }
        }
    };
}
