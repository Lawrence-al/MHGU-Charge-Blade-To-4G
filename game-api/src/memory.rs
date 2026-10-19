//! 保留两种原有访问方式：资源／表现字段使用非对齐读，瓶爆进程使用 volatile。
use core::{mem, ptr};

#[inline]
pub unsafe fn read_u8(address: u32) -> u8 {
    unsafe { ptr::read_unaligned(address as *const u8) }
}

#[inline]
pub unsafe fn read_u16(address: u32) -> u16 {
    unsafe { ptr::read_unaligned(address as *const u16) }
}

#[inline]
pub unsafe fn read_u32(address: u32) -> u32 {
    unsafe { ptr::read_unaligned(address as *const u32) }
}

#[inline]
pub unsafe fn read_f32(address: u32) -> f32 {
    f32::from_bits(unsafe { read_u32(address) })
}

#[inline]
pub unsafe fn load_u8(address: u32) -> u8 {
    unsafe { ptr::read_volatile(address as *const u8) }
}

#[inline]
pub unsafe fn load_u32(address: u32) -> u32 {
    unsafe { ptr::read_volatile(address as *const u32) }
}

#[inline]
pub unsafe fn load_f32(address: u32) -> f32 {
    f32::from_bits(unsafe { load_u32(address) })
}

#[inline]
pub unsafe fn write_u8(address: u32, value: u8) {
    unsafe { ptr::write_volatile(address as *mut u8, value) };
}

#[inline]
pub unsafe fn write_u32(address: u32, value: u32) {
    unsafe { ptr::write_volatile(address as *mut u32, value) };
}

/// 虚表调用沿用瓶爆侧的 volatile 读取；T 必须匹配对应虚函数签名。
#[inline]
pub(crate) unsafe fn method<T: Copy>(object: u32, offset: u32) -> T {
    let table = unsafe { load_u32(object) };
    unsafe { mem::transmute_copy(&load_u32(table + offset)) }
}
