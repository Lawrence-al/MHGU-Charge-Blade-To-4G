//! 瓶爆视觉使用爆点地表，独立于分组、贴地高度和伤害更新。
use crate::{
    Game,
    effect::{EffectNode, Vector},
    memory,
    shell::SaedShell,
};

/// 原生筛选同时保留通用位和普通地面位。
pub const LAND: u32 = 3;
/// 浅水筛选同时保留通用位；自动查询仍允许原生分类返回深水17。
pub const WATER: u32 = 5;

/// 001819B8的a6输出，与物件+0x1060的原生地表记录同布局。
/// +8地表标志、+20/+24水深比较值、+32法线均由游戏填入。
#[repr(C)]
struct GroundInfo([u32; 12]);

impl GroundInfo {
    fn new() -> Self {
        let mut words = [0; 12];
        // 沿用00042800局部记录的初值；查询写有效字段，padding保持初始化值。
        words[5] = (-100000.0f32).to_bits();
        words[9] = 1.0f32.to_bits();
        Self(words)
    }
}

/// 创建参数须已关闭自动地表（Proof+0x63），以免下帧被父对象旧记录覆盖。
/// 只设置新节点的筛选mask；不改shell坐标、瓶数、计时或攻击数据。
pub unsafe fn apply_at_shell(game: Game, shell: SaedShell, node: EffectNode) {
    let query: unsafe extern "C" fn(
        u32,
        *const Vector,
        *const Vector,
        u32,
        *mut f32,
        *mut GroundInfo,
        u32,
        u32,
        u32,
    ) -> u32 = unsafe { game.function(0x001819B8) };
    let classify: unsafe extern "C" fn(u32, *const GroundInfo) -> u32 =
        unsafe { game.function(0x00041D30) };
    let collision = unsafe { memory::load_u32(game.address(0x021270E0)) };
    let position = unsafe {
        [
            memory::load_f32(shell.address() + 0x40),
            memory::load_f32(shell.address() + 0x44),
            memory::load_f32(shell.address() + 0x48),
        ]
    };
    let room = unsafe { shell.room() };
    let flags = unsafe { shell.ground_flags() };
    let mask = unsafe { shell.ground_mask() };
    let mut info = GroundInfo::new();
    let mut found = false;
    // 与现有4G瓶爆贴地相同的两组高度范围，额外取回材质和水深记录。
    // 使用当前实际爆点，避免移动后的瓶爆继续读取出生位置的地表。
    for (upper, lower) in [(300.0, -2000.0), (2000.0, 300.0)] {
        let bottom = Vector([position[0], position[1] + lower, position[2], 0.0]);
        let top = Vector([position[0], position[1] + upper, position[2], 0.0]);
        let mut height = 0.0f32;
        if unsafe {
            query(collision, &bottom, &top, room, &mut height, &mut info, mask, 0, flags)
        } != 0 {
            found = true;
            break;
        }
    }
    if !found {
        // 与00042C44相同的回退：使用父shell原生地表记录，不猜成陆地。
        for (index, word) in info.0.iter_mut().enumerate() {
            *word = unsafe { memory::load_u32(shell.address() + 0x1060 + index as u32 * 4) };
        }
    }
    let surface_mask = unsafe { classify(node.address(), &info) };
    unsafe { node.set_surface_mask(surface_mask) };
}

const _: () = assert!(core::mem::size_of::<GroundInfo>() == 0x30);
