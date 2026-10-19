//! 玩家、装备和盾牌的地址对象。字段偏移仅在本文件维护。
use crate::{
    Game,
    command::{CommandResource, CommandTable},
    effect::WeaponEffectShell,
    memory,
    sound::SoundSequence,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PhialKind {
    Impact,
    Element,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChargeBladePlayer(u32);

impl ChargeBladePlayer {
    pub const EQUIPMENT_OFFSET: u32 = 0x13C4;
    pub const ACTION_ID_OFFSET: u32 = 0x250A;
    pub const COMMAND_TABLE_OFFSET: u32 = 0x24AC;
    const COMMAND_TABLE_POINTER_OFFSET: u32 = 0x24A8;
    const WEAPON_COMMAND_RESOURCE_OFFSET: u32 = 0x24F0;
    const VTABLE_OFFSET: u32 = 0x01750D20;

    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn is_instance(self, game: Game) -> bool {
        unsafe { memory::read_u32(self.0) == game.address(Self::VTABLE_OFFSET) }
    }

    pub unsafe fn action_state(self) -> u32 {
        unsafe { memory::read_u8(self.0 + 5) as u32 }
    }

    pub unsafe fn action_id(self) -> u16 {
        unsafe { memory::read_u16(self.0 + Self::ACTION_ID_OFFSET) }
    }

    pub unsafe fn motion_id(self) -> u16 {
        unsafe { memory::read_u16(self.0 + 0x4B4) }
    }

    pub unsafe fn motion_frame(self) -> f32 {
        unsafe { memory::read_f32(self.0 + 0x500) }
    }

    pub unsafe fn has_phial(self) -> bool {
        unsafe { memory::read_u8(self.0 + 6) != 0 }
    }

    /// 内嵌 cFestaSoundSequence，直接加偏移，不能先读指针。
    pub const fn sound_sequence(self) -> SoundSequence {
        SoundSequence::from_address(self.0 + 0x111C)
    }

    pub unsafe fn equipment(self) -> Equipment {
        Equipment(unsafe { memory::read_u32(self.0 + Self::EQUIPMENT_OFFSET) })
    }

    pub unsafe fn command_chain(self) -> u32 {
        unsafe { memory::read_u32(self.0 + Self::COMMAND_TABLE_OFFSET) }
    }

    pub unsafe fn command_table(self) -> CommandTable {
        CommandTable::from_address(unsafe {
            memory::read_u32(self.0 + Self::COMMAND_TABLE_POINTER_OFFSET)
        })
    }

    pub unsafe fn command_resource(self) -> CommandResource {
        CommandResource::from_address(unsafe {
            memory::read_u32(self.0 + Self::WEAPON_COMMAND_RESOURCE_OFFSET)
        })
    }

    pub unsafe fn set_command_table(self, table: CommandTable) {
        unsafe { memory::write_u32(self.0 + Self::COMMAND_TABLE_POINTER_OFFSET, table.address()) };
    }

    pub unsafe fn phial_kind(self) -> PhialKind {
        unsafe { self.equipment().phial_kind() }
    }

    pub unsafe fn shield(self) -> Shield {
        Shield(unsafe { memory::read_u32(self.0 + 0x23A4) })
    }

    pub unsafe fn effect_shell(self) -> WeaponEffectShell {
        WeaponEffectShell::from_address(unsafe { memory::read_u32(self.0 + 0x2634) })
    }

    /// 0=未强化，1=黄盾，2=红盾，3=蓝盾。
    pub unsafe fn shield_state(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x3344) }
    }

    /// SAED 首爆取位沿用原瓶爆侧的 volatile 读。
    pub unsafe fn position(self) -> [f32; 3] {
        unsafe {
            [
                memory::load_f32(self.0 + 0x40),
                memory::load_f32(self.0 + 0x44),
                memory::load_f32(self.0 + 0x48),
            ]
        }
    }

    pub fn primary_hit_address(self) -> u32 {
        self.0 + 0x3020
    }
    pub fn secondary_hit_address(self) -> u32 {
        self.0 + 0x30D8
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Equipment(u32);

impl Equipment {
    pub const WEAPON_OFFSET: u32 = 0x4D4;
    pub const STYLE_OFFSET: u32 = 0x4D9;

    pub unsafe fn weapon_type(self) -> u8 {
        unsafe { memory::read_u8(self.0 + Self::WEAPON_OFFSET) }
    }

    /// 参数 27 的 s16；0 为榴弹瓶，非 0 为强属性瓶，不是玩家的有瓶标志。
    pub unsafe fn phial_kind(self) -> PhialKind {
        if unsafe { memory::read_u16(self.0 + 0x356) } == 0 {
            PhialKind::Impact
        } else {
            PhialKind::Element
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Shield(u32);

impl Shield {
    pub const fn address(self) -> u32 {
        self.0
    }

    /// 虚函数 +0x130 取得对象引用；不是直接传入盾牌本体地址。
    pub unsafe fn reference(self) -> u32 {
        let table = unsafe { memory::read_u32(self.0) };
        let function: unsafe extern "C" fn(u32) -> u32 =
            unsafe { core::mem::transmute(memory::read_u32(table + 0x130) as usize) };
        unsafe { function(self.0) }
    }
}
