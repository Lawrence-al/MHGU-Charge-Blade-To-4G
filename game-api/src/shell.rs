//! uShellWp14Shell03 及其参数的访问与原生操作。
use crate::{
    Game, abi,
    effect::{EffectCreateParams, Vector},
    memory,
    player::{ChargeBladePlayer, PhialKind},
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaedShell(u32);

impl SaedShell {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }
    pub unsafe fn params(self) -> ShellParams {
        ShellParams(unsafe { memory::load_u32(self.0 + 0x136C) })
    }
    pub unsafe fn room(self) -> u32 {
        unsafe { memory::load_u8(self.0 + 0x1054) as u32 }
    }
    pub unsafe fn delta_frame(self) -> f32 {
        unsafe { memory::load_f32(self.0 + 0x1C) }
    }
    pub unsafe fn is_retired(self) -> bool {
        (unsafe { memory::load_u32(self.0 + 0x0C) } & 7) >= 3
    }
    pub unsafe fn set_action_state(self, state: u8) {
        unsafe { memory::write_u8(self.0 + 5, state) };
    }

    pub unsafe fn heading(self, game: Game) -> [f32; 2] {
        let yaw = (unsafe { memory::load_u32(self.0 + 0xFEC) } & 0xFFFF) as f32;
        let radians = (yaw * (core::f32::consts::TAU / 65536.0)).to_bits();
        unsafe {
            [
                f32::from_bits(abi::trig_call(game.address(0x013EFDE8), radians)),
                f32::from_bits(abi::trig_call(game.address(0x013EFDF4), radians)),
            ]
        }
    }

    pub unsafe fn ground_flags(self) -> u32 {
        let function: unsafe extern "C" fn(u32) -> u32 = unsafe { memory::method(self.0, 0xA8) };
        unsafe { function(self.0) }
    }
    pub unsafe fn ground_mask(self) -> u32 {
        let function: unsafe extern "C" fn(u32, u32) -> u32 =
            unsafe { memory::method(self.0, 0xAC) };
        unsafe { function(self.0, 1) }
    }

    pub unsafe fn housekeeping(self, game: Game) -> u32 {
        let function: unsafe extern "C" fn(u32) -> u32 = unsafe { game.function(0x004A2304) };
        unsafe { function(self.0) }
    }
    pub unsafe fn stop(self) -> u32 {
        let function: unsafe extern "C" fn(u32) -> u32 = unsafe { memory::method(self.0, 0x40) };
        unsafe { function(self.0) }
    }
    pub unsafe fn damage(self, game: Game) -> u32 {
        let function: unsafe extern "C" fn(u32) -> u32 = unsafe { game.function(0x00488FC0) };
        unsafe { function(self.0) }
    }

    /// 同时写入本体和镜像坐标，特效与 hitbox 使用同一个爆点。
    pub unsafe fn set_position(self, position: [f32; 3]) {
        for (index, value) in position.into_iter().enumerate() {
            unsafe {
                memory::write_u32(self.0 + 0x40 + index as u32 * 4, value.to_bits());
                memory::write_u32(self.0 + 0x1000 + index as u32 * 4, value.to_bits());
            }
        }
    }

    /// 一次贴地查询共用同一份碰撞对象和房间，与旧的两次探测取值顺序一致。
    pub unsafe fn ground_probe(self, game: Game, flags: u32, mask: u32) -> GroundProbe {
        GroundProbe {
            function: game.address(0x00181440),
            collision: unsafe { memory::load_u32(game.address(0x021270E0)) },
            room: unsafe { self.room() },
            flags,
            mask,
        }
    }

    pub unsafe fn create_blast_effect(self, game: Game, params: ShellParams) {
        if unsafe { params.suppresses_effect() } {
            return;
        }
        let effect_id: unsafe extern "C" fn(u32, u32, i32) -> u32 =
            unsafe { game.function(0x004A2F5C) };
        let id = unsafe { effect_id(self.0, 0, -1) };
        if id == 0 {
            return;
        }
        let reference: unsafe extern "C" fn(u32) -> u32 = unsafe { memory::method(self.0, 0x130) };
        let fill: unsafe extern "C" fn(u32, *mut EffectCreateParams, u32, u32, u32, u32, i32) =
            unsafe { game.function(0x004A1D34) };
        let create: unsafe extern "C" fn(u32, u32, *const EffectCreateParams) =
            unsafe { game.function(0x004A1E50) };
        let shake: unsafe extern "C" fn(u32, u32, u32) = unsafe { game.function(0x002A063C) };
        let parent: unsafe extern "C" fn(u32) -> u32 = unsafe { game.function(0x004A1B6C) };
        let mut effect = EffectCreateParams::zeroed();
        unsafe {
            effect.construct(game);
            fill(
                self.0,
                &mut effect,
                reference(self.0),
                0,
                self.room(),
                self.0 + 0x40,
                -1,
            );
            create(self.0, id, &effect);
            shake(parent(self.0), 20, 1024);
            effect.destruct(game);
        }
    }
}

pub struct GroundProbe {
    function: u32,
    collision: u32,
    room: u32,
    flags: u32,
    mask: u32,
}

impl GroundProbe {
    /// 返回命中高度；成功字节仍由游戏写入，使用 VFP 返回值转换器。
    pub unsafe fn height(&self, bottom: &Vector, top: &Vector, hit: &mut u8) -> f32 {
        let arguments = [
            self.collision,
            bottom as *const Vector as u32,
            top as *const Vector as u32,
            self.room,
            hit as *mut u8 as u32,
            self.flags,
            self.mask,
            0,
        ];
        f32::from_bits(unsafe { abi::float_call8(self.function, &arguments) })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ShellParams(u32);

impl ShellParams {
    /// 资源子类型 0..3 为榴弹瓶，4..27 为属性瓶，区别于装备参数 27。
    pub unsafe fn kind(self) -> PhialKind {
        if unsafe { memory::load_u8(self.0 + 8) } < 4 {
            PhialKind::Impact
        } else {
            PhialKind::Element
        }
    }
    pub unsafe fn parent(self) -> ChargeBladePlayer {
        ChargeBladePlayer::from_address(unsafe { memory::load_u32(self.0 + 0x0C) })
    }
    pub unsafe fn phial_count(self) -> u8 {
        unsafe { memory::load_u8(self.0 + 0x11) }
    }
    pub unsafe fn set_phial_count(self, count: u8) {
        unsafe { memory::write_u8(self.0 + 0x11, count) };
    }
    pub unsafe fn suppresses_effect(self) -> bool {
        unsafe { memory::load_u8(self.0 + 0x10) != 0 }
    }
}
