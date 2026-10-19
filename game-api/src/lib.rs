//! MHGU 1.4.0 ARM32 对象访问与原生调用。对象由游戏持有，本库只保存地址。
//! 两份 NRO 静态链接此库；插件运行状态分别保存在各自的功能模块中。
#![no_std]

pub mod abi;
pub mod command;
pub mod effect;
pub mod memory;
pub mod player;
pub mod resource;
pub mod shell;
pub mod sound;

/// main 的运行时基址；所有函数偏移均为 IDA 地址减 0x60000000。
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Game(u32);

impl Game {
    pub const fn new(base: u32) -> Self {
        Self(base)
    }

    #[inline]
    pub const fn address(self, offset: u32) -> u32 {
        self.0.wrapping_add(offset)
    }

    /// 内部原生调用入口；T 必须是与该 ARM32 函数一致的函数指针类型。
    #[inline]
    pub(crate) unsafe fn function<T: Copy>(self, offset: u32) -> T {
        unsafe { core::mem::transmute_copy(&self.address(offset)) }
    }
}
