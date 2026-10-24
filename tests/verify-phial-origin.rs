// 执行实际 phial.rs：模拟人物在首爆前后移动，检查首爆取位及后续爆点固定。
// 游戏地面／瓶伤调用使用替身；不模拟粒子、碰撞材质和实际伤害。
extern crate self as forge;
extern crate self as cb_game;
#[derive(Clone, Copy)] pub struct Game(u32);
impl Game {
    pub const fn new(base:u32)->Self {Self(base)}
    pub const fn address(self,offset:u32)->u32 {self.0.wrapping_add(offset)}
    pub(crate) unsafe fn function<T:Copy>(self,offset:u32)->T {unsafe{native::function(self.address(offset))}}
}
#[path="../game-api/src/player.rs"] pub mod player;
#[path="../game-api/src/command.rs"] pub mod command;
#[path="../game-api/src/sound.rs"] pub mod sound;
#[path="../game-api/src/effect.rs"] pub mod effect;
#[path="../game-api/src/resource.rs"] pub mod resource;
#[path="../game-api/src/shell.rs"] pub mod shell;
mod memory {
    pub use super::native::{read_u8,read_u32,read_f32,write_u8,write_u32};
    pub use read_u8 as load_u8;
    pub use read_u32 as load_u32;
    pub use read_f32 as load_f32;
    pub unsafe fn read_u16(address:u32)->u16 {u16::from_le_bytes(unsafe{[read_u8(address),read_u8(address+1)]})}
    pub(crate) unsafe fn method<T:Copy>(object:u32,offset:u32)->T {unsafe{super::native::method(object,offset)}}
}
mod abi {
    pub unsafe fn trig_call(offset:u32,angle:u32)->u32 {unsafe{super::native::trig_call(offset,angle)}}
    pub unsafe fn frame_event(_:u32,_:u32,_:u32)->u32 {panic!("origin检查不调用动作帧入口")}
    pub unsafe fn float_call8(offset:u32,args:*const[u32;8])->u32 {unsafe{super::native::float_call8(offset,&*args)}}
}


pub mod os {
    pub mod sync {
        pub use std::sync::LazyLock;
        pub struct Mutex(std::sync::Mutex<()>);
        impl Mutex {
            pub fn new() -> Self {
                Self(std::sync::Mutex::new(()))
            }
        }
        pub struct ScopedLock<'a>(#[allow(dead_code)] std::sync::MutexGuard<'a, ()>);
        impl<'a> ScopedLock<'a> {
            pub fn new(mutex: &'a Mutex) -> Self {
                Self(mutex.0.lock().unwrap())
            }
        }
    }
}

#[path = "../Moveset-to-4G/src/saed/phial.rs"]
mod phial;
#[path = "../Moveset-to-4G/src/saed/schedule.rs"]
pub mod schedule;
mod native {
    use std::{
        collections::BTreeMap,
        sync::{LazyLock, Mutex},
    };
    static MEMORY: LazyLock<Mutex<BTreeMap<u32, u8>>> =
        LazyLock::new(|| Mutex::new(BTreeMap::new()));
    pub static SHOTS: LazyLock<Mutex<Vec<([f32; 3], u8)>>> =
        LazyLock::new(|| Mutex::new(Vec::new()));
    pub fn address(offset: u32) -> u32 {
        offset
    }
    pub unsafe fn read_u8(address: u32) -> u8 {
        MEMORY.lock().unwrap()[&address]
    }
    pub unsafe fn read_u32(address: u32) -> u32 {
        let memory = MEMORY.lock().unwrap();
        u32::from_le_bytes(core::array::from_fn(|i| memory[&(address + i as u32)]))
    }
    pub unsafe fn read_f32(address: u32) -> f32 {
        f32::from_bits(unsafe { read_u32(address) })
    }
    pub unsafe fn write_u8(address: u32, value: u8) {
        MEMORY.lock().unwrap().insert(address, value);
    }
    pub unsafe fn write_u32(address: u32, value: u32) {
        let mut memory = MEMORY.lock().unwrap();
        for (i, byte) in value.to_le_bytes().into_iter().enumerate() {
            memory.insert(address + i as u32, byte);
        }
    }
    pub unsafe fn trig_call(offset: u32, angle: u32) -> u32 {
        let angle = f32::from_bits(angle);
        match offset {
            0x013EFDE8 => angle.sin(),
            0x013EFDF4 => angle.cos(),
            _ => panic!("未知三角调用"),
        }
        .to_bits()
    }
    pub unsafe fn float_call8(offset: u32, _: &[u32; 8]) -> u32 {
        assert_eq!(offset, 0x00181440);
        // 不命中地面，使用首爆人物 Y；不解引用截为 u32 的主机栈指针。
        0
    }
    unsafe extern "C" fn housekeeping(_: u32) -> u32 {
        7
    }
    unsafe extern "C" fn damage(shell: u32) -> u32 {
        let position = unsafe {
            [
                read_f32(shell + 0x40),
                read_f32(shell + 0x44),
                read_f32(shell + 0x48),
            ]
        };
        let params = unsafe { read_u32(shell + 0x136C) };
        let remaining = unsafe { read_u8(params + 0x11) };
        SHOTS.lock().unwrap().push((position, remaining));
        8
    }
    unsafe extern "C" fn flags(_: u32) -> u32 {
        0
    }
    unsafe extern "C" fn mask(_: u32, _: u32) -> u32 {
        0
    }
    unsafe extern "C" fn stop(shell: u32) -> u32 {
        unsafe { write_u32(shell + 0x0C, 3) };
        9
    }
    pub unsafe fn function<T: Copy>(offset: u32) -> T {
        let pointer = match offset {
            0x004A2304 => housekeeping as *const (),
            0x00488FC0 => damage as *const (),
            _ => panic!("未知函数 {offset:X}"),
        };
        unsafe { core::mem::transmute_copy(&pointer) }
    }
    pub unsafe fn method<T: Copy>(_: u32, offset: u32) -> T {
        let pointer = match offset {
            0xA8 => flags as *const (),
            0xAC => mask as *const (),
            0x40 => stop as *const (),
            _ => panic!("未知虚函数 {offset:X}"),
        };
        unsafe { core::mem::transmute_copy(&pointer) }
    }
}

fn set_position(actor: u32, position: [f32; 3]) {
    for (i, value) in position.into_iter().enumerate() {
        unsafe { native::write_u32(actor + 0x40 + i as u32 * 4, value.to_bits()) };
    }
}

fn main() {
    use schedule::PhialKind;
    phial::init();
    for (kind, subtype, first_ticks) in [(PhialKind::Impact, 2, 1), (PhialKind::Element, 6, 46)] {
        let shell = 0x20000000;
        let params = 0x21000000;
        let player = 0x22000000;
        unsafe {
            native::write_u32(shell + 0x136C, params);
            native::write_u32(params + 0x0C, player);
            native::write_u8(params + 8, subtype);
            native::write_u8(params + 0x10, 1); // 本检查关闭粒子，不改生产代码。
            native::write_u8(params + 0x11, 10);
            native::write_u32(shell + 0xFEC, 0);
            native::write_u32(shell + 0x0C, 1);
            native::write_u8(shell + 0x1054, 0);
            native::write_u32(shell + 0x1C, 1.0f32.to_bits());
            native::write_u32(0x021270E0, 0);
        }
        set_position(player, [10.0, 20.0, 30.0]);
        native::SHOTS.lock().unwrap().clear();
        unsafe { phial::after_init(shell::SaedShell::from_address(shell), Game::new(0)) };
        for _ in 1..first_ticks {
            unsafe { phial::on_frame(shell::SaedShell::from_address(shell)) };
            assert!(native::SHOTS.lock().unwrap().is_empty());
        }
        let first_origin = [100.0, 40.0, 1000.0];
        set_position(player, first_origin);
        unsafe { phial::on_frame(shell::SaedShell::from_address(shell)) };
        assert_eq!(native::SHOTS.lock().unwrap().len(), 1);
        // 首爆后人物继续移动，不应把后续爆点拖到新人物坐标。
        set_position(player, [10000.0, 900.0, 20000.0]);
        for _ in 0..200 {
            unsafe { phial::on_frame(shell::SaedShell::from_address(shell)) };
        }
        let shots = native::SHOTS.lock().unwrap();
        assert_eq!(shots.len(), 10);
        for (i, &(actual, remaining)) in shots.iter().enumerate() {
            let blast = schedule::blast(kind, i as u8);
            assert_eq!(
                actual,
                [
                    first_origin[0] + blast.side,
                    first_origin[1],
                    first_origin[2] + blast.forward
                ]
            );
            assert_eq!(remaining, 9 - i as u8);
        }
        let last = shots.last().unwrap().0;
        for (i, value) in last.into_iter().enumerate() {
            assert_eq!(
                unsafe { native::read_f32(shell + 0x1000 + i as u32 * 4) },
                value
            );
        }
    }
    println!(
        "PASS: actual phial.rs captures the live player position on first blast, anchors all 20 blast positions despite player movement, preserves remaining counts and mirrors"
    );
}
