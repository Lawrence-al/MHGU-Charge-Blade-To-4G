//! 玩家、装备和剑盾的地址对象。字段偏移仅在本文件维护。
use crate::{
    Game, abi,
    command::{CommandResource, CommandTable},
    effect::{PelHandle, WeaponEffectShell},
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
    pub const SHIELD_STATE_OFFSET: u32 = 0x3344;
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

    /// 与 sub_6053A058 相同：+0x4B4 是动画索引，虚函数 +0xC0 映射到实际动画编号。
    pub unsafe fn animation_id(self) -> u32 {
        let function: unsafe extern "C" fn(u32, u32) -> u32 =
            unsafe { memory::method(self.0, 0xC0) };
        unsafe { function(self.0, self.motion_id() as u32) }
    }

    /// GP 开窗阶段：0=等待，1=窗口内，2=结束；单独读取不能证明当前仍在 GP。
    pub unsafe fn gp_stage(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x333C) }
    }

    /// 通用防御有效位 0x10；普通防御也使用这个位。
    pub unsafe fn guard_flags(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x2510) }
    }

    /// 0x20 是特殊 GP 受击反应位，也参与 GU 蓝盾防性修正。
    pub unsafe fn shield_flags(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x3340) }
    }

    pub unsafe fn motion_frame(self) -> f32 {
        unsafe { memory::read_f32(self.0 + 0x500) }
    }

    pub unsafe fn has_phial(self) -> bool {
        unsafe { memory::read_u8(self.0 + 6) != 0 }
    }

    /// CreateShell03读此真实瓶数，必须在原函数发瓶/消耗之前捕获。
    pub unsafe fn phial_count(self) -> u32 {
        unsafe { memory::read_u32(self.0 + 0x3354) }
    }

    pub unsafe fn crossed_frame(self, game: Game, frame: f32) -> bool {
        unsafe { abi::frame_event(game.address(0x00280C1C), self.0, frame.to_bits()) != 0 }
    }

    /// 复用011DEA04的原生空间基准，不调用其无瓶尘土创建。
    /// 611DFEB4给坡面/朝向角；原式为230*(cosX*sinY,-sinX,cosX*cosY)。
    pub unsafe fn saed_ground_pose(self, game: Game) -> ([f32; 3], [f32; 3]) {
        let angles: unsafe extern "C" fn(*mut [u32; 3], u32) -> u32 =
            unsafe { game.function(0x011DFEB4) };
        let mut values = [0u32; 3];
        unsafe { angles(&mut values, self.0) };
        let radians = values.map(|angle| (angle as u16 as f32) * (core::f32::consts::TAU / 65536.0));
        let sin = |angle: f32| unsafe { f32::from_bits(abi::trig_call(game.address(0x013EFDE8), angle.to_bits())) };
        let cos = |angle: f32| unsafe { f32::from_bits(abi::trig_call(game.address(0x013EFDF4), angle.to_bits())) };
        let origin = unsafe { self.position() };
        let forward = 230.0 * cos(radians[0]);
        let position = [origin[0] + forward * sin(radians[1]), origin[1] - 230.0 * sin(radians[0]), origin[2] + forward * cos(radians[1])];
        (position, radians.map(|angle| angle * (180.0 / core::f32::consts::PI)))
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

    /// 主武器 slot0；盾斧原生特效路径 011D2768/011D39D4 使用同一对象。
    pub unsafe fn sword(self) -> Sword {
        Sword(unsafe { memory::read_u32(self.0 + 0x23A0) })
    }

    pub unsafe fn effect_shell(self) -> WeaponEffectShell {
        WeaponEffectShell::from_address(unsafe { memory::read_u32(self.0 + 0x2634) })
    }

    pub unsafe fn sequence_pel(self) -> PelHandle {
        PelHandle::from_address(unsafe { memory::read_u32(self.0 + 0x3304) })
    }

    pub unsafe fn reference(self) -> u32 {
        let reference: unsafe extern "C" fn(u32) -> u32 =
            unsafe { memory::method(self.0, 0x130) };
        unsafe { reference(self.0) }
    }

    /// 0=未强化，1=黄盾，2=红盾，3=蓝盾。
    pub unsafe fn shield_state(self) -> u32 {
        unsafe { memory::read_u32(self.0 + Self::SHIELD_STATE_OFFSET) }
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

    pub unsafe fn style(self) -> u8 {
        unsafe { memory::read_u8(self.0 + Self::STYLE_OFFSET) }
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

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Sword(u32);

impl Sword {
    pub const fn address(self) -> u32 {
        self.0
    }

    /// 与主武器原生特效相同，通过虚函数 +0x130 取得可跟随的对象引用。
    pub unsafe fn reference(self) -> u32 {
        let reference: unsafe extern "C" fn(u32) -> u32 =
            unsafe { memory::method(self.0, 0x130) };
        unsafe { reference(self.0) }
    }
}
