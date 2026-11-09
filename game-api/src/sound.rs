//! 音序、发声归属和 MSS 事件的对象接口。
use crate::{Game, effect::WeaponEffectShell, memory, player::ChargeBladePlayer, shell::SaedShell};

/// 命中声使用的MhHitAtk；直接来源与最终归属玩家是两个字段。
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct HitSoundSource(u32);

impl HitSoundSource {
    pub const fn from_attack(address: u32) -> Self {
        Self(address)
    }

    /// +0x10=2表示shell；+0x0C仍指向直接来源，+0x70已由原函数改为归属玩家。
    pub unsafe fn is_charge_blade_phial(self, game: Game) -> bool {
        unsafe {
            memory::read_u8(self.0 + 0x10) == 2
                && {
                    let shell = memory::read_u32(self.0 + 0x0C);
                    WeaponEffectShell::from_address(shell).is_instance(game)
                        || SaedShell::from_address(shell).is_instance(game)
                }
        }
    }
}

/// 原公共命中声排队入口；保留请求号、世界坐标和场景编号。
pub unsafe fn submit_common_hit(
    game: Game, manager: u32, request: u32, position: u32, room: u32,
) -> u32 {
    let submit: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
        unsafe { game.function(0x004EF098) };
    unsafe { submit(manager, request, position, room) }
}

/// 公共SRQ的一次性空间声；原入口立即复制爆点，不绑定可能已回收的粒子节点。
pub unsafe fn play_common_at(game: Game, request: u16, position: u32, room: u32) -> u32 {
    let manager = unsafe { memory::read_u32(game.address(0x01896F5C)) };
    unsafe { submit_common_hit(game, manager, request as u32, position, room) }
}

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
