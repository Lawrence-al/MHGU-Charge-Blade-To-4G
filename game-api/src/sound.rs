//! 音序、发声归属和 MSS 事件的对象接口。
use crate::{Game, memory, player::ChargeBladePlayer};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SoundSequence(u32);

impl SoundSequence {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }
    pub unsafe fn owner(self) -> SeOwner {
        SeOwner(unsafe { memory::read_u32(self.0 + 0x164) })
    }
    pub unsafe fn current_frame(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x90) }
    }
    pub unsafe fn previous_frame(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0xB0) }
    }
}

#[derive(Clone, Copy)]
pub struct SeRequest {
    pub request_id: u16,
    pub bank: u16,
    pub attach_index: i32,
    pub pool_path: u32,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct SeOwner(u32);

impl SeOwner {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }
    pub unsafe fn is_charge_blade_player(self, game: Game) -> bool {
        unsafe { ChargeBladePlayer::from_address(self.0).is_instance(game) }
    }

    /// 沿用原播放接口的管理器尚未就绪返回 false；不增加对象指针检查。
    pub unsafe fn play(self, game: Game, request: SeRequest) -> bool {
        let manager = unsafe { memory::read_u32(game.address(0x01896F5C)) };
        if manager == 0 {
            return false;
        }
        let write: unsafe extern "C" fn(u32, u32, u32, i32, u32, u32, i32) -> u32 =
            unsafe { game.function(0x004F0BE8) };
        unsafe {
            write(
                manager,
                self.0,
                request.request_id as u32,
                request.attach_index,
                request.pool_path,
                4,
                request.bank as i32,
            )
        };
        true
    }
}

/// MSS 的变长事件：4 字节头、scanCount 个 u32、参数区。
pub struct MssEvent(u32);

impl MssEvent {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub unsafe fn opcode(&self) -> u8 {
        unsafe { memory::read_u8(self.0) }
    }
    unsafe fn params(&self) -> u32 {
        self.0 + 4 + 4 * unsafe { memory::read_u8(self.0 + 2) as u32 }
    }
    pub unsafe fn bank(&self) -> u16 {
        unsafe { memory::read_u16(self.params()) }
    }
    pub unsafe fn request_id(&self) -> u16 {
        unsafe { memory::read_u16(self.params() + 2) }
    }
}
