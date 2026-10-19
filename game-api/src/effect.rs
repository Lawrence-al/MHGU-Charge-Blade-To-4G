//! 游戏特效节点与创建参数；盾牌闪电和 SAED 瓶爆共用原生参数定义。
use crate::{Game, memory};

/// cProofEffectParam 的已确认字段；未知区域原样交给游戏构造／析构。
#[repr(C)]
pub struct ProofEffectParams {
    reserved_00: u32,
    mask0: u32,
    reserved_08: u32,
    misc: u32,
    reserved_10: [u8; 0x30],
    effect_id: u16,
    bone: u16,
    reserved_44: [u8; 2],
    attach_mode: u8,
    reserved_47: [u8; 0x79],
    parent_reference: u32,
    reserved_c4: u32,
}

/// 有效参数区 0xD8 字节；16 字节对齐后的 Rust sizeof 为 0xE0，与旧缓冲区一致。
#[repr(C, align(16))]
pub struct EffectCreateParams {
    reserved_00: [u32; 2],
    priority: u32,
    reserved_0c: u32,
    proof: ProofEffectParams,
}

impl EffectCreateParams {
    pub const fn zeroed() -> Self {
        // 全部字段均为整数／字节数组，没有引用、枚举或无效零值。
        unsafe { core::mem::zeroed() }
    }

    /// 参数在调用点栈上就位后再构造，保留游戏构造与析构调用顺序。
    pub unsafe fn construct(&mut self, game: Game) {
        let function: unsafe extern "C" fn(*mut Self) = unsafe { game.function(0x00040B10) };
        unsafe { function(self) };
    }

    pub unsafe fn destruct(&mut self, game: Game) {
        let function: unsafe extern "C" fn(*mut Self) = unsafe { game.function(0x00040B50) };
        unsafe { function(self) };
    }

    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority;
    }

    /// 覆盖局部位置／旋转、骨骼和附加模式；位置／旋转沿用构造结果。
    pub fn attach_to(&mut self, reference: u32, bone: u16) {
        self.proof.mask0 |= 0x2B;
        self.proof.bone = bone;
        self.proof.attach_mode = 0;
        self.proof.misc |= 2;
        self.proof.parent_reference = reference;
    }
}

// 本类型表示 ARM32 布局，在 64 位主机检查中指针字段也必须保持 u32。
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<ProofEffectParams>() == 0xC8);
    assert!(size_of::<EffectCreateParams>() == 0xE0);
    assert!(align_of::<EffectCreateParams>() == 16);
    assert!(offset_of!(EffectCreateParams, priority) == 0x08);
    assert!(offset_of!(EffectCreateParams, proof) == 0x10);
    assert!(offset_of!(ProofEffectParams, mask0) == 0x04);
    assert!(offset_of!(ProofEffectParams, misc) == 0x0C);
    assert!(offset_of!(ProofEffectParams, bone) == 0x42);
    assert!(offset_of!(ProofEffectParams, attach_mode) == 0x46);
    assert!(offset_of!(ProofEffectParams, parent_reference) == 0xC0);
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectNode(u32);

impl EffectNode {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }
    pub const fn address(self) -> u32 {
        self.0
    }

    pub unsafe fn is_finishing(self) -> bool {
        unsafe { memory::read_u8(self.0 + 0x30) == 2 }
    }

    pub unsafe fn delta_frame(self) -> f32 {
        unsafe { memory::read_f32(self.0 + 0x1C) }
    }

    /// 第二参数 0 走 PSL 同样的收尾路径，停止生成并等待已有粒子结束。
    pub unsafe fn stop(self, game: Game) {
        let function: unsafe extern "C" fn(u32, u32) -> u32 = unsafe { game.function(0x0032A400) };
        unsafe { function(self.0, 0) };
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct WeaponEffectShell(u32);

impl WeaponEffectShell {
    pub const fn from_address(address: u32) -> Self {
        Self(address)
    }

    pub unsafe fn create_unique(
        self,
        game: Game,
        id: u16,
        params: &EffectCreateParams,
    ) -> EffectNode {
        let pel = unsafe { memory::read_u32(self.0 + 0x1388) };
        let create: unsafe extern "C" fn(
            u32,
            u32,
            u32,
            u32,
            *const EffectCreateParams,
            u32,
        ) -> u32 = unsafe { game.function(0x004A1E08) };
        // UNIQUE，sound_table=0：主动创建不另行提交伴生音效。
        EffectNode(unsafe { create(self.0, pel, 2, id as u32, params, 0) })
    }
}

/// GU 地面查询使用 16 字节向量；第四项保留为 0。
#[repr(C, align(16))]
pub struct Vector(pub [f32; 4]);
